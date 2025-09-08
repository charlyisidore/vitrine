//! Group pages using taxonomies.

use std::sync::{Arc, RwLock};

use anyhow::Result;
use async_channel::{Receiver, Sender};

use crate::{Config, Page, ReceiverExt, Site, TaxonomyItem};

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
pub fn run(
    config: &Config,
    site: Arc<RwLock<Site>>,
    page_rx: Receiver<Page>,
    page_tx: Sender<Page>,
) -> Result<()> {
    {
        let mut site = site.write().unwrap();

        // taxonomies.{taxonomy}.{term} = [{entry_1}, {entry_2}, ...]
        // e.g. taxonomies.tags.post = [{url: "/posts/1"...}, {url: "/posts/2"...}, ...]
        for key in &config.taxonomies {
            site.taxonomies.entry(key.to_string()).or_default();
        }
    }

    for page in page_rx.into_iter() {
        if !page.data.is_object() {
            page_tx.send_blocking(page)?;
            continue;
        };

        let mut site = site.write().unwrap();

        for (key, taxonomy) in site.taxonomies.iter_mut() {
            let Some(keys) = page.data.get(key).and_then(|v| {
                // Terms can be specified as an array of string or a single string (converted to
                // an array of strings)
                v.as_array()
                    .map(|v| v.iter().filter_map(|v| v.as_str()).collect())
                    .or_else(|| v.as_str().map(|v| vec![v]))
            }) else {
                continue;
            };

            for key in keys {
                let collection = taxonomy.entry(key.to_string()).or_default();

                collection.push(TaxonomyItem {
                    data: page.data.clone(),
                    date: page.date.clone(),
                    url: page.url.clone(),
                });
            }
        }

        page_tx.send_blocking(page)?;
    }

    Ok(())
}
