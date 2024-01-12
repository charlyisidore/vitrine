//! Generate the navigation tree.

use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

use serde::{Deserialize, Serialize};

use super::{Config, Entry, Error};

/// Navigation tree node.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Tree {
    /// Data associated to the node.
    pub data: Option<Data>,
    /// Child nodes.
    pub children: BTreeMap<String, Tree>,
}

/// Navigation tree node data.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Data {
    /// Title of the entry associated to this node.
    pub title: Option<String>,
    /// URL of the entry associated to this node.
    pub url: String,
}

impl Tree {
    /// Insert a node in the navigation tree with given entry path.
    fn insert<P>(&mut self, path: P, data: Data)
    where
        P: AsRef<Path>,
    {
        let mut node = self;
        for component in path.as_ref().components() {
            if component == Component::RootDir {
                continue;
            }
            let key = component.as_os_str().to_str().unwrap().to_string();
            node = node.children.entry(key).or_default();
        }
        node.data = Some(data);
    }

    /// Return a node given the entry path.
    pub(super) fn get<P>(&self, path: P) -> Option<&Self>
    where
        P: AsRef<Path>,
    {
        let mut node = self;
        for component in path.as_ref().components() {
            if component == Component::RootDir {
                continue;
            }
            let key = component.as_os_str().to_str().unwrap().to_string();
            let child = node.children.get(&key);
            if child.is_none() {
                return None;
            }
            node = child.unwrap();
        }
        Some(node)
    }
}

/// Generate the navigation tree.
pub(super) fn create_navigation_entries(
    entries: impl Iterator<Item = Result<Entry, Error>>,
    config: &Config,
) -> Result<impl Iterator<Item = Result<Entry, Error>>, Error> {
    let entries: Vec<_> = entries.collect::<Result<_, _>>()?;

    // Navigation tree is opt-in
    let entries = if let Some(navigation_config) = config.navigation.as_ref() {
        // Generate the entire navigation tree
        let tree = entries.iter().filter(|entry| entry.format == "html").fold(
            Tree::default(),
            |mut tree, entry| {
                let data = entry.data.as_ref();
                tree.insert(&entry.url, Data {
                    title: data.and_then(|data| data.title.to_owned()),
                    url: entry.url.to_owned(),
                });
                tree
            },
        );

        // In each entry's data, insert the corresponding node
        entries
            .into_iter()
            .map(|entry| {
                let Some(navigation) = tree
                    .get(&entry.url)
                    .map(|tree| serde_json::to_value(tree))
                    .transpose()?
                else {
                    return Ok(entry);
                };

                let Some(mut data) = entry.data else {
                    return Ok(entry);
                };

                let Some(object) = data.extra.as_object_mut() else {
                    return Ok(Entry {
                        data: Some(data),
                        ..entry
                    });
                };

                object
                    .entry(&navigation_config.navigation_key)
                    .or_insert(navigation);

                Ok(Entry {
                    data: Some(data),
                    ..entry
                })
            })
            .collect::<anyhow::Result<_>>()
            .map_err(|error| Error::CreateNavigation {
                source: error.into(),
            })?
    } else {
        entries
    };

    let entries = entries.into_iter().map(|v| Ok(v));

    Ok(entries)
}
