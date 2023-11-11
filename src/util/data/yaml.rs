//! Read YAML data files.

use std::path::Path;

/// Read data from a YAML file.
pub(crate) fn read_file<P>(path: P) -> anyhow::Result<serde_json::Value>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    read_str(content)
}

/// Read data from a YAML string.
pub(crate) fn read_str<S>(content: S) -> anyhow::Result<serde_json::Value>
where
    S: AsRef<str>,
{
    Ok(serde_yaml::from_str(content.as_ref())?)
}
