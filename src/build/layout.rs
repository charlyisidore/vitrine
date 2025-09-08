//! Render layouts.
//!
//! This module uses [`minijinja`] under the hood.

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use anyhow::{Context as _, Result};
use async_channel::{Receiver, Sender};
use minijinja::{Environment, path_loader, value::Rest};
use serde::{Serialize, de::Error};

use crate::{
    Config, DateTime, Page, ReceiverExt, Site, TaxonomyItem, UriRelativeString, Value, to_value,
};

/// Context given to layouts.
#[derive(Debug, Serialize)]
struct Context<'a> {
    content: &'a String,
    date: &'a DateTime,
    languages: &'a BTreeMap<String, UriRelativeString>,
    page: &'a Value,
    site: &'a Value,
    taxonomies: &'a BTreeMap<String, BTreeMap<String, Vec<TaxonomyItem>>>,
    url: &'a UriRelativeString,
    vitrine: Vitrine<'a>,
}

#[derive(Debug, Serialize)]
struct Vitrine<'a> {
    generator: &'a str,
    version: &'a str,
}

/// Render layouts.
pub fn run(
    config: &Config,
    site: Arc<RwLock<Site>>,
    page_rx: Receiver<Page>,
    page_tx: Sender<Page>,
) -> Result<()> {
    use minijinja::{Error, Value};

    if let Some(render) = &config.layout_render {
        for page in page_rx.into_iter() {
            let Some(layout) = page.data.get("layout").and_then(|v| v.as_str()) else {
                page_tx.send_blocking(page)?;
                continue;
            };

            let content = {
                let site = site.read().unwrap();

                let context = Context {
                    content: &page.content,
                    date: &page.date,
                    languages: &page.languages,
                    page: &page.data,
                    site: &site.data,
                    taxonomies: &site.taxonomies,
                    url: &page.url,
                    vitrine: Vitrine {
                        generator: concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION")),
                        version: env!("CARGO_PKG_VERSION"),
                    },
                };
                let context = serde_json::to_value(context)?;

                (render)(layout.to_string(), context)?
            };

            page_tx.send_blocking(Page { content, ..page })?;
        }
    } else {
        let Some(layout_dir) = &config.layout_dir else {
            for page in page_rx.into_iter() {
                page_tx.send_blocking(page)?;
            }
            return Ok(());
        };

        let mut env = Environment::new();
        env.set_loader(path_loader(layout_dir.canonicalize()?));

        minijinja_contrib::add_to_environment(&mut env);

        for (name, f) in &config.layout_filters {
            let f = f.clone();
            env.add_filter(name, move |value: Value, args: Rest<Value>| {
                (|| {
                    let value = to_value(value)?;
                    let args = args.iter().map(to_value).collect::<Result<_, _>>()?;
                    (f)(value, args)
                })()
                .map(Value::from_serialize)
                .map_err(Error::custom)
            });
        }

        for (name, f) in &config.layout_functions {
            let f = f.clone();
            env.add_function(name, move |args: Rest<Value>| {
                (|| {
                    let args = args.iter().map(to_value).collect::<Result<_, _>>()?;
                    (f)(args)
                })()
                .map(Value::from_serialize)
                .map_err(Error::custom)
            });
        }

        for (name, f) in &config.layout_tests {
            let f = f.clone();
            env.add_test(name, move |value: Value, args: Rest<Value>| {
                (|| {
                    let value = to_value(value)?;
                    let args = args.iter().map(to_value).collect::<Result<_, _>>()?;
                    (f)(value, args)
                })()
                .map(Value::from_serialize)
                .map_err(Error::custom)
            });
        }

        for page in page_rx.into_iter() {
            let Some(layout) = page.data.get("layout").and_then(|v| v.as_str()) else {
                page_tx.send_blocking(page)?;
                continue;
            };

            let content = {
                let site = site.read().unwrap();

                let context = Context {
                    content: &page.content,
                    date: &page.date,
                    languages: &page.languages,
                    page: &page.data,
                    site: &site.data,
                    taxonomies: &site.taxonomies,
                    url: &page.url,
                    vitrine: Vitrine {
                        generator: concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION")),
                        version: env!("CARGO_PKG_VERSION"),
                    },
                };

                env.get_template(layout)
                    .with_context(|| {
                        format!(
                            "rendering {:?}",
                            page.file
                                .as_ref()
                                .map(|entry| entry.path().to_string_lossy().to_string())
                                .unwrap_or_default()
                        )
                    })?
                    .render(context)
                    .with_context(|| {
                        format!(
                            "rendering {:?}",
                            page.file
                                .as_ref()
                                .map(|entry| entry.path().to_string_lossy().to_string())
                                .unwrap_or_default()
                        )
                    })?
            };

            page_tx.send_blocking(Page { content, ..page })?;
        }
    }

    Ok(())
}
