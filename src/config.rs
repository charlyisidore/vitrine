//! Configuration.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use mlua::LuaSerdeExt;
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

/// Load the configuration from a file.
///
/// This function executes the script contained in the file and returns the
/// result as a `PartialConfig` object.
pub(super) fn load_config<P>(config_path: P) -> Result<PartialConfig, Error>
where
    P: AsRef<Path>,
{
    let config_path = config_path.as_ref();

    tracing::info!("Loading configuration from {:?}", config_path);

    let config = if let Some(extension) = config_path.extension().and_then(|v| v.to_str()) {
        match extension {
            "json" | "lua" | "rhai" | "toml" | "yaml" => {
                let content =
                    std::fs::read_to_string(config_path).map_err(|error| Error::LoadConfig {
                        config_path: Some(config_path.to_owned()),
                        source: error.into(),
                    })?;

                match extension {
                    "json" => load_config_json(content),
                    "lua" => load_config_lua(content),
                    "rhai" => load_config_rhai(content),
                    "toml" => load_config_toml(content),
                    "yaml" => load_config_yaml(content),
                    _ => unreachable!(),
                }
            },
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

/// Load the configuration from a `json` file.
fn load_config_json<S>(content: S) -> Result<PartialConfig, anyhow::Error>
where
    S: AsRef<str>,
{
    let content = content.as_ref();

    let config = serde_json::from_str(content)?;

    Ok(config)
}

/// Load the configuration from a `lua` script.
fn load_config_lua<S>(script: S) -> Result<PartialConfig, anyhow::Error>
where
    S: AsRef<str>,
{
    let script = script.as_ref();

    // Initialize the lua engine
    // Since `mlua::Lua` is not `Sync`, we wrap it in `Mutex`
    let engine = Arc::new(Mutex::new(mlua::Lua::new()));
    let lua = engine.lock().unwrap();

    // Execute the script
    let config: mlua::Table = lua.load(script).eval()?;

    // Input directory
    let input_dir = config
        .get::<_, Option<String>>("input_dir")?
        .map(|v| v.into());

    // Output directory
    let output_dir = config
        .get::<_, Option<String>>("output_dir")?
        .map(|v| v.into());

    // Base URL
    let base_url = config
        .get::<_, Option<String>>("base_url")?
        .map(|v| v.into());

    // Data directory
    let data_dir = config
        .get::<_, Option<String>>("data_dir")?
        .map(|v| v.into());

    // Layout directory
    let layout_dir = config
        .get::<_, Option<String>>("layout_dir")?
        .map(|v| v.into());

    // Filters for the layout engine
    let layout_filters = config
        .get::<_, Option<mlua::Table>>("layout_filters")?
        .map_or_else(
            || Ok(HashMap::new()),
            |table| {
                table
                    .pairs::<String, mlua::Function>()
                    .map(|pair| -> Result<(String, LayoutFilter), anyhow::Error> {
                        let (key, lua_fn) = pair?;

                        // Since `mlua::Function` is not `Sync`, we store it in the lua registry
                        // See <https://github.com/khvzak/mlua/issues/233#issuecomment-1353831597>
                        let lua_fn_key = lua.create_registry_value(lua_fn)?;

                        // Clone reference of lua context for use in closure
                        let engine = Arc::clone(&engine);

                        let layout_filter =
                            move |value: &tera::Value,
                                  args: &HashMap<String, tera::Value>|
                                  -> tera::Result<tera::Value> {
                                // Wrap closure to avoid repeating `.map_err()`
                                (|| -> mlua::Result<_> {
                                    let lua = engine.lock().unwrap();

                                    // Convert arguments from tera to lua types
                                    let value = lua.to_value(&value)?;
                                    let args = lua.to_value(&args)?;

                                    // Retrieve and call lua function
                                    let lua_fn: mlua::Function = lua.registry_value(&lua_fn_key)?;
                                    let result = lua_fn.call::<_, mlua::Value>((value, args))?;

                                    // Convert result from lua to tera types
                                    let result = lua.from_value(result)?;

                                    Ok(result)
                                })()
                                .map_err(|error| error.to_string().into())
                            };

                        Ok((key, Box::new(layout_filter)))
                    })
                    .collect()
            },
        )?;

    // Functions for the layout engine
    let layout_functions = config
        .get::<_, Option<mlua::Table>>("layout_functions")?
        .map_or_else(
            || Ok(HashMap::new()),
            |table| {
                table
                    .pairs::<String, mlua::Function>()
                    .map(|pair| -> Result<(String, LayoutFunction), anyhow::Error> {
                        let (key, lua_fn) = pair?;

                        // Since `mlua::Function` is not `Sync`, we store it in the lua registry
                        // See <https://github.com/khvzak/mlua/issues/233#issuecomment-1353831597>
                        let lua_fn_key = lua.create_registry_value(lua_fn)?;

                        // Clone reference of lua context for use in closure
                        let engine = Arc::clone(&engine);

                        let layout_function =
                            move |args: &HashMap<String, tera::Value>|
                                -> tera::Result<tera::Value> {
                                // Wrap closure to avoid repeating `.map_err()`
                                (|| -> mlua::Result<_> {
                                    let lua = engine.lock().unwrap();

                                    // Convert arguments from tera to lua types
                                    let args = lua.to_value(&args)?;

                                    // Retrieve and call lua function
                                    let lua_fn: mlua::Function = lua.registry_value(&lua_fn_key)?;
                                    let result = lua_fn.call::<_, mlua::Value>((args,))?;

                                    // Convert result from lua to tera types
                                    let result = lua.from_value(result)?;

                                    Ok(result)
                                })()
                                .map_err(|error| error.to_string().into())
                            };

                        Ok((key, Box::new(layout_function)))
                    })
                    .collect()
            },
        )?;

    // Testers for the layout engine
    let layout_testers = config
        .get::<_, Option<mlua::Table>>("layout_testers")?
        .map_or_else(
            || Ok(HashMap::new()),
            |table| {
                table
                    .pairs::<String, mlua::Function>()
                    .map(|pair| -> Result<(String, LayoutTester), anyhow::Error> {
                        let (key, lua_fn) = pair?;

                        // Since `mlua::Function` is not `Sync`, we store it in the lua registry
                        // See <https://github.com/khvzak/mlua/issues/233#issuecomment-1353831597>
                        let lua_fn_key = lua.create_registry_value(lua_fn)?;

                        // Clone reference of lua context for use in closure
                        let engine = Arc::clone(&engine);

                        let layout_tester = move |value: Option<&tera::Value>,
                                                  args: &[tera::Value]|
                              -> tera::Result<bool> {
                            // Wrap closure to avoid repeating `.map_err()`
                            (|| -> mlua::Result<_> {
                                let lua = engine.lock().unwrap();

                                // Convert arguments from tera to lua types
                                let value = lua.to_value(&value)?;
                                let args = lua.to_value(&args)?;

                                // Retrieve and call lua function
                                let lua_fn: mlua::Function = lua.registry_value(&lua_fn_key)?;
                                let result = lua_fn.call::<_, mlua::Value>((value, args))?;

                                // Convert result from lua to tera types
                                let result = lua.from_value(result)?;

                                Ok(result)
                            })()
                            .map_err(|error| error.to_string().into())
                        };

                        Ok((key, Box::new(layout_tester)))
                    })
                    .collect()
            },
        )?;

    // Prefix for syntax highlight CSS classes
    let syntax_highlight_css_prefix = config
        .get::<_, Option<String>>("syntax_highlight_css_prefix")?
        .unwrap_or_default();

    // Syntax highlight CSS stylesheets
    let syntax_highlight_stylesheets = config
        .get::<_, Option<mlua::Value>>("syntax_highlight_stylesheets")?
        .map(|value| lua.from_value(value))
        .transpose()?
        .unwrap_or_default();

    Ok(PartialConfig {
        input_dir,
        output_dir,
        base_url,
        data_dir,
        layout_dir,
        layout_filters,
        layout_functions,
        layout_testers,
        syntax_highlight_css_prefix,
        syntax_highlight_stylesheets,
        ..Default::default()
    })
}

/// Load the configuration from a `rhai` script.
fn load_config_rhai<S>(script: S) -> Result<PartialConfig, anyhow::Error>
where
    S: AsRef<str>,
{
    let script = script.as_ref();

    // Initialize the rhai engine
    let engine = Arc::new(rhai::Engine::new());

    // Compile the script
    let ast = Arc::new(engine.compile(script)?);

    // Execute the script
    let config = engine
        .eval_ast::<rhai::Dynamic>(&ast)?
        .try_cast::<rhai::Map>()
        .ok_or_else(|| anyhow::anyhow!("The configuration script must return an object"))?;

    // Input directory
    let input_dir = config
        .get("input_dir")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Output directory
    let output_dir = config
        .get("output_dir")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Base URL
    let base_url = config
        .get("base_url")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Data directory
    let data_dir = config
        .get("data_dir")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Layout directory
    let layout_dir = config
        .get("layout_dir")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|v| v.into());

    // Filters for the layout engine
    let layout_filters = config
        .get("layout_filters")
        .and_then(|v| v.to_owned().try_cast::<rhai::Map>())
        .map_or_else(
            || Ok(HashMap::new()),
            |map| -> Result<_, _> {
                map.iter()
                    .map(
                        |(key, value)| -> Result<(String, LayoutFilter), anyhow::Error> {
                            let key = key.to_string();
                            let rhai_fn =
                                value.to_owned().try_cast::<rhai::FnPtr>().ok_or_else(|| {
                                    anyhow::anyhow!("layout_filters must be an object")
                                })?;

                            // Clone references of rhai context for use in closure
                            let engine = Arc::clone(&engine);
                            let ast = Arc::clone(&ast);

                            let layout_filter =
                                move |value: &tera::Value,
                                      args: &HashMap<String, tera::Value>|
                                      -> tera::Result<tera::Value> {
                                    // Wrap closure to avoid repeating `.map_err()`
                                    (|| -> Result<_, Box<rhai::EvalAltResult>> {
                                        // Convert arguments from tera to rhai types
                                        let value = rhai::serde::to_dynamic(value)?.to_owned();
                                        let args = rhai::serde::to_dynamic(args)?.to_owned();

                                        // Call rhai function
                                        let result = rhai_fn.call::<rhai::Dynamic>(
                                            &engine,
                                            &ast,
                                            (value, args),
                                        )?;

                                        // Convert result from rhai to tera types
                                        let result =
                                            rhai::serde::from_dynamic::<tera::Value>(&result)?;

                                        Ok(result)
                                    })()
                                    .map_err(|error| error.to_string().into())
                                };

                            Ok((key, Box::new(layout_filter)))
                        },
                    )
                    .collect()
            },
        )?;

    // Functions for the layout engine
    let layout_functions = config
        .get("layout_functions")
        .and_then(|v| v.to_owned().try_cast::<rhai::Map>())
        .map_or_else(
            || Ok(HashMap::new()),
            |map| -> Result<_, _> {
                map.iter()
                    .map(
                        |(key, value)| -> Result<(String, LayoutFunction), anyhow::Error> {
                            let key = key.to_string();
                            let rhai_fn =
                                value.to_owned().try_cast::<rhai::FnPtr>().ok_or_else(|| {
                                    anyhow::anyhow!("layout_functions must be an object")
                                })?;

                            // Clone references of rhai context for use in closure
                            let engine = Arc::clone(&engine);
                            let ast = Arc::clone(&ast);

                            let layout_function =
                                move |args: &HashMap<String, tera::Value>|
                                      -> tera::Result<tera::Value> {
                                    // Wrap closure to avoid repeating `.map_err()`
                                    (|| -> Result<_, Box<rhai::EvalAltResult>> {
                                        // Convert arguments from tera to rhai types
                                        let args = rhai::serde::to_dynamic(args)?.to_owned();

                                        // Call rhai function
                                        let result = rhai_fn
                                            .call::<rhai::Dynamic>(&engine, &ast, (args,))?;

                                        // Convert result from rhai to tera types
                                        let result =
                                            rhai::serde::from_dynamic::<tera::Value>(&result)?;

                                        Ok(result)
                                    })()
                                    .map_err(|error| error.to_string().into())
                                };

                            Ok((key, Box::new(layout_function)))
                        },
                    )
                    .collect()
            },
        )?;

    // Testers for the layout engine
    let layout_testers = config
        .get("layout_tests")
        .and_then(|v| v.to_owned().try_cast::<rhai::Map>())
        .map_or_else(
            || Ok(HashMap::new()),
            |map| -> Result<_, _> {
                map.iter()
                    .map(
                        |(key, value)| -> Result<(String, LayoutTester), anyhow::Error> {
                            let key = key.to_string();
                            let rhai_fn =
                                value.to_owned().try_cast::<rhai::FnPtr>().ok_or_else(|| {
                                    anyhow::anyhow!("layout_testers must be an object")
                                })?;

                            // Clone references of rhai context for use in closure
                            let engine = Arc::clone(&engine);
                            let ast = Arc::clone(&ast);

                            let layout_tester =
                                move |value: Option<&tera::Value>,
                                      args: &[tera::Value]|
                                      -> tera::Result<bool> {
                                    // Wrap closure to avoid repeating `.map_err()`
                                    (|| -> Result<_, Box<rhai::EvalAltResult>> {
                                        // Convert arguments from tera to rhai types
                                        let value = rhai::serde::to_dynamic(value)?.to_owned();
                                        let args = rhai::serde::to_dynamic(args)?.to_owned();

                                        // Call rhai function
                                        let result = rhai_fn.call::<rhai::Dynamic>(
                                            &engine,
                                            &ast,
                                            (value, args),
                                        )?;

                                        // Convert result from rhai to tera types
                                        let result = rhai::serde::from_dynamic::<bool>(&result)?;

                                        Ok(result)
                                    })()
                                    .map_err(|error| error.to_string().into())
                                };

                            Ok((key, Box::new(layout_tester)))
                        },
                    )
                    .collect()
            },
        )?;

    // Prefix for syntax highlight CSS classes
    let syntax_highlight_css_prefix = config
        .get("syntax_highlight_css_prefix")
        .map(|v| v.to_owned().into_string())
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?
        .unwrap_or_default();

    // Syntax highlight CSS stylesheets
    let syntax_highlight_stylesheets = config
        .get("syntax_highlight_stylesheets")
        .and_then(|v| v.to_owned().try_cast::<rhai::Array>())
        .map_or_else(
            || Ok(Vec::new()),
            |array| {
                array
                    .iter()
                    .map(|v| {
                        v.to_owned()
                            .try_cast::<rhai::Map>()
                            .map(|v| {
                                Ok::<_, &str>(SyntaxHighlightStylesheet {
                                    prefix: v
                                        .get("prefix")
                                        .map(|v| v.to_owned().into_string())
                                        .transpose()?
                                        .unwrap_or_default(),
                                    theme: v
                                        .get("theme")
                                        .ok_or_else(|| {
                                            "Missing theme in syntax_highlight_stylesheets"
                                        })?
                                        .to_owned()
                                        .into_string()?,
                                    url: v
                                        .get("url")
                                        .ok_or_else(|| {
                                            "Missing url in syntax_highlight_stylesheets"
                                        })?
                                        .to_owned()
                                        .into_string()?,
                                })
                            })
                            .transpose()
                            .map_err(|error| anyhow::anyhow!(error))?
                            .ok_or_else(|| {
                                anyhow::anyhow!("Cannot parse syntax_highlight_stylesheets")
                            })
                    })
                    .collect()
            },
        )?;

    Ok(PartialConfig {
        input_dir,
        output_dir,
        base_url,
        data_dir,
        layout_dir,
        layout_filters,
        layout_functions,
        layout_testers,
        syntax_highlight_css_prefix,
        syntax_highlight_stylesheets,
        ..Default::default()
    })
}

/// Load the configuration from a `toml` file.
fn load_config_toml<S>(content: S) -> Result<PartialConfig, anyhow::Error>
where
    S: AsRef<str>,
{
    let content = content.as_ref();

    let config = toml::from_str(content)?;

    Ok(config)
}

/// Load the configuration from a `yaml` file.
fn load_config_yaml<S>(content: S) -> Result<PartialConfig, anyhow::Error>
where
    S: AsRef<str>,
{
    let content = content.as_ref();

    let config = serde_yaml::from_str(content)?;

    Ok(config)
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

#[cfg(test)]
mod tests {
    #[test]
    fn load_config_json() {
        const CONTENT: &str = r#"
        {
            "input_dir": "foo",
            "output_dir": "bar",
            "base_url": "/baz",
            "data_dir": "_data",
            "layout_dir": "_layouts"
        }
        "#;

        let config = super::load_config_json(CONTENT).unwrap();

        assert_eq!(config.input_dir.unwrap().to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap().to_str().unwrap(), "_layouts");
    }

    #[test]
    fn load_config_lua() {
        const CONTENT: &str = r#"
        return {
            input_dir = "foo",
            output_dir = "bar",
            base_url = "/baz",
            data_dir = "_data",
            layout_dir = "_layouts",
            layout_filters = {
                upper = function(value, args) return string.upper(value) end,
            },
            layout_functions = {
                min = function(args) return math.min(table.unpack(args.values)) end,
            },
            layout_testers = {
                odd = function(value) return value % 2 == 1 end,
            },
            syntax_highlight_css_prefix = "highlight-",
            syntax_highlight_stylesheets = {
                {
                    prefix = "highlight-",
                    theme = "base16-ocean.dark",
                    url = "/highlight.css",
                },
            },
        }
        "#;

        let config = super::load_config_lua(CONTENT).unwrap();

        assert_eq!(config.input_dir.unwrap().to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap().to_str().unwrap(), "_layouts");
        assert_eq!(config.layout_filters.len(), 1);
        assert!(config.layout_filters.contains_key("upper"));
        assert_eq!(config.layout_functions.len(), 1);
        assert!(config.layout_functions.contains_key("min"));
        assert_eq!(config.layout_testers.len(), 1);
        assert!(config.layout_testers.contains_key("odd"));
        assert_eq!(config.syntax_highlight_css_prefix, "highlight-");
        assert_eq!(config.syntax_highlight_stylesheets.len(), 1);
        let stylesheet = config.syntax_highlight_stylesheets.get(0).unwrap();
        assert_eq!(stylesheet.prefix, "highlight-");
        assert_eq!(stylesheet.theme, "base16-ocean.dark");
        assert_eq!(stylesheet.url, "/highlight.css");
    }

    #[test]
    fn load_config_rhai() {
        const CONTENT: &str = r#"
        #{
            input_dir: "foo",
            output_dir: "bar",
            base_url: "/baz",
            data_dir: "_data",
            layout_dir: "_layouts",
            layout_filters: #{
                upper: |value, args| value.to_upper(),
            },
            layout_functions: #{
                min: |args| args.values?.reduce(|a, b| min(a, b), 0xffffffff),
            },
            layout_testers: #{
                odd: |value| value % 2 == 1,
            },
            syntax_highlight_css_prefix: "highlight-",
            syntax_highlight_stylesheets: [
                #{
                    prefix: "highlight-",
                    theme: "base16-ocean.dark",
                    url: "/highlight.css",
                },
            ],
        }
        "#;

        let config = super::load_config_rhai(CONTENT).unwrap();

        assert_eq!(config.input_dir.unwrap().to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap().to_str().unwrap(), "_layouts");
        assert_eq!(config.layout_filters.len(), 1);
        assert!(config.layout_filters.contains_key("upper"));
        assert_eq!(config.layout_functions.len(), 1);
        assert!(config.layout_functions.contains_key("min"));
        assert_eq!(config.layout_testers.len(), 1);
        assert!(config.layout_testers.contains_key("odd"));
        assert_eq!(config.syntax_highlight_css_prefix, "highlight-");
        assert_eq!(config.syntax_highlight_stylesheets.len(), 1);
        let stylesheet = config.syntax_highlight_stylesheets.get(0).unwrap();
        assert_eq!(stylesheet.prefix, "highlight-");
        assert_eq!(stylesheet.theme, "base16-ocean.dark");
        assert_eq!(stylesheet.url, "/highlight.css");
    }

    #[test]
    fn load_config_toml() {
        const CONTENT: &str = r#"
            input_dir = "foo"
            output_dir = "bar"
            base_url = "/baz"
            data_dir = "_data"
            layout_dir = "_layouts"
        "#;

        let config = super::load_config_toml(CONTENT).unwrap();

        assert_eq!(config.input_dir.unwrap().to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap().to_str().unwrap(), "_layouts");
    }

    #[test]
    fn load_config_yaml() {
        const CONTENT: &str = r#"
            input_dir: foo
            output_dir: bar
            base_url: /baz
            data_dir: _data
            layout_dir: _layouts
        "#;

        let config = super::load_config_yaml(CONTENT).unwrap();

        assert_eq!(config.input_dir.unwrap().to_str().unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap().to_str().unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
        assert_eq!(config.data_dir.unwrap().to_str().unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap().to_str().unwrap(), "_layouts");
    }
}
