use crate::npm::CliNpmResolver;
use crate::sys::CliSys;
use deno_resolver::npm::DenoInNpmPackageChecker;
use node_resolver::DenoIsBuiltInNodeModuleChecker;
pub type CliCjsTracker =
  deno_resolver::cjs::CjsTracker<DenoInNpmPackageChecker, CliSys>;
pub type CliResolver = deno_resolver::graph::DenoResolver<
  DenoInNpmPackageChecker,
  DenoIsBuiltInNodeModuleChecker,
  CliNpmResolver,
  CliSys,
>;
pub fn on_resolve_diagnostic(
  diagnostic: deno_resolver::graph::MappedResolutionDiagnosticWithPosition,
) {
  log::warn!(
    "{} {}\n    at {}:{}",
    deno_runtime::colors::yellow("Warning"),
    diagnostic.diagnostic,
    diagnostic.referrer,
    diagnostic.start
  );
}
