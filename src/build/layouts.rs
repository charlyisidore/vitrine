//! Render layouts/templates.
//!
//! This module uses [`tera`] under the hood.

use std::collections::HashMap;

use tera::Tera;

use super::{Config, Entry, Error};

/// Layout engine.
pub(super) struct Engine {
    /// Name of the template variable representing the content.
    content_key: String,

    /// Name of the metadata key containing the layout name.
    layout_key: String,

    /// Name of the template variable representing the page.
    page_key: String,

    /// Tera template engine.
    tera: Tera,
}

impl Engine {
    /// Create and configure a layout engine.
    pub(super) fn new(config: &Config) -> Result<Self, Error> {
        config
            .layouts_dir
            .as_ref()
            .map(|layouts_dir| {
                let mut tera =
                    Tera::new(layouts_dir.join("**").join("*").to_str().ok_or_else(|| {
                        Error::NewLayoutEngine {
                            source: anyhow::anyhow!("Invalid layouts_dir"),
                        }
                    })?)
                    .map_err(|error| Error::NewLayoutEngine {
                        source: error.into(),
                    })?;

                for (name, filter) in config.layouts.filters.iter() {
                    let filter = filter.to_owned();
                    let filter = move |value: &tera::Value,
                                       args: &HashMap<String, tera::Value>|
                          -> tera::Result<tera::Value> {
                        filter
                            .call_2(value, args)
                            .map_err(|error| tera::Error::msg(error.to_string()))
                    };
                    tera.register_filter(name, filter);
                }

                for (name, function) in config.layouts.functions.iter() {
                    let function = function.to_owned();
                    let function =
                        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
                            function
                                .call_1(args)
                                .map_err(|error| tera::Error::msg(error.to_string()))
                        };
                    tera.register_function(name, function);
                }

                for (name, tester) in config.layouts.testers.iter() {
                    let tester = tester.to_owned();
                    let tester = move |value: Option<&tera::Value>,
                                       args: &[tera::Value]|
                          -> tera::Result<bool> {
                        tester
                            .call_2(&value, args)
                            .map_err(|error| tera::Error::msg(error.to_string()))
                    };
                    tera.register_tester(name, tester);
                }

                Ok(Self {
                    content_key: config.layouts.content_key.to_owned(),
                    layout_key: config.layouts.layout_key.to_owned(),
                    page_key: config.layouts.page_key.to_owned(),
                    tera,
                })
            })
            .unwrap_or_else(|| {
                Err(Error::NewLayoutEngine {
                    source: anyhow::anyhow!("Missing layouts_dir"),
                })
            })
    }

    /// Render the layout of a [`Entry`].
    ///
    /// This function extracts the `layout` property from the metadata to
    /// determine the layout file. The metadata fields and the content are
    /// merged into a single context for the layout engine. The rendered output
    /// replaces the `content` property in the build entry.
    pub(super) fn render_entry(
        &self,
        entry: Entry,
        provided_data: &serde_json::Value,
    ) -> Result<Entry, Error> {
        // Get metadata
        let mut data = if let Some(entry_data) = entry.data.as_ref() {
            // Merge supplied and entry metadata
            crate::util::data::shallow_merge(provided_data, entry_data).map_err(|error| {
                Error::RenderLayout {
                    input_path: entry.input_path_buf(),
                    layout: None,
                    source: error,
                }
            })?
        } else {
            // No entry metadata, use supplied metadata only
            provided_data.to_owned()
        };

        // The metadata must have a layout property
        let Some(layout) = &data
            .get(&self.layout_key)
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_owned())
        else {
            return Ok(entry);
        };

        if !self.content_key.is_empty() {
            // Add content to the metadata
            if let Some(content) = entry.content.as_ref() {
                data.as_object_mut()
                    .map(|map| map.insert(self.content_key.to_owned(), content.to_owned().into()));
            }
        }

        if !self.page_key.is_empty() {
            // Add page data
            data.as_object_mut().map(|map| {
                map.insert(
                    self.page_key.to_owned(),
                    tera::Map::from_iter([("url".to_owned(), entry.url.to_owned().into())]).into(),
                )
            });
        }

        let content = self
            .render(layout, data)
            .map_err(|error| Error::RenderLayout {
                input_path: entry.input_path_buf(),
                layout: Some(layout.to_owned()),
                source: error,
            })?;

        Ok(Entry {
            content: Some(content),
            ..entry
        })
    }

    /// Render a layout given data.
    fn render<L, D>(&self, layout: L, data: D) -> anyhow::Result<String>
    where
        L: AsRef<str>,
        D: serde::Serialize,
    {
        let layout = layout.as_ref();

        let context = tera::Context::from_serialize(&data)?;

        let output = self.tera.render(&layout, &context)?;

        Ok(output)
    }
}
