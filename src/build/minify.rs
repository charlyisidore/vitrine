//! Minify HTML.
//!
//! This module uses [`minify_html`] under the hood.

use anyhow::Result;
use async_channel::{Receiver, Sender};
use minify_html::{Cfg, minify};

use crate::{Config, Page, ReceiverExt};

/// Minify HTML.
pub fn run(config: &Config, page_rx: Receiver<Page>, page_tx: Sender<Page>) -> Result<()> {
    if config.debug {
        for page in page_rx.into_iter() {
            page_tx.send_blocking(page)?;
        }
        return Ok(());
    }

    let cfg = Cfg::new();

    for page in page_rx.into_iter() {
        page_tx.send_blocking(Page {
            content: String::from_utf8(minify(page.content.as_bytes(), &cfg))?,
            ..page
        })?;
    }

    Ok(())
}
