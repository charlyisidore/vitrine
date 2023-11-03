//! Configuration.

mod json;
mod lua;
mod rhai;
mod toml;
mod yaml;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::Deserialize;

use crate::{error::Error, util::path::PathExt};

/// Default file names for configuration files
const DEFAULT_CONFIG_FILE_NAMES: [&str; 5] = [
    "vitrine.config.json",
    "vitrine.config.lua",
    "vitrine.config.rhai",
    "vitrine.config.toml",
    "vitrine.config.yaml",
];

/// Default input directory
const DEFAULT_INPUT_DIR: &str = ".";

/// Default output directory
const DEFAULT_OUTPUT_DIR: &str = "_site";

/// Default base URL
const DEFAULT_BASE_URL: &str = "";

/// Default data directory
const DEFAULT_DATA_DIR: &str = "_data";

/// Default layout directory
const DEFAULT_LAYOUT_DIR: &str = "_layouts";

/// Configuration for Vitrine.
///
/// This structure represents the configuration given to the site builder.
#[derive(Debug, Default)]
pub(crate) struct Config {
    /// Path to the configuration file.
    pub(crate) config_path: Option<PathBuf>,

    /// Directory of input files.
    pub(crate) input_dir: PathBuf,

    /// Directory of output files.
    ///
    /// If set to `None`, Vitrine does not write files.
    pub(crate) output_dir: Option<PathBuf>,

    /// Prefix for URLs.
    pub(crate) base_url: String,

    /// Directory of data files.
    ///
    /// If set to `None`, Vitrine does not search for data files.
    pub(crate) data_dir: Option<PathBuf>,

    /// Directory of layout files.
    ///
    /// If set to `None`, Vitrine does not use a layout engine.
    pub(crate) layout_dir: Option<PathBuf>,

    /// Custom filters for the layout engine.
    pub(crate) layout_filters: HashMap<String, Function>,

    /// Custom functions for the layout engine.
    pub(crate) layout_functions: HashMap<String, Function>,

    /// Custom testers for the layout engine.
    pub(crate) layout_testers: HashMap<String, Function>,

    /// HTML attributes for syntax highlight `<code>` element.
    pub(crate) syntax_highlight_code_attributes: HashMap<String, String>,

    /// HTML attributes for syntax highlight `<pre>` element.
    pub(crate) syntax_highlight_pre_attributes: HashMap<String, String>,

    /// Prefix for syntax highlight CSS classes.
    pub(crate) syntax_highlight_css_prefix: String,

    /// Formatters for syntax highlight.
    pub(crate) syntax_highlight_formatter: Option<Function>,

    /// Syntax highlight CSS stylesheets.
    pub(crate) syntax_highlight_stylesheets: Vec<SyntaxHighlightStylesheet>,
}

/// Syntax highlight CSS stylesheet configuration.
#[derive(Debug)]
pub(crate) struct SyntaxHighlightStylesheet {
    /// Prefix for class names.
    pub(crate) prefix: String,

    /// Theme name.
    ///
    /// See <https://docs.rs/syntect/latest/syntect/highlighting/struct.ThemeSet.html>
    pub(crate) theme: String,

    /// Output URL of the stylesheet.
    pub(crate) url: String,
}

/// Partial configuration.
///
/// This structure can be used for deserialization.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct PartialConfig {
    /// See [`Config::input_dir`].
    pub(crate) input_dir: Option<String>,

    /// See [`Config::output_dir`].
    pub(crate) output_dir: Option<String>,

    /// See [`Config::base_url`].
    pub(crate) base_url: Option<String>,

    /// See [`Config::data_dir`].
    pub(crate) data_dir: Option<String>,

    /// See [`Config::layout_dir`].
    pub(crate) layout_dir: Option<String>,

    /// See [`Config::layout_filters`].
    #[serde(skip)]
    pub(crate) layout_filters: Option<HashMap<String, Function>>,

    /// See [`Config::layout_functions`].
    #[serde(skip)]
    pub(crate) layout_functions: Option<HashMap<String, Function>>,

    /// See [`Config::layout_testers`].
    #[serde(skip)]
    pub(crate) layout_testers: Option<HashMap<String, Function>>,

    /// See [`Config::syntax_highlight_code_attributes`].
    pub(crate) syntax_highlight_code_attributes: Option<HashMap<String, String>>,

    /// See [`Config::syntax_highlight_pre_attributes`].
    pub(crate) syntax_highlight_pre_attributes: Option<HashMap<String, String>>,

    /// See [`Config::syntax_highlight_css_prefix`].
    pub(crate) syntax_highlight_css_prefix: Option<String>,

    /// See [`Config::syntax_highlight_formatter`].
    #[serde(skip)]
    pub(crate) syntax_highlight_formatter: Option<Function>,

    /// See [`Config::syntax_highlight_stylesheets`].
    pub(crate) syntax_highlight_stylesheets: Option<Vec<PartialSyntaxHighlightStylesheet>>,
}

