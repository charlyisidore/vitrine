//! Load configuration from Lua scripts.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use mlua::LuaSerdeExt;

use super::{Error, LayoutFilterFn, LayoutFunctionFn, LayoutTesterFn, PartialConfig};

/// Load configuration from a Lua file.
pub(super) fn load_config<P>(path: P) -> anyhow::Result<PartialConfig>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let content = std::fs::read_to_string(path).map_err(|error| Error::LoadConfig {
        config_path: Some(path.to_owned()),
        source: error.into(),
    })?;

    load_config_str(content)
}

/// Load configuration from a Lua string.
fn load_config_str<S>(content: S) -> anyhow::Result<PartialConfig>
where
    S: AsRef<str>,
{
    let content = content.as_ref();

    // Initialize the lua engine
    // Since `mlua::Lua` is not `Sync`, we wrap it in `Mutex`
    let engine = Arc::new(Mutex::new(mlua::Lua::new()));
    let lua = engine.lock().unwrap();

    // Execute the script
    let config: mlua::Table = lua.load(content).eval()?;

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
    let layout_filters =
        config
            .get::<_, Option<mlua::Table>>("layout_filters")?
            .map_or_else(
                || Ok(HashMap::new()),
                |table| {
                    table
                        .pairs::<String, mlua::Function>()
                        .map(
                            |pair| -> Result<(String, Box<dyn LayoutFilterFn>), anyhow::Error> {
                                let (key, lua_fn) = pair?;

                                // Since `mlua::Function` is not `Sync`, we store it in the lua
                                // registry.
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
                            },
                        )
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
                    .map(
                        |pair| -> Result<(String, Box<dyn LayoutFunctionFn>), anyhow::Error> {
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
                        },
                    )
                    .collect()
            },
        )?;

    // Testers for the layout engine
    let layout_testers =
        config
            .get::<_, Option<mlua::Table>>("layout_testers")?
            .map_or_else(
                || Ok(HashMap::new()),
                |table| {
                    table
                        .pairs::<String, mlua::Function>()
                        .map(
                            |pair| -> Result<(String, Box<dyn LayoutTesterFn>), anyhow::Error> {
                                let (key, lua_fn) = pair?;

                                // Since `mlua::Function` is not `Sync`, we store it in the lua
                                // registry.
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
                            },
                        )
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

#[cfg(test)]
mod tests {
    #[test]
    fn load_config_str() {
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
                odd = function(value, args) return value % 2 == 1 end,
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

        let config = super::load_config_str(CONTENT).unwrap();

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
}
