//! Walk input directory.
//!
//! This module uses [`ignore`] under the hood.

use std::{
    collections::HashSet,
    path::{Component, Path},
};

use anyhow::Result;
use async_channel::Sender;
use ignore::WalkBuilder;
use iri_string::types::UriRelativeString;

use crate::{Config, Page};

/// Walk input directory.
pub fn run(config: &Config, page_tx: Sender<Page>) -> Result<()> {
    debug_assert!(config.input_dir.is_absolute());
    debug_assert!(config.output_dir.is_absolute());
    debug_assert!(
        config
            .layout_dir
            .as_ref()
            .is_none_or(|path| path.is_absolute())
    );

    let ignore_paths: HashSet<_> = config
        .ignore_paths
        .iter()
        .chain(
            [Some(&config.output_dir), config.layout_dir.as_ref()]
                .into_iter()
                .flatten(),
        )
        .cloned()
        .collect();

    debug_assert!(ignore_paths.iter().all(|path| path.is_absolute()));

    let entries = WalkBuilder::new(&config.input_dir)
        .filter_entry(move |entry| {
            debug_assert!(entry.path().is_absolute());
            !entry.file_name().to_string_lossy().starts_with('_')
                && !ignore_paths.contains(entry.path())
        })
        .build()
        .filter_map(|result| result.ok())
        .filter(|entry| {
            entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
        })
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| ["html", "md"].contains(&extension))
        });

    for entry in entries {
        let path = entry.path().canonicalize()?;
        let markup = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_string())
            .unwrap_or_default();
        let content = std::fs::read_to_string(&path)?;
        let date = entry.metadata()?.modified()?.into();
        let (url, lang) = page_url_lang(path.strip_prefix(&config.input_dir)?)?;

        page_tx.send_blocking(Page {
            file: Some(entry),
            markup,
            content,
            date,
            lang,
            url,
            languages: Default::default(),
            data: Default::default(),
        })?;
    }

    Ok(())
}

/// Create a page URL from given relative path.
fn page_url_lang(path: &Path) -> Result<(UriRelativeString, Option<String>)> {
    // Remove extension:
    // `dir/index.md` -> `dir/index`
    // `dir/page.eo.md` -> `dir/page.eo`
    let path = path.with_extension("");

    // Extract lang:
    // `dir/index` -> (`dir/index`, None)
    // `dir/page.eo` -> (`dir/page`, Some(`eo`))
    let (path, lang) = path
        .extension()
        .map(|s| {
            (
                path.with_extension(""),
                Some(s.to_string_lossy().to_string()),
            )
        })
        .unwrap_or((path, Default::default()));

    let path = if path.file_name().is_some_and(|s| s == "index") {
        // Take directory
        // `dir/index` -> `dir`
        // `index` -> ``
        path.parent().unwrap().to_path_buf()
    } else {
        path
    };

    // Empty path
    if path.components().next().is_none() {
        return Ok(("/".try_into()?, lang));
    }

    let url = path
        .components()
        .fold(String::new(), |mut url, component| match component {
            Component::Normal(segment) => {
                url.push('/');
                url.push_str(&segment.to_string_lossy());
                url
            },
            _ => url,
        })
        .try_into()?;

    Ok((url, lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url() {
        const CASES: [(&str, &str, Option<&str>); 16] = [
            ("index", "/", None),
            ("index.md", "/", None),
            ("index.eo.md", "/", Some("eo")),
            ("index.foo.eo.md", "/index.foo", Some("eo")),
            ("foo", "/foo", None),
            ("foo.md", "/foo", None),
            ("foo.eo.md", "/foo", Some("eo")),
            ("foo.bar.eo.md", "/foo.bar", Some("eo")),
            ("foo/index", "/foo", None),
            ("foo/index.md", "/foo", None),
            ("foo/index.eo.md", "/foo", Some("eo")),
            ("foo/index.foo.eo.md", "/foo/index.foo", Some("eo")),
            ("foo/bar", "/foo/bar", None),
            ("foo/bar.md", "/foo/bar", None),
            ("foo/bar.eo.md", "/foo/bar", Some("eo")),
            ("foo/bar.baz.eo.md", "/foo/bar.baz", Some("eo")),
        ];

        for (input, expected_url, expected_lang) in CASES {
            let (url, lang) = page_url_lang(&Path::new(input)).unwrap();
            assert_eq!(expected_url, url, "{:?}", input);
            assert_eq!(
                expected_lang,
                lang.as_ref().map(|s| s.as_str()),
                "{:?}",
                input
            );
        }
    }
}
