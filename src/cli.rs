//! Command line utilities.

use std::{collections::HashSet, iter::once, path::PathBuf};

use anyhow::{Context, Result};
use async_channel::unbounded;
use axum::response::sse::Event;
pub use clap::Parser;
use clap::{Args, Subcommand};
use log::{error, info};

use crate::Config;

/// Command line usage description.
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// Cubcommand.
    #[command(subcommand)]
    pub command: Command,

    /// Command line options.
    #[command(flatten)]
    pub opts: Opts,
}

/// List of commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build the site
    Build,

    /// Serve the site
    Serve {
        /// Server port
        #[arg(long, default_value_t = 8000)]
        port: u16,
    },
}

/// Command line options.
#[derive(Debug, Args, Clone)]
pub struct Opts {
    /// Configuration file [default: "vitrine.config.ts"]
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Input directory [default: "."]
    #[arg(long, global = true)]
    pub input: Option<PathBuf>,

    /// Output directory [default: "_site"]
    #[arg(long, global = true)]
    pub output: Option<PathBuf>,

    /// URL prefix [default: ""]
    #[arg(long, global = true)]
    pub base_url: Option<String>,
}

/// Run the `build` command.
pub fn build(opts: &Opts) -> Result<()> {
    let (config, script) = Config::from_opts(opts).context("loading configuration")?;

    info!("Building...");

    crate::build(&config)?;

    if let Some((_, handle)) = script {
        drop(config);
        handle.join().unwrap()?;
    }

    Ok(())
}

/// Run the `serve` command.
pub fn serve(opts: &Opts, port: u16) -> Result<()> {
    loop {
        let (config, script) = Config::from_opts(opts).context("loading configuration")?;

        let config = Config {
            debug: true,
            ..config
        };

        let (shutdown_tx, shutdown_rx) = unbounded();
        let (sse_tx, sse_rx) = unbounded();

        info!("Building...");

        match crate::build(&config) {
            Ok(duration) => info!("Built in {} s", duration.as_secs_f32()),
            Err(error) => error!("{:?}", error),
        }

        let serve_handle = {
            let output_dir = config.output_dir.clone();
            std::thread::spawn(move || crate::serve(output_dir, port, sse_rx, shutdown_rx))
        };

        let config_path = script.as_ref().map(|(path, _)| path.clone());

        let watch_paths: HashSet<_> = [
            config_path.clone(),
            Some(config.input_dir.clone()),
            config.layout_dir.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();

        let ignore_paths: HashSet<_> = config
            .ignore_paths
            .iter()
            .chain(once(&config.output_dir))
            .cloned()
            .collect();

        info!("Watching...");

        crate::watch(watch_paths, ignore_paths, move |paths| {
            if config_path
                .as_ref()
                .is_some_and(|path| paths.contains(path))
            {
                info!("Reloading configuration...");

                shutdown_tx.send_blocking(())?;

                Ok(true)
            } else {
                info!("Rebuilding...");

                if let Err(error) = crate::build(&config) {
                    error!("{:?}", error);
                } else {
                    sse_tx.send_blocking(Ok(Event::default().event("reload").data("")))?;
                }

                Ok(false)
            }
        })?;

        serve_handle.join().unwrap().context("serving the site")?;

        if let Some((_, handle)) = script {
            handle.join().unwrap()?;
        }
    }
}
