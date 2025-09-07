use crate::args::CliLockfile;
use crate::npm::CliNpmInstaller;
use crate::npm::CliNpmResolver;
use crate::sys::CliSys;
use crate::tools::coverage::CoverageCollector;
use crate::tools::run::hmr::HmrRunner;
use crate::util::file_watcher::WatcherCommunicator;
use crate::util::file_watcher::WatcherRestartMode;
use deno_ast::ModuleSpecifier;
use deno_core::Extension;
use deno_core::OpState;
use deno_core::PollEventLoopOptions;
use deno_core::error::CoreError;
use deno_core::error::JsError;
use deno_core::futures::FutureExt;
use deno_core::v8;
use deno_error::JsErrorBox;
use deno_lib::worker::LibMainWorker;
use deno_lib::worker::LibMainWorkerFactory;
use deno_lib::worker::ResolveNpmBinaryEntrypointError;
use deno_npm_installer::PackageCaching;
use deno_npm_installer::graph::NpmCachingStrategy;
use deno_runtime::WorkerExecutionMode;
use deno_runtime::deno_permissions::PermissionsContainer;
use deno_runtime::worker::MainWorker;
use deno_semver::npm::NpmPackageReqReference;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use sys_traits::EnvCurrentDir;
use tokio::select;
pub type CreateHmrRunnerCb =
  Box<dyn Fn(deno_core::LocalInspectorSession) -> HmrRunner + Send + Sync>;
pub type CreateCoverageCollectorCb = Box<
  dyn Fn(deno_core::LocalInspectorSession) -> CoverageCollector + Send + Sync,
