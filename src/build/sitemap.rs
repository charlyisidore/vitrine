//! Generate a sitemap.

use chrono::{DateTime, Utc};
use quick_xml::se::Serializer;
use serde::Serialize;

use super::{Config, Entry, EntrySitemap, Error};

/// Preamble of the XML file.
const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>";

/// Generate a sitemap from page entries.
///
/// The generated file follows the [sitemap protocol](https://www.sitemaps.org/protocol.html).
pub(super) fn create_sitemap_entries(
    entries: impl Iterator<Item = Result<Entry, Error>>,
    config: &Config,
) -> Result<impl Iterator<Item = Result<Entry, Error>>, Error> {
    let mut entries: Vec<_> = entries.collect::<Result<_, _>>()?;

    // Sitemap is opt-in
    if let Some(sitemap_config) = config.sitemap.as_ref() {
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

        let sitemap_urls: Vec<SitemapUrl> =
            entries.iter().try_fold(Vec::new(), |mut sitemap, entry| {
                // Generate sitemap only for pages
                if entry.format != "html" {
                    return Ok(sitemap);
                }

                let sitemap_entry = if let Some(sitemap_data) =
                    entry.data.as_ref().and_then(|data| data.sitemap.as_ref())
                {
                    // Get sitemap configuration from metadata
                    match sitemap_data {
                        EntrySitemap::Bool(value) => {
                            if *value {
                                // sitemap = true
                                Default::default()
                            } else {
                                // sitemap = false
                                return Ok(sitemap);
                            }
                        },
                        EntrySitemap::Object {
                            lastmod,
                            changefreq,
                            priority,
                        } => SitemapUrl {
                            lastmod: lastmod.to_owned(),
                            changefreq: changefreq.to_owned(),
                            priority: priority.to_owned(),
                            ..Default::default()
                        },
                    }
                } else {
                    // No metadata, fallback to defaults
                    Default::default()
                };

                // Fallback to defaults for unspecified fields
                let sitemap_entry = SitemapUrl {
                    loc: format!("{}{}", config.base_url, entry.url),
                    lastmod: sitemap_entry
                        .lastmod
                        .or_else(|| entry.data.as_ref().and_then(|data| data.date.to_owned()))
                        .or_else(|| {
                            entry
                                .input_file
                                .as_ref()
                                .and_then(|dir_entry| dir_entry.metadata().ok())
                                .and_then(|metadata| metadata.modified().ok())
                                .map(|date| {
                                    let date: DateTime<Utc> = date.into();
                                    format!("{}", date.format("%+"))
                                })
                        }),
                    changefreq: sitemap_entry
                        .changefreq
                        .or_else(|| sitemap_config.changefreq.to_owned()),
                    priority: sitemap_entry
                        .priority
                        .or_else(|| sitemap_config.priority.to_owned()),
                    ..sitemap_entry
                };

                sitemap.push(sitemap_entry);

                Ok(sitemap)
            })?;

        let urlset = SitemapUrlset {
            xmlns: "http://www.sitemaps.org/schemas/sitemap/0.9",
            url: sitemap_urls,
        };

        let mut buffer = String::new();

        let mut serializer =
            Serializer::with_root(&mut buffer, Some("urlset")).map_err(|error| {
                Error::CreateSitemap {
                    source: error.into(),
                }
            })?;

        if !config.minify {
            serializer.indent(' ', 2);
        }

        urlset
            .serialize(serializer)
            .map_err(|error| Error::CreateSitemap {
                source: error.into(),
            })?;

        let content = if config.minify {
            format!("{}{}", XML_DECLARATION, buffer)
        } else {
            format!("{}\n{}", XML_DECLARATION, buffer)
        };

        entries.push(Entry {
            url: sitemap_config.url.to_owned(),
            format: "xml".to_owned(),
            input_file: None,
            content: Some(content),
            data: None,
        });
    }

    let entries = entries.into_iter().map(|v| Ok(v));

    Ok(entries)
}
