//! Configuration.

mod json;
mod lua;
mod rhai;
mod toml;
mod yaml;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{error::Error, util::path::PathExt};

/// Configuration for Vitrine.
///
/// This structure represents the configuration given to the site builder.
#[derive(Default)]
pub(crate) struct Config {
    /// Path to the configuration file.
    pub(crate) config_path: Option<PathBuf>,

    /// Directory of input files.
    pub(crate) input_dir: PathBuf,

    /// Directory of output files.
    pub(crate) output_dir: PathBuf,

    /// Prefix for URLs.
    pub(crate) base_url: String,

    /// Directory of data files.
    pub(crate) data_dir: Option<PathBuf>,

    /// Directory of layout files.
    pub(crate) layout_dir: Option<PathBuf>,

    /// Custom filters for the layout engine.
    pub(crate) layout_filters: HashMap<String, LayoutFilter>,

    /// Custom functions for the layout engine.
    pub(crate) layout_functions: HashMap<String, LayoutFunction>,

    /// Custom testers for the layout engine.
    pub(crate) layout_testers: HashMap<String, LayoutTester>,

    /// Prefix for syntax highlight CSS classes.
    pub(crate) syntax_highlight_css_prefix: String,

    /// Syntax highlight CSS stylesheets.
    pub(crate) syntax_highlight_stylesheets: Vec<SyntaxHighlightStylesheet>,

    /// Determine whether Vitrine should write output files or not.
    pub(crate) dry_run: bool,
}

// Some fields in `Config`` do not support `#[derive(Debug)]`
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("config_path", &self.config_path)
            .field("input_dir", &self.input_dir)
            .field("output_dir", &self.output_dir)
            .field("base_url", &self.base_url)
            .field("data_dir", &self.data_dir)
            .field("layout_dir", &self.layout_dir)
            .field("layout_filters", &self.layout_filters.keys())
            .field("layout_functions", &self.layout_functions.keys())
            .field("layout_testers", &self.layout_testers.keys())
            .field(
                "syntax_highlight_css_prefix",
                &self.syntax_highlight_css_prefix,
            )
            .field(
                "syntax_highlight_stylesheets",
                &self.syntax_highlight_stylesheets,
            )
            .field("dry_run", &self.dry_run)
            .finish()
    }
}

/// Deserializable configuration.
///
/// This structure has all its fields optional.
#[derive(Default, Deserialize)]
pub(crate) struct PartialConfig {
    /// Path to the configuration file.
    #[serde(skip)]
    pub(crate) config_path: Option<PathBuf>,

    /// Directory of input files.
    pub(crate) input_dir: Option<PathBuf>,

    /// Directory of output files.
    pub(crate) output_dir: Option<PathBuf>,

    /// Prefix for URLs.
    pub(crate) base_url: Option<String>,

    /// Directory of data files.
    pub(crate) data_dir: Option<PathBuf>,

    /// Directory of layout files.
    pub(crate) layout_dir: Option<PathBuf>,

    /// Custom filters for the layout engine.
    #[serde(skip)]
    pub(crate) layout_filters: HashMap<String, LayoutFilter>,

    /// Custom functions for the layout engine.
    #[serde(skip)]
    pub(crate) layout_functions: HashMap<String, LayoutFunction>,

    /// Custom testers for the layout engine.
    #[serde(skip)]
    pub(crate) layout_testers: HashMap<String, LayoutTester>,

    /// Prefix for syntax highlight CSS classes.
    #[serde(default)]
    pub(crate) syntax_highlight_css_prefix: String,

    /// Syntax highlight CSS stylesheets.
    #[serde(default)]
    pub(crate) syntax_highlight_stylesheets: Vec<SyntaxHighlightStylesheet>,
}

// Some fields in `PartialConfig`` do not support `#[derive(Debug)]`
impl std::fmt::Debug for PartialConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartialConfig")
            .field("config_path", &self.config_path)
            .field("input_dir", &self.input_dir)
            .field("output_dir", &self.output_dir)
            .field("base_url", &self.base_url)
            .field("data_dir", &self.data_dir)
            .field("layout_dir", &self.layout_dir)
            .field("layout_filters", &self.layout_filters.keys())
            .field("layout_functions", &self.layout_functions.keys())
            .field("layout_testers", &self.layout_testers.keys())
            .field(
                "syntax_highlight_css_prefix",
                &self.syntax_highlight_css_prefix,
            )
            .field(
                "syntax_highlight_stylesheets",
                &self.syntax_highlight_stylesheets,
            )
            .finish()
    }
}

/// Filter for the layout engine.
pub(crate) type LayoutFilter = Box<
    dyn Fn(&tera::Value, &HashMap<String, tera::Value>) -> tera::Result<tera::Value> + Sync + Send,
>;

