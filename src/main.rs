//! Hackable static site generator.

use anyhow::Result;
use log::LevelFilter;
use vitrine::cli::{Cli, Command, Parser};
#[cfg(feature = "deno")]
use vitrine_deno::deno_runtime::deno_core::JsRuntime;

fn main() -> Result<()> {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .format_timestamp(None)
        .format_target(false)
        .parse_default_env()
        .init();

    #[cfg(feature = "deno")]
    {
        vitrine_deno::rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .unwrap();
        JsRuntime::init_platform(None, false);
    }

    let cli = Cli::parse();

    match cli.command {
        Command::Build => vitrine::cli::build(&cli.opts)?,
        Command::Serve { port } => vitrine::cli::serve(&cli.opts, port)?,
    }

    Ok(())
}
