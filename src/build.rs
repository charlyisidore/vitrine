//! Build the site.

mod assets;
mod feeds;
mod input;
mod languages;
mod layout;
mod markdown;
mod metadata;
mod minify;
mod output;
mod reload;
mod scripts;
mod sitemap;
mod styles;
mod taxonomies;

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use anyhow::Result;
use async_channel::unbounded;
use ignore::DirEntry;
use serde::{Deserialize, Serialize};

use crate::{Config, DateTime, UriRelativeString, Value};

/// A page entry.
///
/// A page represents a future HTML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Directory entry, if any.
    #[serde(skip)]
    file: Option<DirEntry>,

    /// URL from which the entry will be accessible.
    ///
    /// The URL determines the output file name (e.g. `/blog/` outputs
    /// `/blog/index.html`).
    url: UriRelativeString,

    /// Markup format.
    markup: String,

    /// Content of the entry.
    content: String,

    /// Page date.
    date: DateTime,

    /// Page language code.
    lang: Option<String>,

    /// Same page in different languages.
    languages: BTreeMap<String, UriRelativeString>,

    /// Page data.
    data: Value,
}

/// An arbitrary file entry.
#[derive(Debug)]
pub struct File {
    /// URL from which the entry will be accessible.
    pub url: UriRelativeString,

    /// File content.
    pub content: FileContent,
}

/// A file entry content.
#[derive(Debug)]
pub enum FileContent {
    /// Input file path from which the entry comes from, if applicable.
    Path(PathBuf),
    /// File content as a string, if applicable.
    String(String),
    /// File content as bytes, if applicable.
    Bytes(Vec<u8>),
}

/// An image entry.
#[derive(Debug)]
pub struct Image {
    /// Input file path from which the entry comes from.
    pub path: PathBuf,

    /// Image target width.
    pub width: Option<u32>,

    /// Image target height.
    pub height: Option<u32>,

    /// URL from which the entry will be accessible.
    pub url: UriRelativeString,

    /// Content of the entry.
    pub content: Vec<u8>,
}

/// A script entry.
#[derive(Debug)]
pub struct Script {
    /// Input file path from which the entry comes from.
    pub path: PathBuf,

    /// URL from which the entry will be accessible.
    pub url: UriRelativeString,

    /// Content of the entry.
    pub content: String,
}

/// A style entry.
#[derive(Debug)]
pub struct Style {
    /// Input file path from which the entry comes from.
    pub path: PathBuf,

    /// URL from which the entry will be accessible.
    pub url: UriRelativeString,

    /// Content of the entry.
    pub content: String,
}

/// Site data.
#[derive(Debug, Default, Serialize)]
pub struct Site {
    /// Taxonomies.
    pub taxonomies: BTreeMap<String, BTreeMap<String, Vec<TaxonomyItem>>>,

    /// Page data.
    pub data: Value,
}

/// Taxonomy item.
#[derive(Debug, Clone, Serialize)]
pub struct TaxonomyItem {
    /// Item date.
    pub date: DateTime,

    /// Item data.
    pub data: Value,

    /// Item URL.
    pub url: UriRelativeString,
}

/// Build the site.
pub fn build(config: &Config) -> Result<Duration> {
    let start_time = Instant::now();

    std::thread::scope(|s| -> Result<()> {
        let site = Arc::new(RwLock::new(Site::default()));

        site.write().unwrap().data = config.site_data.clone();

        let (input_page_tx, metadata_page_rx) = unbounded();
        let (metadata_page_tx, markdown_page_rx) = unbounded();
        let (markdown_page_tx, taxonomies_page_rx) = unbounded();
        let (taxonomies_page_tx, languages_page_rx) = unbounded();
        let (languages_page_tx, layout_page_rx) = unbounded();
        let (layout_page_tx, assets_page_rx) = unbounded();
        let (assets_page_tx, feeds_page_rx) = unbounded();
        let (assets_file_tx, output_file_rx) = unbounded();
        let (assets_image_tx, output_image_rx) = unbounded();
        let (assets_script_tx, scripts_script_rx) = unbounded();
        let (assets_style_tx, styles_style_rx) = unbounded();
        let (feeds_page_tx, sitemap_page_rx) = unbounded();
        let (sitemap_page_tx, reload_page_rx) = unbounded();
        let (reload_page_tx, minify_page_rx) = unbounded();
        let (minify_page_tx, output_page_rx) = unbounded();
        let (scripts_script_tx, output_script_rx) = unbounded();
        let (styles_style_tx, output_style_rx) = unbounded();

        [
            {
                let taxonomies_site = site.clone();
                let layout_site = site.clone();
                let feeds_file_tx = assets_file_tx.clone();
                let sitemap_file_tx = assets_file_tx.clone();
                s.spawn(|| {
                    self::input::run(config, input_page_tx)?;
                    self::metadata::run(metadata_page_rx, metadata_page_tx)?;
                    self::markdown::run(config, markdown_page_rx, markdown_page_tx)?;
                    self::taxonomies::run(
                        config,
                        taxonomies_site,
                        taxonomies_page_rx,
                        taxonomies_page_tx,
                    )?;
                    self::languages::run(config, languages_page_rx, languages_page_tx)?;
                    self::layout::run(config, layout_site, layout_page_rx, layout_page_tx)?;
                    self::assets::run(
                        config,
                        assets_page_rx,
                        assets_page_tx,
                        assets_file_tx,
                        assets_image_tx,
                        assets_script_tx,
                        assets_style_tx,
                    )?;
                    self::feeds::run(config, feeds_page_rx, feeds_page_tx, feeds_file_tx)?;
                    self::sitemap::run(config, sitemap_page_rx, sitemap_page_tx, sitemap_file_tx)?;
                    self::reload::run(config, reload_page_rx, reload_page_tx)?;
                    self::minify::run(config, minify_page_rx, minify_page_tx)
                })
            },
            s.spawn(|| self::scripts::run(config, scripts_script_rx, scripts_script_tx)),
            s.spawn(|| self::styles::run(config, styles_style_rx, styles_style_tx)),
            s.spawn(|| {
                self::output::run(
                    config,
                    output_file_rx,
                    output_image_rx,
                    output_page_rx,
                    output_script_rx,
                    output_style_rx,
                )
            }),
        ]
        .into_iter()
        .try_for_each(|handle| handle.join().unwrap())
    })?;

    Ok(start_time.elapsed())
}
