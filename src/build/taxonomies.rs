//! Group entries using taxonomies.

use std::collections::HashMap;

use super::{Config, Entry, Error};

/// Group entries using taxonomies.
///
/// Taxonomy keys are specified under the `taxonomies` key in the configuration.
///
/// This function reads the terms given under the taxonomy keys in each entry
/// metadata. The terms can be specified as a string or an array of strings.
/// Then it creates an object organized in two levels. The first level maps
/// taxonomy keys (e.g. `tags`, `category`) to collections of terms. The second
/// level maps terms (e.g. a specific tag) to a list of entries associated to
/// the term. The result is saved in the global data under the key `taxonomies`.
pub(super) fn group_entries(
    entries: impl Iterator<Item = Result<Entry, Error>>,
    config: &Config,
    mut global_data: serde_json::Value,
) -> Result<
    (
        impl Iterator<Item = Result<Entry, Error>>,
        serde_json::Value,
    ),
    Error,
> {
    let entries: Vec<_> = entries.collect::<Result<_, _>>()?;

    // taxonomies.{taxonomy}.{term} = [{entry_1}, {entry_2}, ...]
    // e.g. taxonomies.tags.post = [{url: "/posts/1"...}, {url: "/posts/2"...}, ...]
    let taxonomies: HashMap<String, HashMap<String, Vec<serde_json::Value>>> = config
        .taxonomies
        .iter()
        .map(|key| (key.to_owned(), HashMap::new()))
        .collect();

    let taxonomies = entries
        .iter()
        .try_fold(taxonomies, |mut taxonomies, entry| -> Result<_, _> {
            let Some(data) = entry.data.as_ref() else {
                return Ok(taxonomies);
            };

            let data = serde_json::to_value(data)?;

            for (key, taxonomy) in taxonomies.iter_mut() {
                let Some(keys) = data.get(key).and_then(|v| {
                    // Terms can be specified as an array of string or a single string (converted to
                    // an array of strings)
                    v.as_array()
                        .map(|v| v.iter().filter_map(|v| v.as_str()).collect())
                        .or_else(|| v.as_str().map(|v| Vec::from([v])))
                }) else {
                    continue;
                };

                for key in keys {
                    let collection = taxonomy.entry(key.to_owned()).or_default();

                    let entry = serde_json::Map::from_iter([
                        ("url".to_string(), serde_json::to_value(&entry.url)?),
                        ("content".to_string(), serde_json::to_value(&entry.content)?),
                        (
                            "data".to_string(),
                            entry
                                .data
                                .as_ref()
                                .map(|data| serde_json::to_value(data))
                                .transpose()?
                                .unwrap_or_else(|| serde_json::Value::from(serde_json::Map::new())),
                        ),
                    ]);

                    let entry = serde_json::to_value(entry)?;

                    collection.push(entry);
                }
            }

            Ok(taxonomies)
        })
        .and_then(|taxonomies| serde_json::to_value(taxonomies))
        .map_err(|error| Error::GroupTaxonomies {
            source: error.into(),
        })?;

    let entries = entries.into_iter().map(|v| Ok(v));

    let mut global_data = global_data.as_object_mut().cloned().unwrap_or_default();
    global_data.insert("taxonomies".to_owned(), taxonomies);

    let global_data =
        serde_json::to_value(global_data).map_err(|error| Error::GroupTaxonomies {
            source: error.into(),
        })?;

    Ok((entries, global_data))
}
