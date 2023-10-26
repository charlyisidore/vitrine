//! Command line options.

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub(super) struct Cli {
    /// Configuration file [default: "vitrine.config.rhai"]
    #[arg(long)]
    pub(super) config: Option<PathBuf>,

    /// Input directory [default: "."]
    #[arg(long)]
    pub(super) input_dir: Option<PathBuf>,

    /// Output directory [default: "_site"]
    #[arg(long)]
    pub(super) output_dir: Option<PathBuf>,

    /// URL prefix [default: ""]
    #[arg(long)]
    pub(super) base_url: Option<String>,

    /// Data directory [default: "_data"]
    #[arg(long)]
    pub(super) data_dir: Option<PathBuf>,

    /// Layout directory [default: "_layouts"]
    #[arg(long)]
    pub(super) layout_dir: Option<PathBuf>,

    /// Do not write output files
    #[arg(long)]
    pub(super) dry_run: bool,
}
