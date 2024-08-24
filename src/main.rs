//! A scriptable static site generator.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use futures::future::Either;
use vitrine::{Config, Url};

/// Command line usage description.
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// List of commands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Build the site
    Build {
        /// Build arguments
        #[command(flatten)]
        build_args: BuildArgs,
    },

    /// Serve the site
    Serve {
        /// Build arguments
        #[command(flatten)]
        build_args: BuildArgs,

        /// Server port
        #[arg(long, default_value_t = 8000)]
        port: u16,
    },
}

/// Arguments for `build` and `serve` commands.
#[derive(Debug, Args, Clone)]
struct BuildArgs {
    /// Configuration file [default: "vitrine.config.*"]
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Input directory [default: "."]
    #[arg(long)]
    pub input_dir: Option<PathBuf>,

    /// Output directory [default: "_site"]
    #[arg(long)]
    pub output_dir: Option<PathBuf>,

    /// Layout directory [default: "_layouts"]
    #[arg(long)]
    pub layout_dir: Option<PathBuf>,

    /// URL prefix [default: ""]
    #[arg(long)]
    pub base_url: Option<Url>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { build_args } => {
            let config = Config {
                optimize: true,
                ..create_config(build_args)?
            };

            vitrine::build(&config)?;
        },
        Command::Serve { build_args, port } => {
            // Reload configuration at each iteration
            loop {
                let config = Config {
                    optimize: false,
                    ..create_config(build_args.clone())?
                };

                let Some(output_dir) = &config.output_dir else {
                    panic!("no output directory specified");
                };

                vitrine::build(&config)?;

                let serve = vitrine::serve(&output_dir, port);

                let watch = vitrine::watch(&config, || {
                    vitrine::build(&config)
                        .map_err(|e| vitrine::watch::WatchError::Boxed(Box::new(e)))
                });

                let result = futures::future::try_select(Box::pin(serve), Box::pin(watch)).await;

                match result {
                    Err(Either::Left((e, _))) => Err(anyhow::Error::from(e)),
                    Err(Either::Right((e, _))) => Err(anyhow::Error::from(e)),
                    _ => Ok(()),
                }?;
            }
        },
    }

    Ok(())
}

/// Create configuration from command line arguments.
fn create_config(build_args: BuildArgs) -> anyhow::Result<Config> {
    let config = build_args.config.or_else(default_config_path).map_or_else(
        || Ok(Config::new()),
        |path| {
            println!("Loading configuration from `{}`", path.display());
            Config::from_file(path)
        },
    )?;

    let config = Config {
        input_dir: build_args.input_dir.unwrap_or(config.input_dir),
        output_dir: build_args.output_dir.or(config.output_dir),
        layout_dir: build_args.layout_dir.or(config.layout_dir),
        base_url: build_args.base_url.unwrap_or(config.base_url),
        ..config
    };

    let config = config.normalize()?;
    config.validate()?;

    Ok(config)
}

/// Return the path of a default configuration file, if the latter exists.
fn default_config_path() -> Option<PathBuf> {
    [
        #[cfg(feature = "v8")]
        "vitrine.config.js",
        "vitrine.config.json",
        #[cfg(feature = "mlua")]
        "vitrine.config.lua",
        #[cfg(feature = "rhai")]
        "vitrine.config.rhai",
        "vitrine.config.toml",
        "vitrine.config.yaml",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.exists())
}
