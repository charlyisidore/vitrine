//! Write output files.

use std::{
    fs::{copy, create_dir_all, read_dir, remove_dir_all, write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use async_channel::Receiver;
use log::{info, trace};

use crate::{
    Config, File, FileContent, Image, Page, ReceiverExt, Script, Style, UriRelativeString,
};

/// Output entry.
struct Output {
    /// Output path.
    path: PathBuf,

    /// Output content.
    content: FileContent,
}

/// Write output files.
pub fn run(
    config: &Config,
    file_rx: Receiver<File>,
    image_rx: Receiver<Image>,
    page_rx: Receiver<Page>,
    script_rx: Receiver<Script>,
    style_rx: Receiver<Style>,
) -> Result<()> {
    let output_dir = &config.output_dir;

    assert!(output_dir.is_absolute());

    let outputs: Vec<_> = file_rx
        .into_iter()
        .map(|file| Output {
            path: output_dir.join(file_path(file.url)),
            content: file.content,
        })
        .chain(image_rx.into_iter().map(|image| Output {
            path: output_dir.join(file_path(image.url)),
            content: if image.content.is_empty() {
                FileContent::Path(image.path)
            } else {
                FileContent::Bytes(image.content)
            },
        }))
        .chain(page_rx.into_iter().map(|page| Output {
            path: output_dir.join(page_path(page.url)),
            content: FileContent::String(page.content),
        }))
        .chain(script_rx.into_iter().map(|script| Output {
            path: output_dir.join(file_path(script.url)),
            content: FileContent::String(script.content),
        }))
        .chain(style_rx.into_iter().map(|style| Output {
            path: output_dir.join(file_path(style.url)),
            content: FileContent::String(style.content),
        }))
        .chain(config.copy_paths.iter().map(|(from, to)| Output {
            path: output_dir.join(file_path(to.clone())),
            content: FileContent::Path(from.clone()),
        }))
        .collect();

    if output_dir.exists() {
        remove_dir_all(output_dir)?;
    }

    for output in outputs {
        assert!(output.path.starts_with(&config.output_dir));

        let dir = output.path.parent().unwrap();

        assert!(dir.starts_with(&config.output_dir));

        trace!("create_dir_all({:?})", dir);
        create_dir_all(dir)?;

        match output.content {
            FileContent::Path(path) => {
                info!("Copy {:?} to {:?}", path, output.path);
                copy_all(path, output.path)?;
            },
            FileContent::String(contents) => {
                info!("Write {:?}", output.path);
                write(output.path, contents)?;
            },
            FileContent::Bytes(contents) => {
                info!("Write {:?}", output.path);
                write(output.path, contents)?;
            },
        }
    }

    Ok(())
}

/// Transform a page URL into a file path.
fn page_path(url: UriRelativeString) -> String {
    let mut url = url.to_string();

    let url = if url.ends_with('/') {
        url.push_str("index.html");
        url
    } else if !url.ends_with(".html") {
        url.push_str("/index.html");
        url
    } else {
        url
    };

    url.strip_prefix("/").unwrap().to_string()
}

/// Transform a file URL into a file path.
fn file_path(url: UriRelativeString) -> String {
    url.to_string().strip_prefix("/").unwrap().to_string()
}

/// Copy a file or directory recursively.
fn copy_all(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let (from, to) = (from.as_ref(), to.as_ref());

    if from.is_dir() {
        trace!("create_dir_all({:?})", to);
        create_dir_all(to)?;

        for entry in read_dir(from)? {
            let entry = entry?;
            copy_all(entry.path(), to.join(entry.file_name()))?;
        }
    } else {
        trace!("copy({:?}, {:?})", from, to);
        copy(from, to)?;
    }

    Ok(())
}
