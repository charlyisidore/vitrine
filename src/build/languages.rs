//! Multiple languages support.

use std::collections::BTreeMap;

use anyhow::Result;
use async_channel::{Receiver, Sender};

use crate::{Config, Page, ReceiverExt};

/// Group pages according to their language.
pub fn run(config: &Config, page_rx: Receiver<Page>, page_tx: Sender<Page>) -> Result<()> {
    let Some(default_lang) = &config.default_lang else {
        for page in page_rx.into_iter() {
            page_tx.send_blocking(page)?;
        }
        return Ok(());
    };

    let pages = page_rx
        .into_iter()
        .fold(BTreeMap::<_, Vec<_>>::new(), |mut pages, page| {
            let url = page.url.clone();
            let page = if let Some(lang) = page.lang.as_ref().or(config.default_lang.as_ref()) {
                Page {
                    url: format!("/{}{}", lang, page.url).try_into().unwrap(),
                    ..page
                }
            } else {
                page
            };
            pages.entry(url).or_default().push(page);
            pages
        });

    for pages in pages.into_values() {
        let languages = pages
            .iter()
            .filter_map(|page| {
                page.lang
                    .as_ref()
                    .or(Some(default_lang))
                    .map(|lang| (lang.clone(), page.url.clone()))
            })
            .collect::<BTreeMap<_, _>>();

        for mut page in pages {
            page.languages = languages
                .iter()
                .filter(|(lang, _)| {
                    page.lang
                        .as_ref()
                        .or(Some(default_lang))
                        .is_some_and(|page_lang| page_lang != *lang)
                })
                .map(|(lang, url)| (lang.clone(), url.clone()))
                .collect();

            page_tx.send_blocking(page)?;
        }
    }

    Ok(())
}
