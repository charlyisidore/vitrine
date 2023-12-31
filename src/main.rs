//! A scriptable static site generator written in Rust.

mod build;
mod cli;
mod config;
mod error;
mod serve;
mod util;
mod watch;

use clap::Parser;
use tracing_subscriber::prelude::*;

use crate::{
    cli::Cli,
    config::{load_config, load_config_default, normalize_config, validate_config, Config},
};

/// Entry point of the program.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Log format for debug mode
    #[cfg(debug_assertions)]
    let tracing_format = tracing_subscriber::fmt::layer();

    // Log format for release mode
    #[cfg(not(debug_assertions))]
    let tracing_format = tracing_subscriber::fmt::layer()
        .with_target(false)
        .without_time();

    // Display log messages on the console
    tracing_subscriber::registry()
        .with(tracing_format)
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let cli = Cli::parse();

    // If specified with `--config`, load the provided configuration file.
    // Otherwise, try `vitrine.config.json`, `vitrine.config.rhai`, etc. by default.
    let config = cli.config.map_or_else(
        || load_config_default(),
        |config_path| load_config(config_path),
    )?;

    // Override the configuration with CLI arguments
    let config = Config {
        input_dir: cli.input_dir.unwrap_or(config.input_dir),
        output_dir: cli
            .output_dir
            .or(config.output_dir)
            .filter(|_| !cli.dry_run),
        base_url: cli.base_url.unwrap_or(config.base_url),
        data_dir: cli.data_dir.or(config.data_dir),
        layouts_dir: cli.layouts_dir.or(config.layouts_dir),
        minify: !cli.serve && config.minify,
        serve_port: cli.port,
        ..config
    };

    // Normalize the configuration (e.g. make paths absolute)
    let config = normalize_config(config)?;

    // Check for problems in the configuration
    validate_config(&config)?;

    tracing::debug!("{:#?}", config);

    // Build the site
    build::build(&config)?;

    if cli.serve {
        let serve = serve::serve(&config);
        let watch = watch::watch(&config, || build::build(&config));

        tokio::try_join!(serve, watch)?;
    }

    Ok(())
}
