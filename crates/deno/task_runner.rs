use deno_core::anyhow::Context;
use deno_core::error::AnyError;
use deno_core::futures::future::LocalBoxFuture;
use deno_task_shell::ExecutableCommand;
use deno_task_shell::ExecuteResult;
use deno_task_shell::KillSignal;
use deno_task_shell::ShellCommand;
use deno_task_shell::ShellCommandContext;
use deno_task_shell::ShellPipeReader;
use deno_task_shell::ShellPipeWriter;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::task::JoinHandle;
use tokio::task::LocalSet;
pub fn get_script_with_args(script: &str, argv: &[String]) -> String {
  let additional_args = argv
    .iter()
    .map(|a| format!("\"{}\"", a.replace('"', "\\\"").replace('$', "\\$")))
    .collect::<Vec<_>>()
    .join(" ");
  let script = format!("{script} {additional_args}");
  script.trim().to_owned()
}
pub struct TaskStdio(Option<ShellPipeReader>, ShellPipeWriter);
impl TaskStdio {
  pub fn stdout() -> Self {
    Self(None, ShellPipeWriter::stdout())
  }
  pub fn stderr() -> Self {
    Self(None, ShellPipeWriter::stderr())
  }
  pub fn piped() -> Self {
    let (r, w) = deno_task_shell::pipe();
    Self(Some(r), w)
  }
}
pub struct TaskIo {
  pub stdout: TaskStdio,
  pub stderr: TaskStdio,
}
impl Default for TaskIo {
  fn default() -> Self {
    Self {
      stdout: TaskStdio::stdout(),
      stderr: TaskStdio::stderr(),
    }
  }
}
pub struct RunTaskOptions<'a> {
  pub task_name: &'a str,
  pub script: &'a str,
  pub cwd: PathBuf,
  pub init_cwd: &'a Path,
  pub env_vars: HashMap<OsString, OsString>,
  pub argv: &'a [String],
  pub custom_commands: HashMap<String, Rc<dyn ShellCommand>>,
  pub root_node_modules_dir: Option<&'a Path>,
  pub stdio: Option<TaskIo>,
  pub kill_signal: KillSignal,
}
pub type TaskCustomCommands = HashMap<String, Rc<dyn ShellCommand>>;
pub struct TaskResult {
  pub exit_code: i32,
  pub stdout: Option<Vec<u8>>,
  pub stderr: Option<Vec<u8>>,
}
pub async fn run_task(
  mut opts: RunTaskOptions<'_>,
) -> Result<TaskResult, AnyError> {
  let script = get_script_with_args(opts.script, opts.argv);
  let seq_list = deno_task_shell::parser::parse(&script)
    .with_context(|| format!("Error parsing script '{}'.", opts.task_name))?;
  let env_vars =
    prepare_env_vars(opts.env_vars, opts.init_cwd, opts.root_node_modules_dir);
  if !opts.custom_commands.contains_key("deno") {
    opts
      .custom_commands
      .insert("deno".to_string(), Rc::new(DenoCommand::default()));
  }
  let state = deno_task_shell::ShellState::new(
    env_vars,
    opts.cwd,
    opts.custom_commands,
    opts.kill_signal,
  );
  let stdio = opts.stdio.unwrap_or_default();
  let (
    TaskStdio(stdout_read, stdout_write),
    TaskStdio(stderr_read, stderr_write),
  ) = (stdio.stdout, stdio.stderr);
  fn read(reader: ShellPipeReader) -> JoinHandle<Result<Vec<u8>, AnyError>> {
    tokio::task::spawn_blocking(move || {
      let mut buf = Vec::new();
      reader.pipe_to(&mut buf)?;
      Ok(buf)
    })
  }
  let stdout = stdout_read.map(read);
  let stderr = stderr_read.map(read);
  let local = LocalSet::new();
  let future = async move {
    let exit_code = deno_task_shell::execute_with_pipes(
      seq_list,
      state,
      ShellPipeReader::stdin(),
      stdout_write,
      stderr_write,
    )
    .await;
    Ok::<_, AnyError>(TaskResult {
      exit_code,
      stdout: if let Some(stdout) = stdout {
        Some(stdout.await??)
      } else {
        None
      },
      stderr: if let Some(stderr) = stderr {
        Some(stderr.await??)
      } else {
        None
      },
    })
  };
  local.run_until(future).await
}
fn prepare_env_vars(
  mut env_vars: HashMap<OsString, OsString>,
  initial_cwd: &Path,
  node_modules_dir: Option<&Path>,
) -> HashMap<OsString, OsString> {
  const INIT_CWD_NAME: &str = "INIT_CWD";
  if !env_vars.contains_key(OsStr::new(INIT_CWD_NAME)) {
    env_vars.insert(
      INIT_CWD_NAME.into(),
      initial_cwd.to_path_buf().into_os_string(),
    );
  }
  if !env_vars
    .contains_key(OsStr::new(crate::npm::NPM_CONFIG_USER_AGENT_ENV_VAR))
  {
    env_vars.insert(
      crate::npm::NPM_CONFIG_USER_AGENT_ENV_VAR.into(),
      crate::npm::get_npm_config_user_agent().into(),
    );
  }
  if let Some(node_modules_dir) = node_modules_dir {
    prepend_to_path(
      &mut env_vars,
      node_modules_dir.join(".bin").into_os_string(),
    );
  }
  env_vars
}
fn prepend_to_path(
  env_vars: &mut HashMap<OsString, OsString>,
  value: OsString,
) {
  match env_vars.get_mut(OsStr::new("PATH")) {
    Some(path) => {
      if path.is_empty() {
        *path = value;
      } else {
        let mut new_path = value;
        new_path.push(if cfg!(windows) { ";" } else { ":" });
        new_path.push(&path);
        *path = new_path;
      }
    }
    None => {
      env_vars.insert("PATH".into(), value);
    }
  }
}
pub fn real_env_vars() -> HashMap<OsString, OsString> {
  std::env::vars_os()
    .map(|(k, v)| {
      if cfg!(windows) {
        (k.to_ascii_uppercase(), v)
      } else {
        (k, v)
      }
    })
    .collect()
}
pub(crate) static USE_PKG_JSON_HIDDEN_ENV_VAR_NAME: &str =
  "DENO_INTERNAL_TASK_USE_PKG_JSON";
