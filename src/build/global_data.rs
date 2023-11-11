//! Parse global data files.

use walkdir::WalkDir;

use super::{Config, Error};

/// Create global metadata from the data directory and the configuration.
///
/// First, this function reads data files in the `data_dir` directory if
/// specified. Then, it gets data from the `global_data` configuration variable.
/// Finally, it merges all the data into a single object (shallow merge).
pub(super) fn read(config: &Config) -> Result<serde_json::Value, Error> {
    let dir_data = read_data_dir(config)?;
    let config_data = config.global_data.as_object().cloned().unwrap_or_default();

    // Merge global data from directory and configuration file
    crate::util::data::shallow_merge(dir_data, config_data).map_err(|error| {
        Error::ReadGlobalDataInput {
            input_path: None,
            source: error,
        }
    })
}

/// Read global metadata from the data directory.
fn read_data_dir(config: &Config) -> Result<serde_json::Value, Error> {
    // If `data_dir` is not specified, no data file to read
    let Some(data_dir) = config.data_dir.as_ref() else {
        return Ok(serde_json::Map::new().into());
    };

    debug_assert!(data_dir.is_absolute());

    WalkDir::new(data_dir)
        .into_iter()
        .filter_entry(|entry| {
            // Skip hidden files and directories
            entry.depth() == 0
                || entry
                    .file_name()
                    .to_str()
                    .map(|file_name| !file_name.starts_with("."))
                    .unwrap_or(false)
        })
        .filter_map(|result| {
            // Ignore errors (e.g. permission denied)
            result.ok()
        })
        .filter(|entry| {
            // Keep only files, ignore directories
            entry.file_type().is_file()
        })
        .try_fold(serde_json::Map::new(), |mut dir_data, entry| {
            let path = entry.path();

            let extension = path
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or_default();

            let data = match extension {
                "json" => crate::util::data::json::read_file(path).map_err(|error| {
                    Error::ReadGlobalDataInput {
                        input_path: Some(path.to_owned()),
                        source: error,
                    }
                })?,
                "toml" => crate::util::data::toml::read_file(path).map_err(|error| {
                    Error::ReadGlobalDataInput {
                        input_path: Some(path.to_owned()),
                        source: error,
                    }
                })?,
                "yaml" => crate::util::data::yaml::read_file(path).map_err(|error| {
                    Error::ReadGlobalDataInput {
                        input_path: Some(path.to_owned()),
                        source: error,
                    }
                })?,
                _ => return Ok(dir_data),
            };

            // Get ["path", "to"] in "data_dir/path/to/file.json"
            let dir_components: Vec<_> = path
                .parent()
                .ok_or_else(|| Error::ReadGlobalDataInput {
                    input_path: Some(path.to_owned()),
                    source: anyhow::anyhow!("Cannot get file directory path"),
                })?
                .strip_prefix(&data_dir)
                .map_err(|error| Error::ReadGlobalDataInput {
                    input_path: Some(path.to_owned()),
                    source: error.into(),
                })?
                .components()
                .filter_map(|v| v.as_os_str().to_str())
                .collect();

            // Get "file" in "data_dir/path/to/file.json"
            let stem = path.file_stem().and_then(|v| v.to_str()).ok_or_else(|| {
                Error::ReadGlobalDataInput {
                    input_path: Some(path.to_owned()),
                    source: anyhow::anyhow!("Cannot get file stem"),
                }
            })?;

            // Find or create recursively the object corresponding to the file
            let mut object = &mut dir_data;

            for key in dir_components {
                object = object
                    .entry(key)
                    .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
                    .as_object_mut()
                    .ok_or_else(|| Error::ReadGlobalDataInput {
                        input_path: Some(path.to_owned()),
                        source: anyhow::anyhow!(
                            "Cannot merge data because key {:?} is not an object",
                            key
                        ),
                    })?;
            }

            if object.contains_key(stem) {
                return Err(Error::ReadGlobalDataInput {
                    input_path: Some(path.to_owned()),
                    source: anyhow::anyhow!(
                        "Cannot merge data because key {:?} already exists",
                        stem
                    ),
                });
            }

            object.insert(stem.to_owned(), data);

            Ok(dir_data)
        })
        .map(|v| v.into())
}