/// Function for the layout engine.
pub(crate) type LayoutFunction =
    Box<dyn Fn(&HashMap<String, tera::Value>) -> tera::Result<tera::Value> + Sync + Send>;

/// Tester for the layout engine.
pub(crate) type LayoutTester =
    Box<dyn Fn(Option<&tera::Value>, &[tera::Value]) -> tera::Result<bool> + Sync + Send>;

/// Syntax highlight CSS stylesheet configuration.
#[derive(Debug, Deserialize)]
pub(crate) struct SyntaxHighlightStylesheet {
    /// Prefix for class names.
    #[serde(default)]
    pub(crate) prefix: String,

    /// Theme name.
    ///
    /// See <https://docs.rs/syntect/latest/syntect/highlighting/struct.ThemeSet.html>
    pub(crate) theme: String,

    /// Output URL of the stylesheet.
    pub(crate) url: String,
}

/// Load partial configuration from a file.
pub(super) fn load_config<P>(config_path: P) -> Result<PartialConfig, Error>
where
    P: AsRef<Path>,
{
    let config_path = config_path.as_ref();

    tracing::info!("Loading configuration from {:?}", config_path);

    let config = if let Some(extension) = config_path.extension().and_then(|v| v.to_str()) {
        match extension {
            "json" => self::json::load_config(config_path),
            "lua" => self::lua::load_config(config_path),
            "rhai" => self::rhai::load_config(config_path),
            "toml" => self::toml::load_config(config_path),
            "yaml" => self::yaml::load_config(config_path),
            _ => Err(anyhow::anyhow!("Unknown configuration file extension")),
        }
    } else {
        Err(anyhow::anyhow!("Missing configuration file extension"))
    }
    .map_err(|error| Error::LoadConfig {
        config_path: Some(config_path.to_owned()),
        source: error.into(),
    })?;

    Ok(PartialConfig {
        config_path: Some(config_path.to_owned()),
        ..config
    })
}

/// Normalize the configuration.
///
/// This function normalizes paths to make them absolute.
pub(super) fn normalize_config(config: Config) -> Result<Config, Error> {
    let config_path = config.config_path;

    // Use current directory's path to create absolute paths
    let current_dir = std::env::current_dir().map_err(|error| Error::LoadConfig {
        config_path: config_path.to_owned(),
        source: error.into(),
    })?;

    // Canonicalize input directory
    let input_dir = config
        .input_dir
        .canonicalize()
        .map_err(|error| Error::LoadConfig {
            config_path: config_path.to_owned(),
            source: anyhow::anyhow!(error).context("while normalizing input_dir"),
        })?;

    // Normalize output directory
    let output_dir = if config.output_dir.is_absolute() {
        config.output_dir
    } else {
        current_dir.join(config.output_dir)
    };

    // We don't use `canonicalize()` since the output directory might not exist yet
    let output_dir = output_dir.normalize();

    // Canonicalize data directory
    let data_dir = config
        .data_dir
        .map(|dir| dir.canonicalize())
        .transpose()
        .map_err(|error| Error::LoadConfig {
            config_path: config_path.to_owned(),
            source: anyhow::anyhow!(error).context("while normalizing data_dir"),
        })?;

    // Canonicalize layout directory
    let layout_dir = config
        .layout_dir
        .map(|dir| dir.canonicalize())
        .transpose()
        .map_err(|error| Error::LoadConfig {
            config_path: config_path.to_owned(),
            source: anyhow::anyhow!(error).context("while normalizing layout_dir"),
        })?;

    Ok(Config {
        config_path,
        input_dir,
        output_dir,
        data_dir,
        layout_dir,
        ..config
    })
}

/// Validate the configuration.
///
/// This function checks if the input directories are located inside the output
/// directory.
pub(super) fn validate_config(config: &Config) -> Result<(), Error> {
    // Protection against overwriting input files
    if config.input_dir.starts_with(&config.output_dir) {
        return Err(Error::LoadConfig {
            config_path: config.config_path.to_owned(),
            source: anyhow::anyhow!("input_dir must be located outside output_dir"),
        });
    }

    // Protection against overwriting data files
    if let Some(data_dir) = config.data_dir.as_ref() {
        if data_dir.starts_with(&config.output_dir) {
            return Err(Error::LoadConfig {
                config_path: config.config_path.to_owned(),
                source: anyhow::anyhow!("data_dir must be located outside output_dir"),
            });
        }
    }

    // Protection against overwriting layout files
    if let Some(layout_dir) = config.layout_dir.as_ref() {
        if layout_dir.starts_with(&config.output_dir) {
            return Err(Error::LoadConfig {
                config_path: config.config_path.to_owned(),
                source: anyhow::anyhow!("layout_dir must be located outside output_dir"),
            });
        }
    }

    Ok(())
}
