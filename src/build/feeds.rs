//! Generate feeds.

mod atom;

use anyhow::Result;
use async_channel::{Receiver, Sender};
use quick_xml::se::Serializer;
use serde::Serialize;
use serde_json::to_value;

use crate::{Config, File, FileContent, Page, ReceiverExt};

/// Preamble of the XML file.
const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>";

/// Generate feeds.
///
/// The generated files follow the [RFC 4287](https://www.rfc-editor.org/rfc/rfc4287) specification.
pub fn run(
    config: &Config,
    page_rx: Receiver<Page>,
    page_tx: Sender<Page>,
    file_tx: Sender<File>,
) -> Result<()> {
    let pages = page_rx.into_iter().collect::<Vec<_>>();

    for feed_config in &config.feeds {
        let mut entries = pages.iter().filter(|page| page.markup == "html").try_fold(
            Vec::new(),
            |mut entries, page| -> Result<_> {
                if let Some(filter) = &feed_config.filter {
                    let value = to_value(page)?;
                    if !(filter)(value)? {
                        return Ok(entries);
                    }
                }

                entries.push(atom::Entry {
                    author: Vec::new(),
                    category: Vec::new(),
                    content: None,
                    contributor: Vec::new(),
                    id: page.url.to_string(),
                    link: Vec::from([atom::Link {
                        href: page.url.to_string(),
                        ..Default::default()
                    }]),
                    published: None,
                    rights: None,
                    source: None,
                    summary: None,
                    updated: page.date.to_string(),
                    title: page
                        .data
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    ..Default::default()
                });

                Ok(entries)
            },
        )?;

        // Reverse chronological order
        entries.sort_by(|x, y| y.updated.cmp(&x.updated));

        let feed = atom::Feed {
            xmlns: atom::XMLNS,
            author: feed_config
                .author
                .iter()
                .map(|author| atom::PersonConstruct {
                    name: author.name.clone(),
                    uri: author.uri.clone(),
                    email: author.email.clone(),
                    ..Default::default()
                })
                .collect(),
            category: feed_config
                .category
                .iter()
                .map(|term| atom::Category {
                    term: term.clone(),
                    ..Default::default()
                })
                .collect(),
            contributor: feed_config
                .contributor
                .iter()
                .map(|contributor| atom::PersonConstruct {
                    name: contributor.name.clone(),
                    uri: contributor.uri.clone(),
                    email: contributor.email.clone(),
                    ..Default::default()
                })
                .collect(),
            generator: feed_config.generator.as_ref().map(|text| atom::Generator {
                text: text.clone(),
                ..Default::default()
            }),
            icon: feed_config.icon.clone(),
            id: feed_config
                .id
                .clone()
                .unwrap_or_else(|| feed_config.url.to_string()),
            logo: feed_config.logo.clone(),
            rights: feed_config.rights.clone(),
            subtitle: feed_config.subtitle.clone(),
            title: feed_config.title.clone(),
            updated: feed_config
                .updated
                .clone()
                .or_else(|| entries.first().map(|feed_entry| feed_entry.updated.clone()))
                .unwrap_or_default(),
            entry: entries,
            ..Default::default()
        };

        let mut buffer = String::new();

        let mut serializer = Serializer::with_root(&mut buffer, Some("feed"))?;

        if config.debug {
            serializer.indent(' ', 2);
        }

        feed.serialize(serializer)?;

        let content = if config.debug {
            format!("{}\n{}", XML_DECLARATION, buffer)
        } else {
            format!("{}{}", XML_DECLARATION, buffer)
        };

        file_tx.send_blocking(File {
            url: feed_config.url.clone(),
            content: FileContent::String(content),
        })?;
    }

    for page in pages {
        page_tx.send_blocking(page)?;
    }

    Ok(())
}
