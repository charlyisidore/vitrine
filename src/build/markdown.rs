//! Parse Markdown.
//!
//! [Markdown](<https://spec.commonmark.org/>) is a plain text format for writing structured
//! documents.
//!
//! This module uses [`markdown_it`] under the hood.

#[cfg(feature = "deno")]
mod katex;

use anyhow::{Result, anyhow};
use async_channel::{Receiver, Sender};
use markdown_it::{MarkdownIt, plugins};

use crate::{Config, Page, ReceiverExt};

/// Parse Markdown.
pub fn run(config: &Config, page_rx: Receiver<Page>, page_tx: Sender<Page>) -> Result<()> {
    if let Some(render) = config.markdown_render.as_ref() {
        for page in page_rx.into_iter() {
            if page.markup == "md" {
                page_tx.send_blocking(Page {
                    content: (render)(page.content)?,
                    markup: "html".to_string(),
                    ..page
                })?;
            } else {
                page_tx.send_blocking(page)?;
            }
        }
    } else {
        let mut parser = MarkdownIt::new();

        plugins::cmark::add(&mut parser);

        for plugin in &config.markdown_plugins {
            match plugin.as_ref() {
                "html" => plugins::html::add(&mut parser),
                #[cfg(feature = "deno")]
                "katex" => katex::add(&mut parser),
                "strikethrough" => plugins::extra::strikethrough::add(&mut parser),
                "beautify_links" => plugins::extra::beautify_links::add(&mut parser),
                "attrs" => plugins::extra::attrs::add(&mut parser),
                "linkify" => plugins::extra::linkify::add(&mut parser),
                "tables" => plugins::extra::tables::add(&mut parser),
                "syntect" => plugins::extra::syntect::add(&mut parser),
                "typographer" => plugins::extra::typographer::add(&mut parser),
                "smartquotes" => plugins::extra::smartquotes::add(&mut parser),
                "heading_anchors" => {
                    plugins::extra::heading_anchors::add(&mut parser, |s| slug::slugify(s))
                },
                "footnote" => plugins::extra::footnote::add(&mut parser),
                "sourcepos" => plugins::sourcepos::add(&mut parser),
                _ => return Err(anyhow!("Unknown plugin {:?}", plugin)),
            }
        }

        for page in page_rx.into_iter() {
            if page.markup == "md" {
                page_tx.send_blocking(Page {
                    content: parser.parse(&page.content).render(),
                    markup: "html".to_string(),
                    ..page
                })?;
            } else {
                page_tx.send_blocking(page)?;
            }
        }
    }

    Ok(())
}
