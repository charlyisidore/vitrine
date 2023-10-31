//! Load configuration from Lua scripts.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use mlua::{Lua, Table};

use super::PartialConfig;

/// Load configuration from a Lua file.
pub(super) fn load_config<P>(path: P) -> anyhow::Result<PartialConfig>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    load_config_str(content)
}

/// Load configuration from a Lua string.
fn load_config_str<S>(content: S) -> anyhow::Result<PartialConfig>
where
    S: AsRef<str>,
{
    let content = content.as_ref();

    // `Lua` is not `Sync`, so we wrap it in `Arc<Mutex>`
    let lua_mutex = Arc::new(Mutex::new(Lua::new()));
    let lua = lua_mutex.lock().unwrap();

    // Save the mutex in Lua's context, we can retrieve it with `lua.app_data_ref()`
    lua.set_app_data(Arc::clone(&lua_mutex));

    let result: Table = lua.load(content).eval()?;

    Ok(PartialConfig {
        input_dir: result.get("input_dir")?,
        output_dir: result.get("output_dir")?,
        base_url: result.get("base_url")?,
        data_dir: result.get("data_dir")?,
        layout_dir: result.get("layout_dir")?,
        layout_filters: result
            .get::<_, Option<_>>("layout_filters")?
            .unwrap_or_default(),
        layout_functions: result
            .get::<_, Option<_>>("layout_functions")?
            .unwrap_or_default(),
        layout_testers: result
            .get::<_, Option<_>>("layout_testers")?
            .unwrap_or_default(),
        syntax_highlight_css_prefix: result
            .get::<_, Option<_>>("syntax_highlight_css_prefix")?
            .unwrap_or_default(),
        syntax_highlight_stylesheets: result
            .get::<_, Option<_>>("syntax_highlight_stylesheets")?
            .unwrap_or_default(),
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

        assert_eq!(config.input_dir.unwrap(), "foo");
        assert_eq!(config.output_dir.unwrap(), "bar");
        assert_eq!(config.base_url.unwrap(), "/baz");
        assert_eq!(config.data_dir.unwrap(), "_data");
        assert_eq!(config.layout_dir.unwrap(), "_layouts");
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
    fn load_config_str_empty() {
        const CONTENT: &str = "return {}";

        let config = super::load_config_str(CONTENT).unwrap();

        assert!(config.input_dir.is_none());
        assert!(config.output_dir.is_none());
        assert!(config.base_url.is_none());
        assert!(config.data_dir.is_none());
        assert!(config.layout_dir.is_none());
        assert!(config.layout_filters.is_empty(),);
        assert!(config.layout_functions.is_empty());
        assert!(config.layout_testers.is_empty());
        assert!(config.syntax_highlight_css_prefix.is_empty());
        assert!(config.syntax_highlight_stylesheets.is_empty());
    }
}
