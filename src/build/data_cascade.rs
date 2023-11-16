//! Apply data cascade.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use super::{Entry, EntryData, Error};

/// Parse data in a [`Entry`].
pub(super) fn parse_entry(entry: Entry) -> Result<Entry, Error> {
    let Some(content) = entry.content.as_ref() else {
        return Ok(entry);
    };

    let Some(extension) = entry
        .input_file
        .as_ref()
        .and_then(|dir_entry| dir_entry.path().extension())
        .and_then(|v| v.to_str())
    else {
        return Ok(entry);
    };

    let data = match extension {
        "json" => crate::util::data::json::read_str(content),
        "toml" => crate::util::data::toml::read_str(content),
        "yaml" => crate::util::data::yaml::read_str(content),
        _ => return Ok(entry),
    }
    .map_err(|error| Error::ParseCascadeData {
        input_path: entry.input_path_buf(),
        source: error,
    })?;

    Ok(Entry {
        content: None,
        data,
        format: "data".to_owned(),
        ..entry
    })
}

/// Apply data cascade to entries.
///
/// When there exists a data [`Entry`] with the same name (without extension) as
/// the current [`Entry`], use the data as metadata for the current entry,
/// unless the latter already contains metadata (e.g. front matter).
pub(super) fn cascade_entries(
    entries: impl Iterator<Item = Result<Entry, Error>>,
) -> Result<impl Iterator<Item = Result<Entry, Error>>, Error> {
    let entries: Vec<_> = entries.collect::<Result<_, _>>()?;

    // Collect data entries
    let data_map: HashMap<PathBuf, (PathBuf, EntryData)> = entries
        .iter()
        .filter_map(|entry| {
            if entry.format != "data" {
                return None;
            }

            let Some(data) = entry.data.as_ref() else {
                return None;
            };

            let Some(path) = entry.input_path_buf() else {
                return None;
            };

            let path_stem = path.with_extension("");

            Some((path_stem, (path, data.to_owned())))
        })
        .collect();

    let mut to_remove: HashSet<PathBuf> = HashSet::new();

    // Assign metadata to entries
    let entries: Vec<_> = entries
        .into_iter()
        .map(|entry| {
            if entry.data.is_some() {
                return entry;
            }

            // Apply data cascade to page entries only
            if !["html", "md"].contains(&entry.format.as_str()) {
                return entry;
            }

            let Some(path) = entry.input_path() else {
                return entry;
            };

            let path_stem = path.with_extension("");

            let Some((data_path, data)) = data_map.get(&path_stem) else {
                return entry;
            };

            to_remove.insert(data_path.to_owned());

            Entry {
                data: Some(data.to_owned()),
                ..entry
            }
        })
        .collect();

    // Remove used data entries
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| {
            entry
                .input_path()
                .map_or(true, |path| !to_remove.contains(path))
        })
        .collect();

    let entries = entries.into_iter().map(|v| Ok(v));

    Ok(entries)
}