>;
pub struct CliMainWorkerOptions {
  pub create_hmr_runner: Option<CreateHmrRunnerCb>,
  pub create_coverage_collector: Option<CreateCoverageCollectorCb>,
  pub default_npm_caching_strategy: NpmCachingStrategy,
  pub needs_test_modules: bool,
}
/// Data shared between the factory and workers.
struct SharedState {
  pub create_hmr_runner: Option<CreateHmrRunnerCb>,
  pub create_coverage_collector: Option<CreateCoverageCollectorCb>,
  pub maybe_file_watcher_communicator: Option<Arc<WatcherCommunicator>>,
}
pub struct CliMainWorker {
  worker: LibMainWorker,
  shared: Arc<SharedState>,
}
impl CliMainWorker {
  #[inline]
  pub fn into_main_worker(self) -> MainWorker {
    self.worker.into_main_worker()
  }
  pub async fn setup_repl(&mut self) -> Result<(), CoreError> {
    self.worker.run_event_loop(false).await?;
    Ok(())
  }
  pub async fn run(&mut self) -> Result<i32, CoreError> {
    let mut maybe_coverage_collector =
      self.maybe_setup_coverage_collector().await?;
    let mut maybe_hmr_runner = self.maybe_setup_hmr_runner().await?;
    log::debug!("main_module {}", self.worker.main_module());
    self.worker.execute_preload_modules().await?;
    self.execute_main_module().await?;
    self.worker.dispatch_load_event()?;
    loop {
      if let Some(hmr_runner) = maybe_hmr_runner.as_mut() {
        let hmr_future = hmr_runner.run().boxed_local();
        let event_loop_future = self.worker.run_event_loop(false).boxed_local();
        let result;
        select! {
            hmr_result = hmr_future => { result = hmr_result; },
            event_loop_result = event_loop_future => { result =
            event_loop_result; }
        }
        if let Err(e) = result {
          self
            .shared
            .maybe_file_watcher_communicator
            .as_ref()
            .unwrap()
            .change_restart_mode(WatcherRestartMode::Automatic);
          return Err(e);
        }
      } else {
        self
          .worker
          .run_event_loop(maybe_coverage_collector.is_none())
          .await?;
      }
      let web_continue = self.worker.dispatch_beforeunload_event()?;
      if !web_continue {
        let node_continue = self.worker.dispatch_process_beforeexit_event()?;
        if !node_continue {
          break;
        }
      }
    }
    self.worker.dispatch_unload_event()?;
    self.worker.dispatch_process_exit_event()?;
    if let Some(coverage_collector) = maybe_coverage_collector.as_mut() {
      self
        .worker
        .js_runtime()
        .with_event_loop_future(
          coverage_collector.stop_collecting().boxed_local(),
          PollEventLoopOptions::default(),
        )
        .await?;
    }
    if let Some(hmr_runner) = maybe_hmr_runner.as_mut() {
      self
        .worker
        .js_runtime()
        .with_event_loop_future(
          hmr_runner.stop().boxed_local(),
          PollEventLoopOptions::default(),
        )
        .await?;
    }
    Ok(self.worker.exit_code())
  }
  pub async fn run_for_watcher(self) -> Result<(), CoreError> {
    /// The FileWatcherModuleExecutor provides module execution with safe dispatching of life-cycle events by tracking the
    /// state of any pending events and emitting accordingly on drop in the case of a future
    /// cancellation.
    struct FileWatcherModuleExecutor {
      inner: CliMainWorker,
      pending_unload: bool,
    }
    impl FileWatcherModuleExecutor {
      pub fn new(worker: CliMainWorker) -> FileWatcherModuleExecutor {
        FileWatcherModuleExecutor {
          inner: worker,
          pending_unload: false,
        }
      }
      /// Execute the given main module emitting load and unload events before and after execution
      /// respectively.
      pub async fn execute(&mut self) -> Result<(), CoreError> {
        self.inner.execute_main_module().await?;
        self.inner.worker.dispatch_load_event()?;
        self.pending_unload = true;
        let result = loop {
          match self.inner.worker.run_event_loop(false).await {
            Ok(()) => {}
            Err(error) => break Err(error),
          }
          let web_continue = self.inner.worker.dispatch_beforeunload_event()?;
          if !web_continue {
            let node_continue =
              self.inner.worker.dispatch_process_beforeexit_event()?;
            if !node_continue {
              break Ok(());
            }
          }
        };
        self.pending_unload = false;
        result?;
        self.inner.worker.dispatch_unload_event()?;
        self.inner.worker.dispatch_process_exit_event()?;
        Ok(())
      }
    }
    impl Drop for FileWatcherModuleExecutor {
      fn drop(&mut self) {
        if self.pending_unload {
          let _ = self.inner.worker.dispatch_unload_event();
        }
      }
    }
    let mut executor = FileWatcherModuleExecutor::new(self);
    executor.execute().await
  }
  #[inline]
  pub async fn execute_main_module(&mut self) -> Result<(), CoreError> {
    self.worker.execute_main_module().await
  }
  #[inline]
  pub async fn execute_side_module(&mut self) -> Result<(), CoreError> {
    self.worker.execute_side_module().await
  }
  #[inline]
  pub async fn execute_preload_modules(&mut self) -> Result<(), CoreError> {
    self.worker.execute_preload_modules().await
  }
  pub fn op_state(&mut self) -> Rc<RefCell<OpState>> {
    self.worker.js_runtime().op_state()
  }
  pub async fn maybe_setup_hmr_runner(
    &mut self,
  ) -> Result<Option<HmrRunner>, CoreError> {
    let Some(setup_hmr_runner) = self.shared.create_hmr_runner.as_ref() else {
      return Ok(None);
    };
    let session = self.worker.create_inspector_session();
    let mut hmr_runner = setup_hmr_runner(session);
    self
      .worker
      .js_runtime()
      .with_event_loop_future(
        hmr_runner.start().boxed_local(),
        PollEventLoopOptions::default(),
      )
      .await?;
    Ok(Some(hmr_runner))
  }
  pub async fn maybe_setup_coverage_collector(
    &mut self,
  ) -> Result<Option<CoverageCollector>, CoreError> {
    let Some(create_coverage_collector) =
      self.shared.create_coverage_collector.as_ref()
    else {
      return Ok(None);
    };
    let session = self.worker.create_inspector_session();
    let mut coverage_collector = create_coverage_collector(session);
    self
      .worker
      .js_runtime()
      .with_event_loop_future(
        coverage_collector.start_collecting().boxed_local(),
        PollEventLoopOptions::default(),
      )
      .await?;
    Ok(Some(coverage_collector))
  }
  #[allow(clippy::result_large_err)]
  pub fn execute_script_static(
    &mut self,
    name: &'static str,
    source_code: &'static str,
  ) -> Result<v8::Global<v8::Value>, JsError> {
    self.worker.js_runtime().execute_script(name, source_code)
  }
}
#[derive(Debug, thiserror::Error, deno_error::JsError)]
pub enum CreateCustomWorkerError {
  #[class(inherit)]
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[class(inherit)]
  #[error(transparent)]
  Core(#[from] CoreError),
  #[class(inherit)]
  #[error(transparent)]
  ResolvePkgFolderFromDenoReq(
    #[from] deno_resolver::npm::ResolvePkgFolderFromDenoReqError,
  ),
  #[class(inherit)]
  #[error(transparent)]
  UrlParse(#[from] deno_core::url::ParseError),
  #[class(inherit)]
  #[error(transparent)]
  ResolveNpmBinaryEntrypoint(#[from] ResolveNpmBinaryEntrypointError),
  #[class(inherit)]
  #[error(transparent)]
  NpmPackageReq(JsErrorBox),
  #[class(inherit)]
  #[error(transparent)]
  LockfileWrite(#[from] deno_resolver::lockfile::LockfileWriteError),
}
pub struct CliMainWorkerFactory {
  lib_main_worker_factory: LibMainWorkerFactory<CliSys>,
  maybe_lockfile: Option<Arc<CliLockfile>>,
  npm_installer: Option<Arc<CliNpmInstaller>>,
  npm_resolver: CliNpmResolver,
  root_permissions: PermissionsContainer,
  shared: Arc<SharedState>,
  sys: CliSys,
  default_npm_caching_strategy: NpmCachingStrategy,
  needs_test_modules: bool,
}
impl CliMainWorkerFactory {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    lib_main_worker_factory: LibMainWorkerFactory<CliSys>,
    maybe_file_watcher_communicator: Option<Arc<WatcherCommunicator>>,
    maybe_lockfile: Option<Arc<CliLockfile>>,
    npm_installer: Option<Arc<CliNpmInstaller>>,
    npm_resolver: CliNpmResolver,
    sys: CliSys,
    options: CliMainWorkerOptions,
    root_permissions: PermissionsContainer,
  ) -> Self {
    Self {
      lib_main_worker_factory,
      maybe_lockfile,
      npm_installer,
      npm_resolver,
      root_permissions,
      sys,
      shared: Arc::new(SharedState {
        create_hmr_runner: options.create_hmr_runner,
        create_coverage_collector: options.create_coverage_collector,
        maybe_file_watcher_communicator,
      }),
      default_npm_caching_strategy: options.default_npm_caching_strategy,
      needs_test_modules: options.needs_test_modules,
    }
  }
  pub async fn create_main_worker(
    &self,
    mode: WorkerExecutionMode,
    main_module: ModuleSpecifier,
    preload_modules: Vec<ModuleSpecifier>,
  ) -> Result<CliMainWorker, CreateCustomWorkerError> {
    self
      .create_custom_worker(
        mode,
        main_module,
        preload_modules,
        self.root_permissions.clone(),
        vec![],
        Default::default(),
        None,
      )
      .await
  }
  pub async fn create_main_worker_with_unconfigured_runtime(
    &self,
    mode: WorkerExecutionMode,
    main_module: ModuleSpecifier,
    preload_modules: Vec<ModuleSpecifier>,
    unconfigured_runtime: Option<deno_runtime::UnconfiguredRuntime>,
  ) -> Result<CliMainWorker, CreateCustomWorkerError> {
    self
      .create_custom_worker(
        mode,
        main_module,
        preload_modules,
        self.root_permissions.clone(),
        vec![],
        Default::default(),
        unconfigured_runtime,
      )
      .await
  }
  #[allow(clippy::too_many_arguments)]
  pub async fn create_custom_worker(
    &self,
    mode: WorkerExecutionMode,
    main_module: ModuleSpecifier,
    preload_modules: Vec<ModuleSpecifier>,
    permissions: PermissionsContainer,
    custom_extensions: Vec<Extension>,
    stdio: deno_runtime::deno_io::Stdio,
    unconfigured_runtime: Option<deno_runtime::UnconfiguredRuntime>,
  ) -> Result<CliMainWorker, CreateCustomWorkerError> {
    let main_module = match NpmPackageReqReference::from_specifier(&main_module)
    {
      Ok(package_ref) => {
        if let Some(npm_installer) = &self.npm_installer {
          let reqs = &[package_ref.req().clone()];
          npm_installer
            .add_package_reqs(
              reqs,
              if matches!(
                self.default_npm_caching_strategy,
                NpmCachingStrategy::Lazy
              ) {
                PackageCaching::Only(reqs.into())
              } else {
                PackageCaching::All
              },
            )
            .await
            .map_err(CreateCustomWorkerError::NpmPackageReq)?;
        }
        let referrer =
          ModuleSpecifier::from_directory_path(self.sys.env_current_dir()?)
            .unwrap()
            .join("package.json")?;
        let package_folder =
          self.npm_resolver.resolve_pkg_folder_from_deno_module_req(
            package_ref.req(),
            &referrer,
          )?;
        let main_module =
          self.lib_main_worker_factory.resolve_npm_binary_entrypoint(
            &package_folder,
            package_ref.sub_path(),
          )?;
        if let Some(lockfile) = &self.maybe_lockfile {
          lockfile.write_if_changed()?;
        }
        main_module
      }
      _ => main_module,
    };
    let mut worker = self.lib_main_worker_factory.create_custom_worker(
      mode,
      main_module,
      preload_modules,
      permissions,
      custom_extensions,
      stdio,
      unconfigured_runtime,
    )?;
    if self.needs_test_modules {
      macro_rules! test_file {
                ($($file:literal),*) => {
                    $(worker.js_runtime()
                    .lazy_load_es_module_with_code(concat!("ext:cli/", $file),
                    deno_core::ascii_str_include!(concat!("js/", $file)),) ?;)*
                };
            }
      test_file!(
        "40_test_common.js",
        "40_test.js",
        "40_bench.js",
        "40_jupyter.js",
        "40_lint_selector.js",
        "40_lint.js"
      );
    }
    Ok(CliMainWorker {
      worker,
      shared: self.shared.clone(),
    })
  }
}
