//! Configure the site builder.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;
#[cfg(feature = "js")]
use vitrine_derive::FromJs;
#[cfg(feature = "lua")]
use vitrine_derive::FromLua;
#[cfg(feature = "rhai")]
use vitrine_derive::FromRhai;
use vitrine_derive::VitrineNoop;

use crate::{
    build::{
        feeds::FeedFilter,
        layout::{LayoutFilter, LayoutFunction, LayoutTest},
        markdown::syntax_highlight::SyntaxHighlighter,
    },
    util::{
        path::PathExt,
        url::{Url, UrlPath},
        value::Value,
    },
};

/// List of configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Error loading JavaScript.
    #[cfg(feature = "js")]
    #[error("failed to load JavaScript")]
    FromJs(#[source] crate::util::eval::js::JsError),
    /// Error loading JSON.
    #[error("failed to load JSON")]
    FromJson(#[source] crate::util::eval::json::JsonError),
    /// Error loading Lua.
    #[cfg(feature = "lua")]
    #[error("failed to load Lua")]
    FromLua(#[source] crate::util::eval::lua::LuaError),
    /// Error loading Rhai.
    #[cfg(feature = "rhai")]
    #[error("failed to load Rhai")]
    FromRhai(#[source] crate::util::eval::rhai::RhaiError),
    /// Error loading TOML.
    #[error("failed to load TOML")]
    FromToml(#[source] crate::util::eval::toml::TomlError),
    /// Error loading YAML.
    #[error("failed to load YAML")]
    FromYaml(#[source] crate::util::eval::yaml::YamlError),
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Missing file extension error.
    #[error("missing file extension")]
    MissingFileExtension,
    /// Error normalizing configuration.
    #[error("failed to normalize configuration")]
    Normalize(#[source] Box<Self>),
    /// Error validating configuration.
    #[error("failed to validate configuration")]
    Validate(String),
    /// Unsupported file extension error.
    #[error("unsupported file extension `{extension}`")]
    UnsupportedFileExtension {
        /// The file extension.
        extension: String,
    },
    /// Add a file path to error context.
    #[error("file `{path}`")]
    WithFile {
        /// Source error.
        source: Box<Self>,
        /// File path.
        path: PathBuf,
    },
    /// Add a directory path to error context.
    #[error("directory `{path}`")]
    WithDir {
        /// Source error.
        source: Box<Self>,
        /// Directory path.
        path: PathBuf,
    },
}

/// Type used for maps in [`Config`].
pub type Map<K, V> = std::collections::HashMap<K, V>;

/// Configuration for the site builder.
#[derive(Clone, Debug, Default, Deserialize, VitrineNoop)]
#[cfg_attr(feature = "js", derive(FromJs))]
#[cfg_attr(feature = "lua", derive(FromLua))]
#[cfg_attr(feature = "rhai", derive(FromRhai))]
pub struct Config {
    /// Configuration file path, if any.
    #[serde(skip)]
    #[vitrine(skip)]
    pub config_path: Option<PathBuf>,

    /// Base URL of the site.
    #[serde(default)]
    #[vitrine(default)]
    pub base_url: Url,

    /// Directory of input files.
    #[serde(default = "default_input_dir")]
    #[vitrine(default = "default_input_dir")]
    pub input_dir: PathBuf,

    /// Directory of output files.
    ///
    /// If set to `None`, no file is written.
    #[serde(default = "default_output_dir")]
    #[vitrine(default = "default_output_dir")]
    pub output_dir: Option<PathBuf>,

    /// Directory of layout files.
    ///
    /// If set to `None`, no layout is rendered.
    #[serde(default = "default_layout_dir")]
    #[vitrine(default = "default_layout_dir")]
    pub layout_dir: Option<PathBuf>,

    /// Layout engine configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub layout: LayoutConfig,

    /// Feeds configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub feeds: Vec<FeedConfig>,

    /// Markdown configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub markdown: MarkdownConfig,

    /// Site data.
    #[serde(default)]
    #[vitrine(default)]
    pub site_data: Value,

    /// Sitemap configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub sitemap: Option<SitemapConfig>,

    /// Syntax highlight configuration.
    #[serde(default)]
    #[vitrine(default)]
    pub syntax_highlight: SyntaxHighlightConfig,

    /// Determine if the site should be optimized (minified, compressed...).
    #[serde(skip)]
    #[vitrine(skip)]
    pub optimize: bool,
}

/// Configuration for feed generation.
#[derive(Clone, Debug, Default, Deserialize, VitrineNoop)]
#[cfg_attr(feature = "js", derive(FromJs))]
#[cfg_attr(feature = "lua", derive(FromLua))]
#[cfg_attr(feature = "rhai", derive(FromRhai))]
pub struct FeedConfig {
    /// URL of the feed.
    pub url: String,

    /// Authors of the feed.
    #[serde(default)]
    #[vitrine(default)]
    pub author: Vec<FeedPersonConfig>,

    /// Categories of the feed.
    #[serde(default)]
    #[vitrine(default)]
    pub category: Vec<String>,

    /// Contributors of the feed.
    #[serde(default)]
    #[vitrine(default)]
    pub contributor: Vec<FeedPersonConfig>,

    /// Generator of the feed.
    #[serde(default)]
    #[vitrine(default)]
    pub generator: Option<String>,

    /// Image that provides iconic visual identification for the feed.
    #[serde(default)]
    #[vitrine(default)]
    pub icon: Option<String>,

    /// Unique identifier of the feed.
    #[serde(default)]
    #[vitrine(default)]
    pub id: Option<String>,

    /// Image that provides visual identification for the feed.
    #[serde(default)]
    #[vitrine(default)]
    pub logo: Option<String>,

    /// Information about rights held in and over the feed.
    #[serde(default)]
    #[vitrine(default)]
    pub rights: Option<String>,

    /// Description or subtitle for the feed.
    #[serde(default)]
    #[vitrine(default)]
    pub subtitle: Option<String>,

    /// Title for the feed.
    pub title: String,

    /// The most recent instant in time when the feed was modified.
    #[serde(default)]
    #[vitrine(default)]
    pub updated: Option<String>,

    /// Predicate that determines whether an entry belongs to the feed or not.
    #[serde(skip)]
    #[vitrine(default)]
    pub filter: Option<FeedFilter>,
}

/// Configuration for feed persons (author or contributor).
#[derive(Clone, Debug, Default, Deserialize, VitrineNoop)]
#[cfg_attr(feature = "js", derive(FromJs))]
#[cfg_attr(feature = "lua", derive(FromLua))]
#[cfg_attr(feature = "rhai", derive(FromRhai))]
pub struct FeedPersonConfig {
    /// Person name.
    pub name: String,

    /// Person website.
    #[serde(default)]
    #[vitrine(default)]
    pub uri: Option<String>,

    /// Person email.
    #[serde(default)]
    #[vitrine(default)]
    pub email: Option<String>,
}

/// Configuration for the layout engine.
#[derive(Clone, Debug, Default, Deserialize, VitrineNoop)]
#[cfg_attr(feature = "js", derive(FromJs))]
#[cfg_attr(feature = "lua", derive(FromLua))]
#[cfg_attr(feature = "rhai", derive(FromRhai))]
pub struct LayoutConfig {
    /// Engine identifier.
    #[serde(default)]
    #[vitrine(default)]
    pub engine: Option<String>,

    /// Custom filters.
    #[serde(skip)]
    #[vitrine(default)]
    pub filters: Map<String, LayoutFilter>,

    /// Custom functions.
    #[serde(skip)]
    #[vitrine(default)]
    pub functions: Map<String, LayoutFunction>,

    /// Custom tests.
    #[serde(skip)]
    #[vitrine(default)]
    pub tests: Map<String, LayoutTest>,
}

/// Configuration for the markdown parser.
#[derive(Clone, Debug, Default, Deserialize, VitrineNoop)]
#[cfg_attr(feature = "js", derive(FromJs))]
#[cfg_attr(feature = "lua", derive(FromLua))]
#[cfg_attr(feature = "rhai", derive(FromRhai))]
pub struct MarkdownConfig {
    /// List of plugins to add.
    #[serde(default)]
    #[vitrine(default)]
    pub plugins: Vec<String>,
}

/// Configuration for syntax highlight.
#[derive(Clone, Debug, Default, Deserialize, VitrineNoop)]
#[cfg_attr(feature = "js", derive(FromJs))]
#[cfg_attr(feature = "lua", derive(FromLua))]
#[cfg_attr(feature = "rhai", derive(FromRhai))]
pub struct SyntaxHighlightConfig {
    /// Prefix for CSS class names.
    #[serde(default)]
    #[vitrine(default)]
    pub css_prefix: String,

    /// HTML attributes for syntax highlight `<code>` element.
    #[serde(default)]
    #[vitrine(default)]
    pub code_attributes: Map<String, String>,

    /// HTML attributes for syntax highlight `<pre>` element.
    #[serde(default)]
    #[vitrine(default)]
    pub pre_attributes: Map<String, String>,

    /// Syntax highlight function.
    #[serde(skip)]
    #[vitrine(default)]
    pub highlighter: Option<SyntaxHighlighter>,

    /// Syntax highlight themes.
    #[serde(default)]
    #[vitrine(default)]
    pub themes: Vec<SyntaxHighlightThemeConfig>,
}

/// Configuration for sitemap generation.
#[derive(Clone, Debug, Default, Deserialize, VitrineNoop)]
#[cfg_attr(feature = "js", derive(FromJs))]
#[cfg_attr(feature = "lua", derive(FromLua))]
#[cfg_attr(feature = "rhai", derive(FromRhai))]
pub struct SitemapConfig {
    /// Default page change frequency.
    #[serde(default)]
    #[vitrine(default)]
    pub changefreq: Option<String>,

    /// Default priority.
    #[serde(default)]
    #[vitrine(default)]
    pub priority: Option<f64>,

    /// Domain to prepend to URLs, if `base_url` does not specify it.
    #[serde(default)]
    #[vitrine(default)]
    pub url_prefix: String,

    /// URL of the sitemap.
    #[serde(default = "default_sitemap_url")]
    #[vitrine(default = "default_sitemap_url")]
    pub url: UrlPath,
}

/// Configuration for a syntax highlight theme.
#[derive(Clone, Debug, Deserialize, VitrineNoop)]
#[cfg_attr(feature = "js", derive(FromJs))]
#[cfg_attr(feature = "lua", derive(FromLua))]
#[cfg_attr(feature = "rhai", derive(FromRhai))]
pub struct SyntaxHighlightThemeConfig {
    /// Theme name.
    ///
    /// See <https://docs.rs/syntect/latest/syntect/highlighting/struct.ThemeSet.html>
    pub name: String,

    /// Output URL of the stylesheet.
    pub url: UrlPath,

    /// Prefix for CSS selectors.
    #[serde(default)]
    #[vitrine(default)]
    pub selector: String,
}

impl Config {
    /// Create a configuration with default values.
    pub fn new() -> Self {
        Self {
            input_dir: default_input_dir(),
            output_dir: default_output_dir(),
            layout_dir: default_layout_dir(),
            ..Default::default()
        }
    }

    /// Load configuration from file path.
    ///
    /// This method selects the parser according to the file extension.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        let Some(extension) = path.extension().and_then(|v| v.to_str()) else {
            return Err(ConfigError::WithFile {
                source: Box::new(ConfigError::MissingFileExtension),
                path: path.to_owned(),
            });
        };

        let config: Self = match extension {
            #[cfg(feature = "js")]
            "js" => crate::util::eval::js::from_file(path).map_err(ConfigError::FromJs),
            "json" => crate::util::eval::json::from_file(path).map_err(ConfigError::FromJson),
            #[cfg(feature = "lua")]
            "lua" => crate::util::eval::lua::from_file(path).map_err(ConfigError::FromLua),
            #[cfg(feature = "rhai")]
            "rhai" => crate::util::eval::rhai::from_file(path).map_err(ConfigError::FromRhai),
            "toml" => crate::util::eval::toml::from_file(path).map_err(ConfigError::FromToml),
            "yaml" => crate::util::eval::yaml::from_file(path).map_err(ConfigError::FromYaml),
            _ => Err(ConfigError::UnsupportedFileExtension {
                extension: extension.to_owned(),
            }),
        }
        .map_err(|source| ConfigError::WithFile {
            source: Box::new(source),
            path: path.to_owned(),
        })?;

        Ok(Self {
            config_path: Some(path.to_path_buf()),
            ..config
        })
    }

    /// Normalize the configuration.
    ///
    /// This method normalizes paths to make them absolute.
    pub fn normalize(self) -> Result<Config, ConfigError> {
        // Canonicalize config path
        let config_path = self
            .config_path
            .as_ref()
            .map(|path| path.canonicalize())
            .transpose()
            .map_err(|source| {
                ConfigError::Normalize(Box::new(ConfigError::WithFile {
                    source: Box::new(source.into()),
                    path: self.config_path.expect("config_path must exist"),
                }))
            })?;

        // Canonicalize input directory
        let input_dir = self.input_dir.canonicalize().map_err(|source| {
            ConfigError::Normalize(Box::new(ConfigError::WithDir {
                source: Box::new(source.into()),
                path: self.input_dir,
            }))
        })?;

        // Normalize output directory
        let output_dir = self
            .output_dir
            .as_ref()
            .map(|path| path.to_absolute())
            .transpose()
            .map_err(|source| {
                ConfigError::Normalize(Box::new(ConfigError::WithDir {
                    source: Box::new(source.into()),
                    path: self.output_dir.expect("output_dir must exist"),
                }))
            })?;

        // Canonicalize layout directory
        let layout_dir = self
            .layout_dir
            .as_ref()
            .map(|dir| dir.canonicalize())
            .transpose()
            .map_err(|source| {
                ConfigError::Normalize(Box::new(ConfigError::WithDir {
                    source: Box::new(source.into()),
                    path: self.layout_dir.expect("layout_dir must exist"),
                }))
            })?;

        Ok(Config {
            config_path,
            input_dir,
            output_dir,
            layout_dir,
            ..self
        })
    }

    /// Validate the configuration.
    ///
    /// This method returns an error if the source directories are located
    /// inside the output directory.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(output_dir) = &self.output_dir {
            // Protection against overwriting config file
            if self
                .config_path
                .as_ref()
                .is_some_and(|path| path.starts_with(output_dir))
            {
                return Err(ConfigError::Validate(
                    "configuration file must be located outside `output_dir`".to_string(),
                ));
            }

            // Protection against overwriting input files
            if self.input_dir.starts_with(output_dir) {
                return Err(ConfigError::Validate(
                    "`input_dir` must be located outside `output_dir`".to_string(),
                ));
            }

            // Protection against overwriting layout files
            if let Some(layout_dir) = &self.layout_dir {
                if layout_dir.starts_with(output_dir) {
                    return Err(ConfigError::Validate(
                        "`layout_dir` must be located outside `output_dir`".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Return the default input directory.
pub fn default_input_dir() -> PathBuf {
    ".".into()
}

/// Return the default output directory.
pub fn default_output_dir() -> Option<PathBuf> {
    Some("_site".into())
}

/// Return the default layout directory if it exists.
pub fn default_layout_dir() -> Option<PathBuf> {
    // Returns the path only if it exists
    Some(PathBuf::from("_layouts")).filter(|path| path.exists())
}

/// Return the default URL of the sitemap.
fn default_sitemap_url() -> UrlPath {
    UrlPath::from("/sitemap.xml")
}
