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

/// Default file names for configuration files
pub(super) const DEFAULT_CONFIG_FILE_NAMES: [&str; 5] = [
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
    pub(crate) layout_filters: HashMap<String, Box<dyn LayoutFilterFn>>,

    /// Custom functions for the layout engine.
    pub(crate) layout_functions: HashMap<String, Box<dyn LayoutFunctionFn>>,

    /// Custom testers for the layout engine.
    pub(crate) layout_testers: HashMap<String, Box<dyn LayoutTesterFn>>,

    /// Prefix for syntax highlight CSS classes.
    pub(crate) syntax_highlight_css_prefix: String,

    /// Syntax highlight CSS stylesheets.
    pub(crate) syntax_highlight_stylesheets: Vec<SyntaxHighlightStylesheet>,
}

/// Partial configuration.
///
/// This structure can be used for deserialization.
#[derive(Debug, Default, Deserialize)]
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
    pub(crate) layout_filters: HashMap<String, Box<dyn LayoutFilterFn>>,

    /// See [`Config::layout_functions`].
    #[serde(skip)]
    pub(crate) layout_functions: HashMap<String, Box<dyn LayoutFunctionFn>>,

    /// See [`Config::layout_testers`].
    #[serde(skip)]
    pub(crate) layout_testers: HashMap<String, Box<dyn LayoutTesterFn>>,

    /// See [`Config::syntax_highlight_css_prefix`].
    #[serde(default)]
    pub(crate) syntax_highlight_css_prefix: String,

    /// See [`Config::syntax_highlight_stylesheets`].
    #[serde(default)]
    pub(crate) syntax_highlight_stylesheets: Vec<SyntaxHighlightStylesheet>,
}

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

impl<'lua> mlua::FromLua<'lua> for SyntaxHighlightStylesheet {
    fn from_lua(value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        use mlua::LuaSerdeExt;
        lua.from_value(value)
    }
}

/// Generate [`Fn`] traits for layout engine filters/functions/testers.
///
/// This macro automatically implements [`Debug`] and [`mlua::FromLua`] for the
/// generated trait.
macro_rules! create_layout_fn_trait {
    ($(#[$($attrs:tt)*])* $name:ident, ($($arg_name:ident: $arg_type:ty),*) -> $ret_type:ty) => {
        $(#[$($attrs)*])*
        pub(crate) trait $name: Fn($($arg_type),*) -> $ret_type + Send + Sync {}

        impl<F> $name for F where F: Fn($($arg_type),*) -> $ret_type + Send + Sync {}

        impl std::fmt::Debug for dyn $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, stringify!($name))
            }
        }

        impl<'lua> mlua::FromLua<'lua> for Box<dyn $name> {
            fn from_lua(value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
                use mlua::LuaSerdeExt;
                use std::sync::{Arc, Mutex};

                let function = value
                    .as_function()
                    .ok_or_else(|| mlua::Error::external("expected a function"))?
                    .to_owned();

                let function_key = lua.create_registry_value(function)?;

                let lua_mutex = unsafe {
                    crate::util::r#unsafe::static_lifetime(
                        lua.app_data_ref::<Arc<Mutex<mlua::Lua>>>()
                            .ok_or_else(|| mlua::Error::external("missing lua app data"))?
                            .as_ref(),
                    )
                };

                Ok(Box::new(
                    move |$($arg_name: $arg_type),*| -> $ret_type {
                        let lua = lua_mutex
                            .lock()
                            .map_err(|error| tera::Error::msg(error.to_string()))?;

                        $(
                            let $arg_name = lua.to_value(&$arg_name)
                            .map_err(|error| tera::Error::msg(error.to_string()))?;
                        )*

                        let function: mlua::Function = lua.registry_value(&function_key)
                            .map_err(|error| tera::Error::msg(error.to_string()))?;

                        let result = function.call::<_, mlua::Value>(($($arg_name),*))
                            .map_err(|error| tera::Error::msg(error.to_string()))?;

                        let result = lua.from_value(result)
                            .map_err(|error| tera::Error::msg(error.to_string()))?;

                        Ok(result)
                    },
                ))
            }
        }
    };
}

create_layout_fn_trait!(
    /// Filter for the layout engine.
    LayoutFilterFn,
    (value: &tera::Value, args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value>
);

create_layout_fn_trait!(
    /// Function for the layout engine.
    LayoutFunctionFn,
    (args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value>
);

create_layout_fn_trait!(
    /// Tester for the layout engine.
    LayoutTesterFn,
    (value: Option<&tera::Value>, args: &[tera::Value]) -> tera::Result<bool>
);

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
        input_dir: config
            .input_dir
            .unwrap_or_else(|| DEFAULT_INPUT_DIR.to_owned())
            .into(),
        output_dir: config
            .output_dir
            .map_or_else(|| Some(DEFAULT_OUTPUT_DIR.into()), |path| Some(path.into())),
        base_url: config
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned())
            .into(),
        data_dir: config.data_dir.map_or_else(
            || {
                // Defaults to `DEFAULT_DATA_DIR`, but only if it exists
                Some(DEFAULT_DATA_DIR)
                    .map(|dir| Path::new(dir))
                    .filter(|path| path.exists())
                    .map(|path| path.to_owned())
            },
            |v| Some(v.into()),
        ),
        layout_dir: config.layout_dir.map_or_else(
            || {
                // Defaults to `DEFAULT_LAYOUT_DIR`, but only if it exists
                Some(DEFAULT_LAYOUT_DIR)
                    .map(|dir| Path::new(dir))
                    .filter(|path| path.exists())
                    .map(|path| path.to_owned())
            },
            |v| Some(v.into()),
        ),
        layout_filters: config.layout_filters,
        layout_functions: config.layout_functions,
        layout_testers: config.layout_testers,
        syntax_highlight_css_prefix: config.syntax_highlight_css_prefix,
        syntax_highlight_stylesheets: config.syntax_highlight_stylesheets,
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
