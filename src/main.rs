//! A scriptable static site generator.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { build_args } => {
            let config = Config {
                optimize: true,
                ..create_config(build_args)?
            };
            vitrine::build(&config)?;
        },
        Command::Serve {
            build_args,
            port: _port,
        } => {
            let _config = Config {
                optimize: false,
                ..create_config(build_args)?
            };
            todo!();
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
        #[cfg(feature = "js")]
        "vitrine.config.js",
        "vitrine.config.json",
        #[cfg(feature = "lua")]
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
