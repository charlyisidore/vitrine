//! Bundle contents.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use super::{Entry, EntryData, Error};

/// Bundle contents.
///
/// The `contents` property in metadata can be used to merge entries. This
/// function reads in each entry the `contents` property as an object mapping
/// keys to relative paths. Then it replaces paths by the content of their
/// respective entries. The result can be used in layouts through the `contents`
/// variable, e.g. to inline CSS or JS code. Besides, entries that have been
/// merged are removed and do not produce any output.
pub(super) fn bundle_entries(
    entries: impl Iterator<Item = Result<Entry, Error>>,
) -> Result<impl Iterator<Item = Result<Entry, Error>>, Error> {
    let entries: Vec<_> = entries.collect::<Result<_, _>>()?;

    let entry_map: HashMap<PathBuf, String> = entries
        .iter()
        .filter_map(|entry| {
            entry.input_file.as_ref().map(|dir_entry| {
                (
                    dir_entry.path().to_owned(),
                    entry.content.clone().unwrap_or_default(),
                )
            })
        })
        .collect();

    let mut to_keep: HashSet<PathBuf> = HashSet::new();
    let mut to_remove: HashSet<PathBuf> = HashSet::new();

    let entries: Vec<_> = entries
        .into_iter()
        .map(|entry| {
            if entry
                .data
                .as_ref()
                .filter(|data| !data.contents.is_empty())
                .is_none()
            {
                return Ok(entry);
            }

            let Some(dir) = entry
                .input_file
                .as_ref()
                .and_then(|v| v.path().parent().map(|v| v.to_owned()))
            else {
                return Ok(entry);
            };

            let data = entry.data.unwrap_or_default();

            let contents = data
                .contents
                .iter()
                .map(|(key, path)| {
                    let content = if path == "." {
                        entry.content.clone().unwrap_or_default()
                    } else {
                        let path = dir.join(&path).canonicalize().map_err(|error| {
                            anyhow::anyhow!(error).context(format!("Entry {:?}", path))
                        })?;

                        let content = entry_map
                            .get(&path)
                            .ok_or_else(|| anyhow::anyhow!("Entry {:?} not found", path))?
                            .to_owned();

                        to_remove.insert(path.to_owned());

                        content
                    };

                    Ok::<_, anyhow::Error>((key.to_owned(), content))
                })
                .collect::<Result<_, _>>()
                .map_err(|error| {
                    error.context(format!(
                        "In `contents` metadata in {:?}",
                        entry.input_file.as_ref().map(|v| v.path().to_owned())
                    ))
                })?;

            to_keep.insert(
                entry
                    .input_file
                    .as_ref()
                    .map(|v| v.path().to_owned())
                    .unwrap(),
            );

            Ok(Entry {
                data: Some(EntryData { contents, ..data }),
                ..entry
            })
        })
        .collect::<Result<_, _>>()
        .map_err(|error| Error::BundleContents { source: error })?;

    let entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| {
            entry
                .input_path()
                .map(|path| to_keep.contains(path) || !to_remove.contains(path))
                .unwrap_or(true)
        })
        .collect();

    Ok(entries.into_iter().map(|entry| Ok(entry)))
}
