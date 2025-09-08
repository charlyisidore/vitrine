//! Configure the site builder.

#[cfg(feature = "deno")]
pub mod deno;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    thread::JoinHandle,
};

use anyhow::{Result, anyhow};

use crate::{PathExt, UriRelativeString, Value, cli::Opts};

/// Configuration for the site builder.
#[derive(Default)]
pub struct Config {
    /// Base URL of the site.
    pub base_url: String,

    /// Directory of input files.
    pub input_dir: PathBuf,

    /// Directory of output files.
    pub output_dir: PathBuf,

    /// Directory of layout files.
    ///
    /// If set to `None`, no layout is rendered.
    pub layout_dir: Option<PathBuf>,

    /// Paths of files to ignore.
    pub ignore_paths: HashSet<PathBuf>,

    /// Paths of files to copy.
    pub copy_paths: HashMap<PathBuf, UriRelativeString>,

    /// Site data.
    pub site_data: Value,

    /// Taxonomies.
    pub taxonomies: Vec<String>,

    /// Default language.
    pub default_lang: Option<String>,

    /// Markdown plugin names.
    pub markdown_plugins: Vec<String>,

    /// Custom Markdown render function.
    pub markdown_render: Option<Box<dyn Fn(String) -> Result<String> + Send + Sync>>,

    /// Custom layout render function.
    #[allow(clippy::type_complexity)]
    pub layout_render: Option<Box<dyn Fn(String, Value) -> Result<String> + Send + Sync>>,

    /// Custom layout filters.
    #[allow(clippy::type_complexity)]
    pub layout_filters:
        HashMap<String, Arc<dyn Fn(Value, Vec<Value>) -> Result<Value> + Send + Sync>>,

    /// Custom layout functions.
    #[allow(clippy::type_complexity)]
    pub layout_functions: HashMap<String, Arc<dyn Fn(Vec<Value>) -> Result<Value> + Send + Sync>>,

    /// Custom layout texts.
    #[allow(clippy::type_complexity)]
    pub layout_tests: HashMap<String, Arc<dyn Fn(Value, Vec<Value>) -> Result<bool> + Send + Sync>>,

    /// Feeds configuration.
    pub feeds: Vec<FeedConfig>,

    /// Sitemap generation configuration.
    pub sitemap: Option<SitemapConfig>,

    /// Debug mode.
    pub debug: bool,
}

/// Configuration for feed generation.
pub struct FeedConfig {
    /// URL of the feed.
    pub url: UriRelativeString,

    /// Authors of the feed.
    pub author: Vec<FeedPersonConfig>,

    /// Categories of the feed.
    pub category: Vec<String>,

    /// Contributors of the feed.
    pub contributor: Vec<FeedPersonConfig>,

    /// Generator of the feed.
    pub generator: Option<String>,

    /// Image that provides iconic visual identification for the feed.
    pub icon: Option<String>,

    /// Unique identifier of the feed.
    pub id: Option<String>,

    /// Image that provides visual identification for the feed.
    pub logo: Option<String>,

    /// Information about rights held in and over the feed.
    pub rights: Option<String>,

    /// Description or subtitle for the feed.
    pub subtitle: Option<String>,

    /// Title for the feed.
    pub title: String,

    /// The most recent instant in time when the feed was modified.
    pub updated: Option<String>,

    /// Predicate that determines whether a page belongs to the feed or not.
    pub filter: Option<Box<dyn Fn(Value) -> Result<bool> + Send + Sync>>,
}

/// Configuration for feed persons (author or contributor).
pub struct FeedPersonConfig {
    /// Person name.
    pub name: String,

    /// Person website.
    pub uri: Option<String>,

    /// Person email.
    pub email: Option<String>,
}

/// Configuration for sitemap generation.
#[derive(Default)]
pub struct SitemapConfig {
    /// Default page change frequency.
    pub changefreq: Option<String>,

