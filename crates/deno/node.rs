use crate::npm::CliNpmResolver;
use crate::sys::CliSys;
use deno_resolver::npm::DenoInNpmPackageChecker;
pub type CliNodeResolver = deno_runtime::deno_node::NodeResolver<
  DenoInNpmPackageChecker,
  CliNpmResolver,
  CliSys,
>;
pub type CliPackageJsonResolver = node_resolver::PackageJsonResolver<CliSys>;
