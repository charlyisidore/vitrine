use std::{io::Read, sync::Arc};

use vitrine_deno::{
  CliFactory,
  deno_cache_dir::file_fetcher::File,
  deno_runtime::{
    WorkerExecutionMode,
    deno_core::{
      JsRuntime, anyhow, anyhow::anyhow, resolve_url_or_path, serde_json,
      serde_v8, v8,
    },
  },
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
  let mut args = std::env::args();

  let (path, source) = match (args.next(), args.next(), args.next()) {
    (_, Some(path), None) => (Some(path), None),
    (_, None, None) => {
      let mut buf = String::new();
      std::io::stdin().read_to_string(&mut buf)?;
      (None, Some(buf))
    }
    (bin, _, _) => {
      println!("Usage: {} [FILE]", bin.unwrap_or_else(|| "run".to_string()));
      return Ok(());
    }
  };

  JsRuntime::init_platform(None, false);

  let main_module = resolve_url_or_path(
    &path.unwrap_or_else(|| "./$deno$stdin.mts".into()),
    &std::env::current_dir()?,
  )?;

  let cli_factory = CliFactory::from_flags(Arc::new(Default::default()));
  let worker_factory = cli_factory.create_cli_main_worker_factory().await?;

  if let Some(source) = source {
    let file_fetcher = cli_factory.file_fetcher()?;
    file_fetcher.insert_memory_files(File {
      url: main_module.clone(),
      maybe_headers: None,
      source: source.as_bytes().to_vec().into(),
      mtime: None,
    });
  }

  let mut worker = worker_factory
    .create_main_worker(
      WorkerExecutionMode::Run,
      main_module.clone(),
      Default::default(),
    )
    .await?
    .into_main_worker();

  let module_id = worker.preload_main_module(&main_module).await?;
  worker.evaluate_module(module_id).await?;

  let runtime = &mut worker.js_runtime;
  let namespace = runtime.get_module_namespace(module_id)?;

  let scope = &mut runtime.handle_scope();
  let namespace = v8::Local::new(scope, namespace);
  let default_key = v8::String::new(scope, "default").unwrap();
  let default_value = namespace.get(scope, default_key.into()).unwrap();

  let function = default_value.try_cast::<v8::Function>()?;

  let scope = &mut v8::TryCatch::new(scope);
  let this = v8::undefined(scope);
  let result = function.call(scope, this.into(), &[]).ok_or_else(|| {
    if scope.has_caught() {
      anyhow!(scope.exception().unwrap().to_rust_string_lossy(scope))
    } else {
      anyhow!("unknown error")
    }
  })?;

  let result = serde_v8::from_v8::<serde_json::Value>(scope, result)?;
  let result = serde_json::to_string_pretty(&result)?;

  println!("{}", result);

  Ok(())
}