    /// Default priority.
    pub priority: Option<f64>,

    /// Domain to prepend to URLs, if `base_url` does not specify it.
    pub url_prefix: String,

    /// URL of the sitemap.
    pub url: Option<UriRelativeString>,
}

impl Config {
    /// Create a configuration from a [`Opts`] object.
    ///
    /// Returns a tuple containing a [`Config`] object. When a configuration
    /// file has been read, returns its path and a thread handle.
    #[allow(clippy::type_complexity)]
    pub fn from_opts(opts: &Opts) -> Result<(Config, Option<(PathBuf, JoinHandle<Result<()>>)>)> {
        let config = Self {
            input_dir: opts.input.clone().unwrap_or_else(|| ".".into()),
            output_dir: opts.output.clone().unwrap_or_else(|| "_site".into()),
            base_url: opts.base_url.clone().unwrap_or_default(),
            layout_dir: Some(PathBuf::from("_layouts")).filter(|path| path.exists()),
            ..Default::default()
        };

        let Some(config_path) = opts.config.clone().or_else(|| {
            ["vitrine.config.ts", "vitrine.config.js"]
                .into_iter()
                .map(PathBuf::from)
                .find(|path| path.exists())
        }) else {
            let config = Self::normalize(config)?;

            config.check()?;

            return Ok((config, None));
        };

        let config_path = config_path.canonicalize()?;

        let (config, handle) = crate::config::deno::from_path(&config_path, config)?;

        let config = Self::normalize(config)?;

        // Prevent overwriting config file
        if config_path.starts_with(&config.output_dir) {
            return Err(anyhow!(
                "configuration file must be located outside `output_dir`",
            ));
        }

        config.check()?;

        Ok((config, Some((config_path, handle))))
    }

    /// Normalize configuration.
    ///
    /// Canonicalize all paths.
    pub fn normalize(self) -> Result<Self> {
        let current_dir = std::env::current_dir()?;

        let config = Self {
            input_dir: self.input_dir.canonicalize()?,
            output_dir: current_dir.join(self.output_dir).normalize(),
            layout_dir: self
                .layout_dir
                .map(|path| path.canonicalize())
                .transpose()?,
            ..self
        };

        let config = Self {
            ignore_paths: config
                .ignore_paths
                .into_iter()
                .map(|path| current_dir.join(path).normalize())
                .collect(),
            copy_paths: config
                .copy_paths
                .into_iter()
                .map(|(path, url)| Ok((path.canonicalize()?, url)))
                .collect::<Result<_>>()?,
            ..config
        };

        Ok(config)
    }

    /// Check if configuration is valid.
    pub fn check(&self) -> Result<()> {
        debug_assert!(self.input_dir.is_absolute());
        debug_assert!(self.output_dir.is_absolute());
        debug_assert!(
            self.layout_dir
                .as_ref()
                .is_none_or(|path| path.is_absolute())
        );
        debug_assert!(self.ignore_paths.iter().all(|path| path.is_absolute()));
        debug_assert!(self.copy_paths.keys().all(|path| path.is_absolute()));

        // Prevent overwriting input files
        if self.input_dir.starts_with(&self.output_dir) {
            return Err(anyhow!("`input_dir` must be located outside `output_dir`"));
        }

        // Prevent overwriting layout files
        if self
            .layout_dir
            .as_ref()
            .is_some_and(|path| path.starts_with(&self.output_dir))
        {
            return Err(anyhow!("`layout_dir` must be located outside `output_dir`"));
        }

        // Prevent
        for path in self.copy_paths.keys() {
            if !path.starts_with(&self.input_dir) {
                return Err(anyhow!(
                    "copied file {:?} must be located inside `input_dir`",
                    path
                ));
            }

            if path.starts_with(&self.output_dir) {
                return Err(anyhow!(
                    "copied file {:?} must be located outside `output_dir`",
                    path
                ));
            }
        }

        Ok(())
    }
}
