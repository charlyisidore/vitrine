//! Generate feeds.

pub mod atom;

use std::path::PathBuf;

use thiserror::Error;

use crate::util::{function::Function, value::Value};

/// Preamble of the XML file.
const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>";

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum FeedsError {
    /// Any error.
    #[error(transparent)]
    Any(#[from] crate::util::function::AnyError),
    /// Date/time error.
    #[error(transparent)]
    DateTime(#[from] crate::util::date_time::DateTimeError),
    /// Deserialization error.
    #[error(transparent)]
    QuickXmlDe(#[from] quick_xml::DeError),
    /// Provides a file path to the context of an existing error.
    #[error("file `{path}`")]
    WithFile {
        /// Source error.
        source: Box<Self>,
        /// File path.
        path: PathBuf,
    },
}

/// Filter to determine if an entry belongs to a feed.
pub type FeedFilter = Function<(Value,), bool>;

/// Pipeline task.
pub mod task {
    use quick_xml::se::Serializer;
    use serde::Serialize;

    use super::{atom, FeedsError, XML_DECLARATION};
    use crate::{
        build::{Page, Xml},
        config::Config,
        util::{
            date_time::DateTime,
            pipeline::{Receiver, Sender, Task},
        },
    };

    /// Task to generate feeds.
    #[derive(Debug)]
    pub struct FeedsTask<'config> {
        config: &'config Config,
    }

    impl<'config> FeedsTask<'config> {
        /// Create a pipeline task to generate feeds.
        pub fn new(config: &'config Config) -> Self {
            Self { config }
        }
    }

    impl Task<(Page, Xml), (Page, Xml), FeedsError> for FeedsTask<'_> {
        fn process(
            self,
            (rx_page, rx_xml): (Receiver<Page>, Receiver<Xml>),
            (tx_page, tx_xml): (Sender<Page>, Sender<Xml>),
        ) -> Result<(), FeedsError> {
            let pages: Vec<Page> = rx_page.into_iter().collect();

            for xml in rx_xml {
                tx_xml.send(xml).unwrap();
            }

            for config in &self.config.feeds {
                let mut entries: Vec<atom::Entry> = pages.iter().try_fold(
                    Vec::new(),
                    |mut entries, page| -> Result<_, FeedsError> {
                        if !config
                            .filter
                            .as_ref()
                            .map(|filter| filter.call(page.data.clone()))
                            .transpose()?
                            .unwrap_or(false)
                        {
                            return Ok(entries);
                        }

                        let link = Vec::from([atom::Link {
                            href: page.url.to_string(),
                            ..Default::default()
                        }]);

                        let updated = page
                            .data
                            .as_map()
                            .and_then(|map| map.get("date"))
                            .and_then(|v| v.as_str())
                            .map(DateTime::parse)
                            .transpose()?
                            .unwrap_or_else(|| page.date.clone())
                            .to_string();

                        let title = page
                            .data
                            .as_map()
                            .and_then(|map| map.get("title"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_default();

                        entries.push(atom::Entry {
                            author: Vec::new(),
                            category: Vec::new(),
                            content: None,
                            contributor: Vec::new(),
                            id: page.url.to_string(),
                            link,
                            published: None,
                            rights: None,
                            source: None,
                            summary: None,
                            updated,
                            title,
                            ..Default::default()
                        });

                        Ok(entries)
                    },
                )?;

                // Reverse chronological order
                entries.sort_by(|x, y| y.updated.cmp(&x.updated));

                let author = config
                    .author
                    .iter()
                    .map(|author| atom::PersonConstruct {
                        name: author.name.clone(),
                        uri: author.uri.clone(),
                        email: author.email.clone(),
                        ..Default::default()
                    })
                    .collect();

                let category = config
                    .category
                    .iter()
                    .map(|term| atom::Category {
                        term: term.clone(),
                        ..Default::default()
                    })
                    .collect();

                let contributor = config
                    .contributor
                    .iter()
                    .map(|contributor| atom::PersonConstruct {
                        name: contributor.name.clone(),
                        uri: contributor.uri.clone(),
                        email: contributor.email.clone(),
                        ..Default::default()
                    })
                    .collect();

                let generator = config.generator.as_ref().map(|text| atom::Generator {
                    text: text.clone(),
                    ..Default::default()
                });

                let updated = config
                    .updated
                    .clone()
                    .or_else(|| entries.first().map(|feed_entry| feed_entry.updated.clone()))
                    .unwrap_or_default();

                let feed = atom::Feed {
                    xmlns: atom::XMLNS,
                    author,
                    category,
                    contributor,
                    generator,
                    icon: config.icon.clone(),
                    id: config.id.clone().unwrap_or_else(|| config.url.clone()),
                    logo: config.logo.clone(),
                    rights: config.rights.clone(),
                    subtitle: config.subtitle.clone(),
                    title: config.title.clone(),
                    updated,
                    entry: entries,
                    ..Default::default()
                };

                let mut buffer = String::new();

                let mut serializer = Serializer::with_root(&mut buffer, Some("feed"))?;
                serializer.indent(' ', 2);

                feed.serialize(serializer)?;

                let url = config.url.clone().into();
                let content = format!("{}\n{}", XML_DECLARATION, buffer);

                tx_xml.send(Xml { url, content }).unwrap();
            }

            for page in pages {
                tx_page.send(page).unwrap();
            }

            Ok(())
        }
    }
}
