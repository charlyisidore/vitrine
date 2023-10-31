//! Render layouts/templates.
//!
//! This module uses [`tera`] under the hood.

use tera::Tera;

use super::{Config, Entry, Error};

/// Layout engine.
pub(super) struct Engine {
    /// Tera template engine.
    tera: Tera,
}

impl Engine {
    /// Create and configure a layout engine.
    pub(super) fn new(config: &Config) -> Result<Self, Error> {
        config
            .layout_dir
            .as_ref()
            .map(|layout_dir| {
                let mut tera =
                    Tera::new(layout_dir.join("**").join("*").to_str().ok_or_else(|| {
                        Error::NewLayoutEngine {
                            source: anyhow::anyhow!("Invalid layout_dir"),
                        }
                    })?)
                    .map_err(|error| Error::NewLayoutEngine {
                        source: error.into(),
                    })?;

                // Since [`tera`]` requires `'static` lifetime from filters/functions/testers,
                // we cannot use functions created at runtime. Therefore, we use
                // `static_lifetime` as a workaround.
                // See <https://github.com/Keats/tera/issues/767>
                for (name, filter) in config.layout_filters.iter() {
                    tera.register_filter(name, unsafe {
                        crate::util::r#unsafe::static_lifetime(&filter.0)
                    });
                }

                for (name, function) in config.layout_functions.iter() {
                    tera.register_function(name, unsafe {
                        crate::util::r#unsafe::static_lifetime(&function.0)
                    });
                }

                for (name, tester) in config.layout_testers.iter() {
                    tera.register_tester(name, unsafe {
                        crate::util::r#unsafe::static_lifetime(&tester.0)
                    });
                }

                Ok(Self { tera })
            })
            .unwrap_or_else(|| {
                Err(Error::NewLayoutEngine {
                    source: anyhow::anyhow!("Missing layout_dir"),
                })
            })
    }

    /// Render the layout of a [`Entry`].
    ///
    /// This function extracts the `layout` property from the metadata to
    /// determine the layout file. The metadata fields and the content are
    /// merged into a single context for the layout engine. The rendered output
    /// replaces the `content` property in the build entry.
    pub(super) fn render_entry(&self, entry: Entry) -> Result<Entry, Error> {
        // The entry must have content and metadata
        if let (Some(content), Some(data)) = (entry.content.as_ref(), entry.data.as_ref()) {
            // The metadata must have a layout property
            if let Some(layout) = data.layout.as_ref().filter(|v| !v.is_empty()) {
                let content =
                    self.render(layout, content, data)
                        .map_err(|error| Error::RenderLayout {
                            input_path: entry.input_path_buf(),
                            layout: layout.to_owned(),
                            source: error,
                        })?;

                return Ok(Entry {
                    content: Some(content),
                    ..entry
                });
            }
        }

        Ok(entry)
    }

    /// Render a layout given content and metadata.
    fn render<L, S, C>(&self, layout: L, content: S, data: C) -> Result<String, anyhow::Error>
    where
        L: AsRef<str>,
        S: AsRef<str>,
        C: serde::Serialize,
    {
        let layout = layout.as_ref();
        let content = content.as_ref();

        let mut context = tera::Context::from_serialize(&data)?;

        context.insert("content", content);

        let output = self.tera.render(&layout, &context)?;

        Ok(output)
    }
}