// Convert `PartialConfig` to `Config`
impl Into<Config> for PartialConfig {
    fn into(self) -> Config {
        Config {
            config_path: None,
            input_dir: self
                .input_dir
                .unwrap_or_else(|| DEFAULT_INPUT_DIR.to_owned())
                .into(),
            output_dir: self
                .output_dir
                .map_or_else(|| Some(DEFAULT_OUTPUT_DIR.into()), |path| Some(path.into())),
            base_url: self
                .base_url
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned())
                .into(),
            data_dir: self.data_dir.map_or_else(
                || {
                    // Defaults to `DEFAULT_DATA_DIR`, but only if it exists
                    Some(DEFAULT_DATA_DIR)
                        .map(|dir| Path::new(dir))
                        .filter(|path| path.exists())
                        .map(|path| path.to_owned())
                },
                |v| Some(v.into()),
            ),
            layout_dir: self.layout_dir.map_or_else(
                || {
                    // Defaults to `DEFAULT_LAYOUT_DIR`, but only if it exists
                    Some(DEFAULT_LAYOUT_DIR)
                        .map(|dir| Path::new(dir))
                        .filter(|path| path.exists())
                        .map(|path| path.to_owned())
                },
                |v| Some(v.into()),
            ),
            layout_filters: self.layout_filters.unwrap_or_default(),
            layout_functions: self.layout_functions.unwrap_or_default(),
            layout_testers: self.layout_testers.unwrap_or_default(),
            syntax_highlight_code_attributes: self
                .syntax_highlight_code_attributes
                .unwrap_or_default(),
            syntax_highlight_pre_attributes: self
                .syntax_highlight_pre_attributes
                .unwrap_or_default(),
            syntax_highlight_css_prefix: self.syntax_highlight_css_prefix.unwrap_or_default(),
            syntax_highlight_formatter: self.syntax_highlight_formatter,
            syntax_highlight_stylesheets: self
                .syntax_highlight_stylesheets
                .unwrap_or_default()
                .into_iter()
                .map(|stylesheet| stylesheet.into())
                .collect(),
        }
    }
}

/// See [`SyntaxHighlightStylesheet`].
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PartialSyntaxHighlightStylesheet {
    /// See [`SyntaxHighlightStylesheet::prefix`].
    pub(crate) prefix: Option<String>,

    /// See [`SyntaxHighlightStylesheet::theme`].
    pub(crate) theme: String,

    /// See [`SyntaxHighlightStylesheet::url`].
    pub(crate) url: String,
}

// Convert `PartialSyntaxHighlightStylesheet` to `SyntaxHighlightStylesheet`
impl Into<SyntaxHighlightStylesheet> for PartialSyntaxHighlightStylesheet {
    fn into(self) -> SyntaxHighlightStylesheet {
        SyntaxHighlightStylesheet {
            prefix: self.prefix.unwrap_or_default(),
            theme: self.theme,
            url: self.url,
        }
    }
}

/// Generic function handler.
#[derive(Clone)]
pub(crate) enum Function {
    Lua(lua::Function),
    Rhai(rhai::Function),
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lua(function) => function.fmt(f),
            Self::Rhai(function) => function.fmt(f),
        }
    }
}

/// Generate a `call_N(...)` method for [`Function`].
macro_rules! impl_function_call {
    (
        $method_name:ident($($arg_name:ident: $arg_type:tt),*)
    ) => {
        pub(crate) fn $method_name<$($arg_type,)* R>(&self, $($arg_name: &$arg_type),*)
            -> anyhow::Result<R>
        where
            $(
                $arg_type: serde::Serialize + ?Sized,
            )*
            R: serde::de::DeserializeOwned,
        {
            match self {
                Self::Lua(function) => function.$method_name($($arg_name),*),
                Self::Rhai(function) => function.$method_name($($arg_name),*),
            }
        }
    }
}

impl Function {
    impl_function_call!(call_1(a1: A1));

    impl_function_call!(call_2(a1: A1, a2: A2));
}

impl<'lua> mlua::FromLua<'lua> for Function {
    fn from_lua(value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        Ok(Function::Lua(lua::Function::from_lua(value, lua)?))
    }
}

impl rhai::FromRhai for Function {
    fn from_rhai(
        value: &::rhai::Dynamic,
        engine: Arc<::rhai::Engine>,
        ast: Arc<::rhai::AST>,
    ) -> anyhow::Result<Self> {
        Ok(Function::Rhai(rhai::Function::from_rhai(
            value, engine, ast,
        )?))
    }
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

    // Load `PartialConfig` structure
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

    // Convert `PartialConfig` to `Config`
    Ok(Config {
        config_path: Some(config_path.to_owned()),
        ..config.into()
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
