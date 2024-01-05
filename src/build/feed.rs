//! Generate feeds.

use chrono::{DateTime, Utc};
use quick_xml::se::Serializer;
use serde::Serialize;

use super::{Config, Entry, Error};
use crate::util::feed::atom;

/// Preamble of the XML file.
const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>";

/// Generate feeds.
///
/// The generated files follow the [RFC 4287](https://www.rfc-editor.org/rfc/rfc4287) specification.
pub(super) fn create_feeds_entries(
    entries: impl Iterator<Item = Result<Entry, Error>>,
    config: &Config,
) -> Result<impl Iterator<Item = Result<Entry, Error>>, Error> {
    let mut entries: Vec<_> = entries.collect::<Result<_, _>>()?;

    for feed_config in config.feeds.iter() {
        let mut feed_entries: Vec<atom::Entry> = entries
            .iter()
            .try_fold(
                Vec::new(),
                |mut feed_entries, entry| -> anyhow::Result<Vec<atom::Entry>> {
                    // Generate feed only for pages
                    if entry.format != "html" {
                        return Ok(feed_entries);
                    }

                    let include = match feed_config.filter.as_ref() {
                        Some(filter) => {
                            let data = serde_json::to_value(&entry.data)?;
                            filter.call_1(&data)?
                        },
                        None => true,
                    };

                    if !include {
                        return Ok(feed_entries);
                    }

                    // TODO
                    feed_entries.push(atom::Entry {
                        author: Vec::new(),
                        category: Vec::new(),
                        content: None,
                        contributor: Vec::new(),
                        id: entry.url.to_owned(),
                        link: Vec::from([atom::Link {
                            href: entry.url.to_owned(),
                            ..Default::default()
                        }]),
                        published: None,
                        rights: None,
                        source: None,
                        summary: None,
                        updated: entry
                            .data
                            .as_ref()
                            .and_then(|data| data.date.to_owned())
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
                            })
                            .unwrap_or_default(),
                        title: entry
                            .data
                            .as_ref()
                            .and_then(|data| data.title.to_owned())
                            .unwrap_or_default(),
                        ..Default::default()
                    });

                    Ok(feed_entries)
                },
            )
            .map_err(|error| Error::CreateFeed { source: error })?;

        // Reverse chronological order
        feed_entries.sort_by(|x, y| y.updated.cmp(&x.updated));

        let feed = atom::Feed {
            xmlns: atom::XMLNS,
            author: feed_config
                .author
                .iter()
                .map(|author| atom::PersonConstruct {
                    name: author.name.to_owned(),
                    uri: author.uri.to_owned(),
                    email: author.email.to_owned(),
                    ..Default::default()
                })
                .collect(),
            category: feed_config
                .category
                .iter()
                .map(|term| atom::Category {
                    term: term.to_owned(),
                    ..Default::default()
                })
                .collect(),
            contributor: feed_config
                .contributor
                .iter()
                .map(|contributor| atom::PersonConstruct {
                    name: contributor.name.to_owned(),
                    uri: contributor.uri.to_owned(),
                    email: contributor.email.to_owned(),
                    ..Default::default()
                })
                .collect(),
            generator: feed_config.generator.as_ref().map(|text| atom::Generator {
                text: text.to_owned(),
                ..Default::default()
            }),
            icon: feed_config.icon.to_owned(),
            id: feed_config
                .id
                .to_owned()
                .unwrap_or_else(|| feed_config.url.to_owned()),
            logo: feed_config.logo.to_owned(),
            rights: feed_config.rights.to_owned(),
            subtitle: feed_config.subtitle.to_owned(),
            title: feed_config.title.to_owned(),
            updated: feed_config
                .updated
                .to_owned()
                .or_else(|| {
                    feed_entries
                        .first()
                        .map(|feed_entry| feed_entry.updated.to_owned())
                })
                .unwrap_or_default(),
            entry: feed_entries,
            ..Default::default()
        };

        let mut buffer = String::new();

        let mut serializer = Serializer::with_root(&mut buffer, Some("feed")).map_err(|error| {
            Error::CreateFeed {
                source: error.into(),
            }
        })?;

        if !config.minify {
            serializer.indent(' ', 2);
        }

        feed.serialize(serializer)
            .map_err(|error| Error::CreateFeed {
                source: error.into(),
            })?;

        let content = if config.minify {
            format!("{}{}", XML_DECLARATION, buffer)
        } else {
            format!("{}\n{}", XML_DECLARATION, buffer)
        };

        entries.push(Entry {
            url: feed_config.url.to_owned(),
            format: "xml".to_owned(),
            content: Some(content),
            ..Default::default()
        });
    }

    let entries = entries.into_iter().map(|v| Ok(v));

    Ok(entries)
}
