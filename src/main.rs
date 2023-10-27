//! A scriptable static site generator written in Rust.

mod build;
mod cli;
mod config;
mod error;
mod util;

use std::path::Path;

use clap::Parser;
use tracing_subscriber::prelude::*;

use crate::{
    build::build,
    cli::Cli,
    config::{load_config, normalize_config, validate_config, Config},
};

/// Default file names for configuration files
const DEFAULT_CONFIG_FILE_NAMES: [&str; 5] = [
    "vitrine.config.json",
    "vitrine.config.lua",
    "vitrine.config.rhai",
    "vitrine.config.toml",
    "vitrine.config.yaml",
];

/// Default directory for input files
const DEFAULT_INPUT_DIR: &str = ".";

/// Default directory for output files
const DEFAULT_OUTPUT_DIR: &str = "_site";

/// Default directory for data files
const DEFAULT_DATA_DIR: &str = "_data";

/// Default directory for layouts
const DEFAULT_LAYOUT_DIR: &str = "_layouts";

/// Entry point of the program.
fn main() -> anyhow::Result<()> {
    // Display log messages on the console
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let cli = Cli::parse();

    // If specified with `--config`, load the provided configuration file.
    // Otherwise, try `vitrine.config.json`, `vitrine.config.rhai`, etc.
    // Otherwise, create an empty configuration object.
    let config = cli
        .config
        .or_else(|| {
            DEFAULT_CONFIG_FILE_NAMES
                .into_iter()
                .map(|file_name| Path::new(file_name))
                .find(|path| path.exists())
                .map(|path| path.to_owned())
        })
        .map(|config_path| load_config(config_path))
        .transpose()?
        .unwrap_or_default();

    // Override the configuration with CLI arguments
    let config = Config {
        // Configuration file path
        config_path: config.config_path,

        // Input directory
        input_dir: cli
            .input_dir
            .or(config.input_dir)
            .unwrap_or_else(|| DEFAULT_INPUT_DIR.into()),

        // Output directory
        output_dir: cli
            .output_dir
            .or(config.output_dir)
            .unwrap_or_else(|| DEFAULT_OUTPUT_DIR.into()),

        // URL prefix
        base_url: cli.base_url.or(config.base_url).unwrap_or_default(),

        // Data directory
        data_dir: cli.data_dir.or(config.data_dir).or_else(|| {
            // Defaults to `DEFAULT_DATA_DIR`, but only if it exists
            Some(DEFAULT_DATA_DIR)
                .map(|dir| Path::new(dir))
                .filter(|path| path.exists())
                .map(|path| path.to_owned())
        }),

        // Layout directory
        layout_dir: cli.layout_dir.or(config.layout_dir).or_else(|| {
            // Defaults to `DEFAULT_LAYOUT_DIR`, but only if it exists
            Some(DEFAULT_LAYOUT_DIR)
                .map(|dir| Path::new(dir))
                .filter(|path| path.exists())
                .map(|path| path.to_owned())
        }),

        // Filters for layouts
        layout_filters: config.layout_filters,

        // Functions for layouts
        layout_functions: config.layout_functions,

        // Test functions for layouts
        layout_tests: config.layout_tests,

        // Prefix for syntax highlight CSS class names
        syntax_highlight_css_prefix: config.syntax_highlight_css_prefix,

        // Syntax highlight CSS stylesheets
        syntax_highlight_stylesheets: config.syntax_highlight_stylesheets,

        // Do not write files
        dry_run: cli.dry_run,
    };

    // Normalize the configuration (e.g. make paths absolute)
    let config = normalize_config(config)?;

    // Check for problems in the configuration
    validate_config(&config)?;

    tracing::debug!("{:#?}", config);

    // Build the site
    build(&config)?;

    Ok(())
}
