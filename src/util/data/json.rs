//! Read JSON data files.

use std::path::Path;

use serde::de::DeserializeOwned;

/// Read data from a JSON file.
pub(crate) fn read_file<T, P>(path: P) -> anyhow::Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    read_str(content)
}

/// Read data from a JSON string.
pub(crate) fn read_str<T, S>(content: S) -> anyhow::Result<T>
where
    T: DeserializeOwned,
    S: AsRef<str>,
{
    Ok(serde_json::from_str(content.as_ref())?)
}
