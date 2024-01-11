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

/// Return the default layouts directory.
fn default_layouts_dir() -> Option<PathBuf> {
    // Returns the path only if it exists
    Some(PathBuf::from("_layouts")).filter(|path| path.exists())
}

/// Return the default name of the content variable in layouts.
fn default_layouts_content_key() -> String {
    "content".to_owned()
}

/// Return the default name of the layout key in front matter data.
fn default_layouts_layout_key() -> String {
    "layout".to_owned()
}

/// Return the default name of the page variable in layouts.
fn default_layouts_page_key() -> String {
    "page".to_owned()
}

/// Return the default URL of the sitemap.
fn default_sitemap_url() -> String {
    "/sitemap.xml".to_owned()
}

/// Return the default value for the `minify` option.
fn default_minify() -> bool {
    true
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

    /// Feeds configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) feeds: Vec<FeedConfig>,

    /// Directory of layout files.
    ///
    /// If set to `None`, Vitrine does not use a layout engine.
    #[serde(default = "default_layouts_dir")]
    #[vitrine(default = "default_layouts_dir")]
    pub(crate) layouts_dir: Option<PathBuf>,

    /// Layout engine configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) layouts: LayoutsConfig,

    /// Sitemap configuration.
    pub(crate) sitemap: Option<SitemapConfig>,

    /// Syntax highlight configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) syntax_highlight: SyntaxHighlightConfig,

    /// Taxonomies configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) taxonomies: Vec<String>,

    /// Ignore specific files or path patterns.
    #[serde(default)]
    #[vitrine(default)]
    pub(crate) ignore: Vec<String>,

    /// Paths to ignore from input files.
    #[serde(skip)]
    #[vitrine(skip)]
    pub(crate) input_ignore_paths: Vec<PathBuf>,

    /// Determine whether CSS, HTML and JS should be minified.
    #[serde(default = "default_minify")]
    #[vitrine(default = "default_minify")]
    pub(crate) minify: bool,

    /// Server port.
    #[serde(skip)]
    #[vitrine(skip)]
    pub(crate) serve_port: u16,
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
            feeds: Default::default(),
            layouts_dir: default_layouts_dir(),
            layouts: Default::default(),
            sitemap: Default::default(),
            syntax_highlight: Default::default(),
            taxonomies: Default::default(),
            ignore: Default::default(),
            input_ignore_paths: Default::default(),
            minify: default_minify(),
            serve_port: Default::default(),
        }
    }
}

/// Configuration for feed generation.
#[derive(Debug, Default, Deserialize, FromLua, FromRhai)]
pub(crate) struct FeedConfig {
    /// URL of the feed.
    pub(crate) url: String,

    /// Authors of the feed.
    pub(crate) author: Vec<FeedPersonConfig>,

    /// Categories of the feed.
    pub(crate) category: Vec<String>,

    /// Contributors of the feed.
    pub(crate) contributor: Vec<FeedPersonConfig>,

    /// Generator of the feed.
    pub(crate) generator: Option<String>,

    /// Image that provides iconic visual identification for the feed.
    pub(crate) icon: Option<String>,

    /// Unique identifier of the feed.
    pub(crate) id: Option<String>,

    /// Image that provides visual identification for the feed.
    pub(crate) logo: Option<String>,

    /// Information about rights held in and over the feed.
    pub(crate) rights: Option<String>,

    /// Description or subtitle for the feed.
    pub(crate) subtitle: Option<String>,

    /// Title for the feed.
    pub(crate) title: String,

    /// The most recent instant in time when the feed was modified.
    pub(crate) updated: Option<String>,

    /// Predicate that determines whether an entry belongs to the feed or not.
    #[serde(skip)]
    #[vitrine(default)]
    pub(crate) filter: Option<Function>,
}

/// Configuration for feed persons (author or contributor).
#[derive(Debug, Default, Deserialize, FromLua, FromRhai)]
pub(crate) struct FeedPersonConfig {
    /// Person name.
    pub(crate) name: String,

