//! Bundle SCSS and CSS styles.
//!
//! This module uses [`grass`] and [`lightningcss`] under the hood.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::Result;
use async_channel::{Receiver, Sender};
use grass::Options;
use lightningcss::{
    bundler::{Bundler, SourceProvider},
    stylesheet::{ParserOptions, PrinterOptions},
};
use parcel_sourcemap::SourceMap;

use crate::{Config, ReceiverExt, Style};

/// Bundle SCSS and CSS styles.
pub fn run(config: &Config, style_rx: Receiver<Style>, style_tx: Sender<Style>) -> Result<()> {
    let fs = FileProvider::new(&config.input_dir);

    for style in style_rx.into_iter() {
        let mut source_map = config
            .debug
            .then(|| SourceMap::new(&config.input_dir.to_string_lossy()));

        let mut bundler = Bundler::new(&fs, source_map.as_mut(), ParserOptions::default());
        let stylesheet = bundler.bundle(&style.path).unwrap();

        let printer_options = PrinterOptions {
            minify: !config.debug,
            source_map: source_map.as_mut(),
            project_root: Some(&config.input_dir.to_string_lossy()),
            ..Default::default()
        };

        let result = stylesheet.to_css(printer_options)?;
        let mut content = result.code;

        if let Some(mut source_map) = source_map {
            let url = style.url.path_str().to_string() + ".map";

            content.push_str(&format!("\n/*# sourceMappingURL={} */\n", url));

            style_tx.send_blocking(Style {
                content: source_map.to_json(None)?,
                path: style.path.clone(),
                url: url.try_into()?,
            })?;
        }

        style_tx.send_blocking(Style { content, ..style })?;
    }

    Ok(())
}

/// Implementation of [`SourceProvider`] that caches compiled files.
struct FileProvider<'config> {
    input_dir: &'config Path,
    inputs: Mutex<HashMap<PathBuf, *mut String>>,
}

impl<'config> FileProvider<'config> {
    /// Create a new [`FileProvider`].
    pub fn new(input_dir: &'config Path) -> Self {
        FileProvider {
            input_dir,
            inputs: Mutex::new(HashMap::new()),
        }
    }
}

unsafe impl Sync for FileProvider<'_> {}
unsafe impl Send for FileProvider<'_> {}

impl SourceProvider for FileProvider<'_> {
    type Error = std::io::Error;

    fn read<'a>(&'a self, file: &Path) -> Result<&'a str, Self::Error> {
        let mut inputs = self.inputs.lock().unwrap();

        if let Some(ptr) = inputs.get(file) {
            return Ok(unsafe { &**ptr });
        }

        let source = std::fs::read_to_string(file)?;

        let source = match file.extension().and_then(|s| s.to_str()) {
            Some("scss") => {
                let load_path = file.parent().unwrap();
                let options = Options::default().load_path(load_path);
                grass::from_string(source, &options).map_err(std::io::Error::other)?
            },
            _ => source,
        };

        let ptr = Box::into_raw(Box::new(source));

        inputs.insert(file.to_path_buf(), ptr);

        // SAFETY: this is safe because the pointer is not dropped until the
        // `FileProvider` is, and we never remove from the list of pointers stored in
        // the map.
        Ok(unsafe { &*ptr })
    }

    fn resolve(&self, specifier: &str, originating_file: &Path) -> Result<PathBuf, Self::Error> {
        Ok(if specifier.starts_with('/') {
            self.input_dir.join(
                specifier
                    .trim_start_matches('/')
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .collect::<PathBuf>(),
            )
        } else {
            originating_file.with_file_name(specifier)
        })
    }
}

impl Drop for FileProvider<'_> {
    fn drop(&mut self) {
        for (_, ptr) in self.inputs.lock().unwrap().iter() {
            std::mem::drop(unsafe { Box::from_raw(*ptr) })
        }
    }
}
