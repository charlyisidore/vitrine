//! Generate sitemaps.

use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;

/// Preamble of the XML file.
const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>";

/// XML namespace for sitemaps.
const XMLNS: &str = "http://www.sitemaps.org/schemas/sitemap/0.9";

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum SitemapError {
    /// Deserialization error.
    #[error(transparent)]
    QuickXmlDe(#[from] quick_xml::DeError),
    /// Missing configuration:
    /// Provides a file path to the context of an existing error.
    #[error("file `{path}`")]
    WithFile {
        /// Source error.
        source: Box<Self>,
        /// File path.
        path: PathBuf,
    },
}

// <urlset>...</urlset>
#[derive(Debug, Default, Serialize)]
struct SitemapUrlset<'a> {
    #[serde(rename = "@xmlns")]
    xmlns: &'a str,
    url: Vec<SitemapUrl>,
}

// <url>...</url>
#[derive(Debug, Default, Serialize)]
struct SitemapUrl {
    loc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    lastmod: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changefreq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<f64>,
}

/// Pipeline task.
pub mod task {
    use quick_xml::se::Serializer;
    use serde::Serialize;

    use super::{SitemapError, SitemapUrl, SitemapUrlset, XMLNS, XML_DECLARATION};
    use crate::{
        build::{Page, Xml},
        config::Config,
        util::{
            pipeline::{Receiver, Sender, Task},
            value::Value,
        },
    };

    /// Task to generate sitemaps.
    #[derive(Debug)]
    pub struct SitemapTask<'config> {
        config: &'config Config,
    }

    impl<'config> SitemapTask<'config> {
        /// Create a pipeline task to generate sitemaps.
        pub fn new(config: &'config Config) -> Self {
            Self { config }
        }
    }

    impl Task<(Page,), (Page, Xml), SitemapError> for SitemapTask<'_> {
        fn process(
            self,
            (rx,): (Receiver<Page>,),
            (tx_page, tx_xml): (Sender<Page>, Sender<Xml>),
        ) -> Result<(), SitemapError> {
            let Some(config) = &self.config.sitemap else {
                // Just forward pages
                for page in rx {
                    tx_page.send(page).unwrap();
                }
                return Ok(());
            };

            let mut urlset = Vec::<SitemapUrl>::new();

            for page in rx {
                if let Some(sitemap_url) = page
                    .data
                    .get("sitemap")
                    .and_then(|page_sitemap| {
                        match page_sitemap {
                            Value::Map(map) => Some(SitemapUrl {
                                lastmod: map
                                    .get("lastmod")
                                    .and_then(|v| v.as_str().to_owned())
                                    .map(|s| s.to_string()),
                                changefreq: map
                                    .get("changefreq")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                priority: map.get("priority").and_then(|v| v.as_f64()),
                                ..Default::default()
                            }),
                            Value::Bool(value) => {
                                if *value {
                                    // sitemap = true
                                    Some(Default::default())
                                } else {
                                    // sitemap = false
                                    None
                                }
                            },
                            _ => Some(Default::default()),
                        }
                    })
                    .or_else(|| Some(Default::default()))
                    .map(|sitemap_url| {
                        // Fallback to defaults for unspecified fields
                        SitemapUrl {
                            loc: format!(
                                "{}{}{}",
                                config.url_prefix, self.config.base_url, page.url
                            ),
                            lastmod: sitemap_url
                                .lastmod
                                .or_else(|| {
                                    page.data
                                        .get("date")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                })
                                .or_else(|| Some(page.date.to_string())),
                            changefreq: sitemap_url
                                .changefreq
                                .or_else(|| config.changefreq.clone()),
                            priority: sitemap_url.priority.or(config.priority),
                        }
                    })
                {
                    urlset.push(sitemap_url);
                }

                tx_page.send(page).unwrap();
            }

            let urlset = SitemapUrlset {
                xmlns: XMLNS,
                url: urlset,
            };

            let mut buffer = String::new();

            let mut serializer = Serializer::with_root(&mut buffer, Some("urlset"))?;

            if !self.config.optimize {
                serializer.indent(' ', 2);
            }

            urlset.serialize(serializer)?;

            let content = if self.config.optimize {
                format!("{}{}", XML_DECLARATION, buffer)
            } else {
                format!("{}\n{}", XML_DECLARATION, buffer)
            };

            tx_xml
                .send(Xml {
                    url: format!("{}{}", self.config.base_url, config.url).into(),
                    content,
                })
                .unwrap();

            Ok(())
        }
    }
}
