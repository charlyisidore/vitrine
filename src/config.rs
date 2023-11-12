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
use vitrine_derive::{FromLua, FromRhai};

use crate::{
    error::Error,
    util::{function::Function, path::PathExt},
};

/// Default file names for configuration files.
const DEFAULT_CONFIG_FILE_NAMES: [&str; 5] = [
    "vitrine.config.json",
    "vitrine.config.lua",
    "vitrine.config.rhai",
    "vitrine.config.toml",
    "vitrine.config.yaml",
];

/// Return the default input directory.
fn default_input_dir() -> PathBuf {
    PathBuf::from(".")
}

/// Return the default output directory.
fn default_output_dir() -> Option<PathBuf> {
    Some(PathBuf::from("_site"))
}

/// Return the default base URL.
fn default_base_url() -> String {
    String::from("")
}

/// Return the default data directory.
fn default_data_dir() -> Option<PathBuf> {
    // Returns the path only if it exists
    Some(PathBuf::from("_data")).filter(|path| path.exists())
}

/// Return the default layout directory.
fn default_layout_dir() -> Option<PathBuf> {
    // Returns the path only if it exists
    Some(PathBuf::from("_layouts")).filter(|path| path.exists())
}

/// Return the default name of the content variable in layouts.
fn default_layout_content_key() -> String {
    "content".to_owned()
}

/// Return the default name of the layout key in front matter data.
fn default_layout_layout_key() -> String {
    "layout".to_owned()
}

/// Configuration for Vitrine.
///
/// This structure represents the configuration given to the site builder.
#[derive(Debug, Deserialize, FromLua, FromRhai)]
pub(crate) struct Config {
    /// Path to the configuration file.
    #[serde(skip)]
    #[vitrine(skip)]
    pub(crate) config_path: Option<PathBuf>,

    /// Directory of input files.
    #[serde(default = "default_input_dir")]
    #[vitrine(default = "default_input_dir")]
    pub(crate) input_dir: PathBuf,

    /// Directory of output files.
    ///
    /// If set to `None`, Vitrine does not write files.
    #[serde(default = "default_output_dir")]
    #[vitrine(default = "default_output_dir")]
    pub(crate) output_dir: Option<PathBuf>,

    /// Prefix for URLs.
    #[serde(default = "default_base_url")]
    #[vitrine(default = "default_base_url")]
    pub(crate) base_url: String,

    /// Directory of data files.
    ///
    /// If set to `None`, Vitrine does not search for data files.
    #[serde(default = "default_data_dir")]
    #[vitrine(default = "default_data_dir")]
    pub(crate) data_dir: Option<PathBuf>,

    /// Global data.
    ///
    /// This data is merged with the data loaded from the directory specified in
    /// [`Config::data_dir`].
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) global_data: serde_json::Value,

    /// Directory of layout files.
    ///
    /// If set to `None`, Vitrine does not use a layout engine.
    #[serde(default = "default_layout_dir")]
    #[vitrine(default = "default_layout_dir")]
    pub(crate) layout_dir: Option<PathBuf>,

    /// Layout engine configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) layout: LayoutConfig,

    /// Syntax highlight configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) syntax_highlight: SyntaxHighlightConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_path: Default::default(),
            input_dir: default_input_dir(),
            output_dir: default_output_dir(),
            base_url: default_base_url(),
            data_dir: default_data_dir(),
            global_data: Default::default(),
            layout_dir: default_layout_dir(),
            layout: Default::default(),
            syntax_highlight: Default::default(),
        }
    }
}

/// Configuration for the layout engine.
#[derive(Debug, Deserialize, FromLua, FromRhai)]
pub(crate) struct LayoutConfig {
    /// Name of the template variable representing the content.
    #[serde(default = "default_layout_content_key")]
    #[vitrine(default = "default_layout_content_key")]
    pub(crate) content_key: String,

    /// Name of the metadata key containing the layout name.
    #[serde(default = "default_layout_layout_key")]
    #[vitrine(default = "default_layout_layout_key")]
    pub(crate) layout_key: String,

    /// Custom filters for the layout engine.
    #[serde(skip)]
    #[vitrine(default)]
    pub(crate) filters: HashMap<String, Function>,

    /// Custom functions for the layout engine.
    #[serde(skip)]
    #[vitrine(default)]
    pub(crate) functions: HashMap<String, Function>,

    /// Custom testers for the layout engine.
    #[serde(skip)]
    #[vitrine(default)]
    pub(crate) testers: HashMap<String, Function>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            content_key: default_layout_content_key(),
            layout_key: default_layout_layout_key(),
            filters: Default::default(),
            functions: Default::default(),
            testers: Default::default(),
        }
    }
}

