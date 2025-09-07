//! Enable live reloading.

use anyhow::Result;
use async_channel::{Receiver, Sender};
use lol_html::{RewriteStrSettings, element, html_content::ContentType, rewrite_str};

use crate::{Config, Page, ReceiverExt};

const RELOAD_JS: &str = r#"<script>
const eventSource = new EventSource("/_vitrine");
eventSource.addEventListener("reload", () => location.reload());
</script>"#;

/// Enable live reloading.
pub fn run(config: &Config, page_rx: Receiver<Page>, page_tx: Sender<Page>) -> Result<()> {
    if !config.debug {
        for page in page_rx.into_iter() {
            page_tx.send_blocking(page)?;
        }
        return Ok(());
    }

    for page in page_rx.into_iter() {
        let content = rewrite_str(&page.content, RewriteStrSettings {
            element_content_handlers: vec![element!("head", |element| {
                element.append(RELOAD_JS, ContentType::Html);
                Ok(())
            })],
            ..Default::default()
        })?;

        page_tx.send_blocking(Page { content, ..page })?;
    }

    Ok(())
}