    /// Person website.
    pub(crate) uri: Option<String>,

    /// Person email.
    pub(crate) email: Option<String>,
}

/// Configuration for the layout engine.
#[derive(Debug, Deserialize, FromLua, FromRhai)]
pub(crate) struct LayoutsConfig {
    /// Name of the template variable representing the content.
    #[serde(default = "default_layouts_content_key")]
    #[vitrine(default = "default_layouts_content_key")]
    pub(crate) content_key: String,

    /// Name of the metadata key containing the layout name.
    #[serde(default = "default_layouts_layout_key")]
    #[vitrine(default = "default_layouts_layout_key")]
    pub(crate) layout_key: String,

    /// Name of the metadata key containing the page data.
    #[serde(default = "default_layouts_page_key")]
    #[vitrine(default = "default_layouts_page_key")]
    pub(crate) page_key: String,

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

impl Default for LayoutsConfig {
    fn default() -> Self {
        Self {
            content_key: default_layouts_content_key(),
            layout_key: default_layouts_layout_key(),
            page_key: default_layouts_page_key(),
            filters: Default::default(),
            functions: Default::default(),
            testers: Default::default(),
        }
    }
}

/// Configuration object for sitemap generation.
#[derive(Debug, Default, Deserialize, FromLua, FromRhai)]
pub(crate) struct SitemapConfig {
    /// Default page change frequency.
    pub(crate) changefreq: Option<String>,

    /// Default priority.
    pub(crate) priority: Option<f64>,

    /// URL of the sitemap.
    #[serde(default = "default_sitemap_url")]
    #[vitrine(default = "default_sitemap_url")]
    pub(crate) url: String,
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
                "lua" => crate::util::data::lua::read_file(config_path),
                "rhai" => crate::util::data::rhai::read_file(config_path),
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

    // Canonicalize config path
    let config_path = config_path
        .as_ref()
        .map(|path| path.canonicalize())
        .transpose()
        .map_err(|error| Error::LoadConfig {
            config_path: config_path.to_owned(),
            source: anyhow::anyhow!(error)
                .context(format!("While normalizing config path: {:?}", config_path)),
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

    // Canonicalize layouts directory
    let layouts_dir = config
        .layouts_dir
        .as_ref()
        .map(|dir| dir.canonicalize())
        .transpose()
        .map_err(|error| Error::LoadConfig {
            config_path: config_path.to_owned(),
            source: anyhow::anyhow!(error).context(format!(
                "While normalizing layouts_dir: {:?}",
                config.layouts_dir
            )),
        })?;

    // Paths to ignore from input files
    let mut input_ignore_paths = config.input_ignore_paths;

    // Exclude configuration file
    if let Some(config_path) = config_path.as_ref() {
        debug_assert!(config_path.is_absolute());
        input_ignore_paths.push(config_path.to_owned());
    }

    // Exclude output directory
    if let Some(output_dir) = output_dir.as_ref() {
        debug_assert!(output_dir.is_absolute());
        input_ignore_paths.push(output_dir.to_owned());
    }

    // Exclude data directory
    if let Some(data_dir) = data_dir.as_ref() {
        debug_assert!(data_dir.is_absolute());
        input_ignore_paths.push(data_dir.to_owned());
    }

    // Exclude layouts directory
    if let Some(layouts_dir) = layouts_dir.as_ref() {
        debug_assert!(layouts_dir.is_absolute());
        input_ignore_paths.push(layouts_dir.to_owned());
    }

    Ok(Config {
        config_path,
        input_dir,
        output_dir,
        data_dir,
        layouts_dir,
        input_ignore_paths,
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
        if let Some(layouts_dir) = config.layouts_dir.as_ref() {
            if layouts_dir.starts_with(output_dir) {
                return Err(Error::LoadConfig {
                    config_path: config.config_path.to_owned(),
                    source: anyhow::anyhow!("layouts_dir must be located outside output_dir"),
                });
            }
        }
    }

    Ok(())
}
