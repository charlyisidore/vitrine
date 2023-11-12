//! Read YAML data files.

use std::path::Path;

use serde::de::DeserializeOwned;

/// Read data from a YAML file.
pub(crate) fn read_file<T, P>(path: P) -> anyhow::Result<T>
where
    P: AsRef<Path>,
    T: DeserializeOwned,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    read_str(content)
}

/// Read data from a YAML string.
pub(crate) fn read_str<T, S>(content: S) -> anyhow::Result<T>
where
    S: AsRef<str>,
    T: DeserializeOwned,
{
    Ok(serde_yaml::from_str(content.as_ref())?)
}