/// Configuration for syntax highlight.
#[derive(Debug, Default, Deserialize, FromLua, FromRhai)]
pub(crate) struct SyntaxHighlightConfig {
    /// HTML attributes for syntax highlight `<code>` element.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) code_attributes: HashMap<String, String>,

    /// HTML attributes for syntax highlight `<pre>` element.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) pre_attributes: HashMap<String, String>,

    /// Prefix for syntax highlight CSS classes.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) css_prefix: String,

    /// Formatters for syntax highlight.
    #[serde(skip)]
    #[vitrine(default)]
    pub(crate) formatter: Option<Function>,

    /// Syntax highlight CSS stylesheets.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) stylesheets: Vec<SyntaxHighlightStylesheetConfig>,
}

/// Configuration for a syntax highlight CSS stylesheet.
#[derive(Debug, Deserialize, FromLua, FromRhai)]
pub(crate) struct SyntaxHighlightStylesheetConfig {
    /// Prefix for class names.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) prefix: String,

    /// Theme name.
    ///
    /// See <https://docs.rs/syntect/latest/syntect/highlighting/struct.ThemeSet.html>
    pub(crate) theme: String,

    /// Output URL of the stylesheet.
    pub(crate) url: String,
}

/// Load configuration from a default file (e.g. `vitrine.config.json`).
///
/// Default file names are specified in [`DEFAULT_CONFIG_FILE_NAMES`].
pub(super) fn load_config_default() -> Result<Config, Error> {
    Ok(DEFAULT_CONFIG_FILE_NAMES
        .into_iter()
        .map(|file_name| Path::new(file_name))
        .find(|path| path.exists())
        .map(|path| load_config(path))
        .transpose()?
        .unwrap_or_default())
}

/// Load configuration from a file.
pub(super) fn load_config<P>(config_path: P) -> Result<Config, Error>
where
    P: AsRef<Path>,
{
    let config_path = config_path.as_ref();

    tracing::info!("Loading configuration from {:?}", config_path);

    let config: Config =
        if let Some(extension) = config_path.extension().and_then(|v| v.to_str()) {
            match extension {
                "json" => crate::util::data::json::read_file(config_path),
                "lua" => self::lua::load_config(config_path),
                "rhai" => self::rhai::load_config(config_path),
                "toml" => crate::util::data::toml::read_file(config_path),
                "yaml" => crate::util::data::yaml::read_file(config_path),
                _ => Err(anyhow::anyhow!("Unknown configuration file extension")),
            }
        } else {
            Err(anyhow::anyhow!("Missing configuration file extension"))
        }
        .map_err(|error| Error::LoadConfig {
            config_path: Some(config_path.to_owned()),
            source: error.into(),
        })?;

    Ok(Config {
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
            source: anyhow::anyhow!(error).context(format!(
                "While normalizing input_dir: {:?}",
                config.input_dir
            )),
        })?;

    // Normalize output directory
    let output_dir = config.output_dir.map(|output_dir| {
        // We don't use `canonicalize()` since the output directory might not exist yet
        if output_dir.is_absolute() {
            output_dir
        } else {
            current_dir.join(output_dir)
        }
        .normalize()
    });

    // Canonicalize data directory
    let data_dir = config
        .data_dir
        .as_ref()
        .map(|dir| dir.canonicalize())
        .transpose()
        .map_err(|error| Error::LoadConfig {
            config_path: config_path.to_owned(),
            source: anyhow::anyhow!(error)
                .context(format!("While normalizing data_dir: {:?}", config.data_dir)),
        })?;

    // Canonicalize layout directory
    let layout_dir = config
        .layout_dir
        .as_ref()
        .map(|dir| dir.canonicalize())
        .transpose()
        .map_err(|error| Error::LoadConfig {
            config_path: config_path.to_owned(),
            source: anyhow::anyhow!(error).context(format!(
                "While normalizing layout_dir: {:?}",
                config.layout_dir
            )),
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
    if let Some(output_dir) = config.output_dir.as_ref() {
        // Protection against overwriting input files
        if config.input_dir.starts_with(output_dir) {
            return Err(Error::LoadConfig {
                config_path: config.config_path.to_owned(),
                source: anyhow::anyhow!("input_dir must be located outside output_dir"),
            });
        }

        // Protection against overwriting data files
        if let Some(data_dir) = config.data_dir.as_ref() {
            if data_dir.starts_with(output_dir) {
                return Err(Error::LoadConfig {
                    config_path: config.config_path.to_owned(),
                    source: anyhow::anyhow!("data_dir must be located outside output_dir"),
                });
            }
        }

        // Protection against overwriting layout files
        if let Some(layout_dir) = config.layout_dir.as_ref() {
            if layout_dir.starts_with(output_dir) {
                return Err(Error::LoadConfig {
                    config_path: config.config_path.to_owned(),
                    source: anyhow::anyhow!("layout_dir must be located outside output_dir"),
                });
            }
        }
    }

    Ok(())
}