pub struct NpmCommand;
impl ShellCommand for NpmCommand {
  fn execute(
    &self,
    mut context: ShellCommandContext,
  ) -> LocalBoxFuture<'static, ExecuteResult> {
    if context.args.first().and_then(|s| s.to_str()) == Some("run")
      && context.args.len() >= 2
      && !context
        .args
        .iter()
        .any(|s| s.to_string_lossy().starts_with('-'))
    {
      let mut args: Vec<OsString> = Vec::with_capacity(context.args.len());
      args.push("task".into());
      args.extend(context.args.into_iter().skip(1));
      let mut state = context.state;
      state.apply_env_var(
        OsStr::new(USE_PKG_JSON_HIDDEN_ENV_VAR_NAME),
        OsStr::new("1"),
      );
      return ExecutableCommand::new(
        "deno".to_string(),
        std::env::current_exe().unwrap(),
      )
      .execute(ShellCommandContext {
        args,
        state,
        ..context
      });
    }
    let npm_path = match context.state.resolve_command_path(OsStr::new("npm")) {
      Ok(path) => path,
      Err(err) => {
        let _ = context.stderr.write_line(&format!("{}", err));
        return Box::pin(std::future::ready(ExecuteResult::from_exit_code(
          err.exit_code(),
        )));
      }
    };
    ExecutableCommand::new("npm".to_string(), npm_path).execute(context)
  }
}
pub struct DenoCommand(ExecutableCommand);
impl Default for DenoCommand {
  fn default() -> Self {
    Self(ExecutableCommand::new(
      "deno".to_string(),
      std::env::current_exe().unwrap(),
    ))
  }
}
impl ShellCommand for DenoCommand {
  fn execute(
    &self,
    context: ShellCommandContext,
  ) -> LocalBoxFuture<'static, ExecuteResult> {
    self.0.execute(context)
  }
}
pub struct NodeCommand;
impl ShellCommand for NodeCommand {
  fn execute(
    &self,
    context: ShellCommandContext,
  ) -> LocalBoxFuture<'static, ExecuteResult> {
    if context.args.is_empty()
      || ({
        let first_arg = context.args[0].to_string_lossy();
        first_arg.starts_with('-')
      })
    {
      return ExecutableCommand::new("node".to_string(), PathBuf::from("node"))
        .execute(context);
    }
    let mut args: Vec<OsString> = Vec::with_capacity(7 + context.args.len());
    args.extend([
      "run".into(),
      "-A".into(),
      "--unstable-bare-node-builtins".into(),
      "--unstable-detect-cjs".into(),
      "--unstable-sloppy-imports".into(),
      "--unstable-unsafe-proto".into(),
    ]);
    args.extend(context.args);
    let mut state = context.state;
    state.apply_env_var(
      OsStr::new(USE_PKG_JSON_HIDDEN_ENV_VAR_NAME),
      OsStr::new("1"),
    );
    ExecutableCommand::new("deno".to_string(), std::env::current_exe().unwrap())
      .execute(ShellCommandContext {
        args,
        state,
        ..context
      })
  }
}
pub struct NodeGypCommand;
impl ShellCommand for NodeGypCommand {
  fn execute(
    &self,
    context: ShellCommandContext,
  ) -> LocalBoxFuture<'static, ExecuteResult> {
    if context
      .state
      .resolve_command_path(OsStr::new("node-gyp"))
      .is_err()
    {
      log::warn!(
        "{} node-gyp was used in a script, but was not listed as a dependency. Either add it as a dependency or install it globally (e.g. `npm install -g node-gyp`)",
        crate::colors::yellow("Warning")
      );
    }
    ExecutableCommand::new(
      "node-gyp".to_string(),
      "node-gyp".to_string().into(),
    )
    .execute(context)
  }
}
pub struct NpxCommand;
impl ShellCommand for NpxCommand {
  fn execute(
    &self,
    mut context: ShellCommandContext,
  ) -> LocalBoxFuture<'static, ExecuteResult> {
    if let Some(first_arg) = context.args.first().cloned() {
      match context.state.resolve_custom_command(&first_arg) {
        Some(command) => {
          let context = ShellCommandContext {
            args: context.args.into_iter().skip(1).collect::<Vec<_>>(),
            ..context
          };
          command.execute(context)
        }
        _ => {
          let npx_path =
            match context.state.resolve_command_path(OsStr::new("npx")) {
              Ok(npx) => npx,
              Err(err) => {
                let _ = context.stderr.write_line(&format!("{}", err));
                return Box::pin(std::future::ready(
                  ExecuteResult::from_exit_code(err.exit_code()),
                ));
              }
            };
          ExecutableCommand::new("npx".to_string(), npx_path).execute(context)
        }
      }
    } else {
      let _ = context.stderr.write_line("npx: missing command");
      Box::pin(std::future::ready(ExecuteResult::from_exit_code(1)))
    }
  }
}
/// Runs a module in the node_modules folder.
#[derive(Clone)]
pub struct NodeModulesFileRunCommand {
  pub command_name: String,
  pub path: PathBuf,
}
impl ShellCommand for NodeModulesFileRunCommand {
  fn execute(
    &self,
    mut context: ShellCommandContext,
  ) -> LocalBoxFuture<'static, ExecuteResult> {
    let mut args: Vec<OsString> = vec![
      "run".into(),
      "--ext=js".into(),
      "-A".into(),
      self.path.clone().into_os_string(),
    ];
    args.extend(context.args);
    let executable_command = deno_task_shell::ExecutableCommand::new(
      "deno".to_string(),
      std::env::current_exe().unwrap(),
    );
    context.state.apply_env_var(
      OsStr::new("DENO_INTERNAL_NPM_CMD_NAME"),
      OsStr::new(&self.command_name),
    );
    executable_command.execute(ShellCommandContext { args, ..context })
  }
}
