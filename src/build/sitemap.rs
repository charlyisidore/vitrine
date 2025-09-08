//! Generate a sitemap.

use anyhow::Result;
use async_channel::{Receiver, Sender};
use quick_xml::se::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::from_value;

use crate::{Config, DateTime, File, FileContent, Page, ReceiverExt};

/// Preamble of the XML file.
const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>";
const XMLNS: &str = "http://www.sitemaps.org/schemas/sitemap/0.9";

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
    lastmod: Option<DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changefreq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<f64>,
}

/// Sitemap configuration for a page.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PageSitemap {
    /// If false, the page will not be shown in the sitemap.
    Bool(bool),
    /// Sitemap configuration for the page.
    Object {
        /// Date of last modification.
        lastmod: Option<DateTime>,

        /// Change frequency.
        changefreq: Option<String>,

        /// Priority.
        priority: Option<f64>,
    },
}

/// Generate a sitemap from page entries.
///
/// The generated file follows the [sitemap protocol](<https://www.sitemaps.org/protocol.html>).
pub fn run(
    config: &Config,
    page_rx: Receiver<Page>,
    page_tx: Sender<Page>,
    file_tx: Sender<File>,
) -> Result<()> {
    if let Some(sitemap_config) = config.sitemap.as_ref() {
        let mut urlset = Vec::new();

        for page in page_rx.into_iter() {
            if page.markup != "html" {
                page_tx.send_blocking(page)?;
                continue;
            }

            let sitemap_url = if let Some(page_sitemap) = page
                .data
                .get("sitemap")
                .and_then(|value| from_value(value.clone()).ok())
            {
                // Get sitemap configuration from metadata
                match page_sitemap {
                    PageSitemap::Bool(value) => {
                        if value {
                            // sitemap = true
                            Default::default()
                        } else {
                            // sitemap = false
                            page_tx.send_blocking(page)?;
                            continue;
                        }
                    },
                    PageSitemap::Object {
                        lastmod,
                        changefreq,
                        priority,
                    } => SitemapUrl {
                        lastmod,
                        changefreq,
                        priority,
                        ..Default::default()
                    },
                }
            } else {
                // No metadata, fallback to defaults
                Default::default()
            };

            // Fallback to defaults for unspecified fields
            let sitemap_url = SitemapUrl {
                loc: format!(
                    "{}{}{}",
                    sitemap_config.url_prefix, config.base_url, page.url
                ),
                lastmod: sitemap_url.lastmod.or_else(|| Some(page.date.clone())),
                changefreq: sitemap_url
                    .changefreq
                    .or_else(|| sitemap_config.changefreq.clone()),
                priority: sitemap_url.priority.or(sitemap_config.priority),
            };

            urlset.push(sitemap_url);

            page_tx.send_blocking(page)?;
        }

        let urlset = SitemapUrlset {
            xmlns: XMLNS,
            url: urlset,
        };

        let mut buffer = String::new();

        let mut serializer = Serializer::with_root(&mut buffer, Some("urlset"))?;

        if config.debug {
            serializer.indent(' ', 2);
        }

        urlset.serialize(serializer)?;

        let content = if config.debug {
            format!("{}\n{}", XML_DECLARATION, buffer)
        } else {
            format!("{}{}", XML_DECLARATION, buffer)
        };

        file_tx.send_blocking(File {
            url: sitemap_config
                .url
                .clone()
                .unwrap_or_else(|| "/sitemap.xml".try_into().unwrap()),
            content: FileContent::String(content),
        })?;
    } else {
        for page in page_rx.into_iter() {
            page_tx.send_blocking(page)?;
        }
    }

    Ok(())
}
