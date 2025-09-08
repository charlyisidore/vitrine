//! This module provides file formatting utilities using
//! [`dprint-plugin-typescript`](https://github.com/dprint/dprint-plugin-typescript).
//!
//! At the moment it is only consumed using CLI but in
//! the future it can be easily extended to provide
//! the same functions as ops available in JS runtime.
use crate::args::FmtOptionsConfig;
use deno_core::error::AnyError;
use std::path::Path;
/// Formats JSON and JSONC using the rules provided by .deno()
/// of configuration builder of <https://github.com/dprint/dprint-plugin-json>.
/// See <https://github.com/dprint/dprint-plugin-json/blob/cfa1052dbfa0b54eb3d814318034cdc514c813d7/src/configuration/builder.rs#L87> for configuration.
pub fn format_json(
  file_path: &Path,
  file_text: &str,
  fmt_options: &FmtOptionsConfig,
) -> Result<Option<String>, AnyError> {
  let config = get_resolved_json_config(fmt_options);
  dprint_plugin_json::format_text(file_path, file_text, &config)
}
fn get_resolved_json_config(
  options: &FmtOptionsConfig,
) -> dprint_plugin_json::configuration::Configuration {
  let mut builder =
    dprint_plugin_json::configuration::ConfigurationBuilder::new();
  builder.deno();
  if let Some(use_tabs) = options.use_tabs {
    builder.use_tabs(use_tabs);
  }
  if let Some(line_width) = options.line_width {
    builder.line_width(line_width);
  }
  if let Some(indent_width) = options.indent_width {
    builder.indent_width(indent_width);
  }
  builder.build()
}
