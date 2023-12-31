//! Normalize URLs.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use super::{Config, Entry, Error};

/// List of elements and their attributes containing URLs.
///
/// See <https://html.spec.whatwg.org/multipage/indices.html#attributes-3>.
const ELEMENTS_URL_ATTRIBUTES: [(&str, &str); 18] = [
    ("blockquote", "cite"),
    ("del", "cite"),
    ("ins", "cite"),
    ("q", "cite"),
    ("object", "data"),
    ("a", "href"),
    ("area", "href"),
    ("link", "href"),
    ("video", "poster"),
    ("audio", "src"),
    ("embed", "src"),
    ("iframe", "src"),
    ("img", "src"),
    ("input", "src"),
    ("script", "src"),
    ("source", "src"),
    ("track", "src"),
    ("video", "src"),
];

/// Normalize the URL of a [`Entry`].
///
/// By default, the URL of a build entry is determined by the path relative to
/// the input directory, e.g. `/path/to/page.md`. Assuming that the output file
/// will be rendered as HTML, this function normalizes the `url` property to
/// make it ["canonical"][w3], by removing `.html` extensions and `index.html`
/// components, e.g. `/path/to/page.html` becomes `/path/to/page`;
/// `/blog/index.html` becomes `/blog`.
///
/// The URL can be overriden by specifying the `url` field in the metadata (e.g.
/// front matter).
///
/// [w3]: https://www.w3.org/Provider/Style/URI
pub(super) fn normalize_entry(entry: Entry) -> Result<Entry, Error> {
    let url = entry
        .data
        .as_ref()
        .and_then(|data| data.url.to_owned())
        .map(|url| {
            if url.starts_with("/") {
                Ok(url)
            } else {
                Err(Error::NormalizeUrl {
                    input_path: entry.input_path_buf(),
                    source: anyhow::anyhow!("URL must start with /"),
                })
            }
        })
        .transpose()?
        .unwrap_or_else(|| normalize_url(entry.url));

    Ok(Entry { url, ..entry })
}

/// Replace local paths by web URLs.
///
/// Source files can link to other files using local paths (e.g.
/// `./other-page.md`). This function replaces these paths by web URLs (e.g.
/// `/path/to/other-page`) in the HTML code. Supported attributes are specified
/// in [`ELEMENTS_URL_ATTRIBUTES`].
pub(super) fn rewrite_url_entries(
    entries: impl Iterator<Item = Result<Entry, Error>>,
    config: &Config,
) -> Result<impl Iterator<Item = Result<Entry, Error>>, Error> {
    let base_url = config.base_url.to_owned();
    let entries: Vec<_> = entries.collect::<Result<_, _>>()?;

    // Mapping from element tag names to their attributes containing URLs
    let elements_url_attributes: HashMap<&str, HashSet<&str>> = ELEMENTS_URL_ATTRIBUTES
        .iter()
        .fold(HashMap::new(), |mut map, (tag_name, attribute)| {
            let attributes = map.entry(tag_name).or_default();
            attributes.insert(attribute);
            map
        });

    // Create a selector to match all elements from `ELEMENTS_URL_ATTRIBUTES`
    let selector = ELEMENTS_URL_ATTRIBUTES
        .iter()
        .map(|(tag_name, attribute)| format!("{tag_name}[{attribute}]"))
        .collect::<Vec<_>>()
        .join(",");

    // Create a mapping from absolute input paths to output URLs
    let urls: HashMap<PathBuf, String> = entries
        .iter()
        .filter_map(|entry| {
            entry
                .input_path()
                .and_then(|path| path.canonicalize().ok())
                .map(|path| (path, format!("{}{}", base_url, entry.url)))
        })
        .collect();

    let entries = entries.into_iter().map(move |entry| {
        // Skip non-HTML files
        if entry.format != "html" {
            return Ok(entry);
        }

        let Some(input_path) = entry.input_path() else {
            return Ok(entry);
        };

        let Some(content) = entry.content.as_ref() else {
            return Ok(entry);
        };

        let dir = input_path.parent().unwrap();

        let content = lol_html::rewrite_str(&content, lol_html::RewriteStrSettings {
            element_content_handlers: vec![lol_html::element!(selector, |element| {
                let Some(attributes) = elements_url_attributes.get(element.tag_name().as_str())
                else {
                    return Ok(());
                };

                for attribute in attributes {
                    if let Some(url) = element
                        .get_attribute(attribute)
                        .and_then(|href| {
                            let href = href.trim();
                            (!href.starts_with("/") && !href.contains("://"))
                                .then(|| dir.join(href))
                        })
                        .and_then(|path| path.canonicalize().ok())
                        .and_then(|path| urls.get(&path))
                    {
                        element.set_attribute(attribute, &url)?;
                    }
                }
                Ok(())
            })],
            ..lol_html::RewriteStrSettings::default()
        })
        .map_err(|error| Error::RewriteUrl {
            input_path: Some(input_path.to_owned()),
            source: error.into(),
        })?;

        Ok(Entry {
            content: Some(content),
            ..entry
        })
    });

    Ok(entries)
}

/// Normalize a URL string.
fn normalize_url<S>(url: S) -> String
where
    S: AsRef<str>,
{
    let url = url.as_ref();

    let url = if let Some((dir, file_name)) = url.rsplit_once('/') {
        if let Some((stem, _extension)) = file_name.rsplit_once('.') {
            if stem == "index" {
                // `/index.{ext}` -> `/`
                // `/dir/index.{ext}` -> `/dir`
                if dir.is_empty() { "/" } else { dir }.to_owned()
            } else {
                // `/dir/page.{ext}` -> `/dir/page`
                [dir, stem].join("/")
            }
        } else {
            // `/dir/page` -> `/dir/page`
            url.to_owned()
        }
    } else {
        unreachable!("URL must start with /")
    };

    debug_assert!(url.starts_with("/"));

    url
}

#[cfg(test)]
mod tests {
    #[test]
    fn normalize_url() {
        const CASES: [(&str, &str); 5] = [
            ("/index.md", "/"),
            ("/blog.md", "/blog"),
            ("/blog/index.md", "/blog"),
            ("/blog/1970-01-01-hello.md", "/blog/1970-01-01-hello"),
            ("/eo/blog/index.md", "/eo/blog"),
        ];

        for (input, expected) in CASES {
            let result = super::normalize_url(input);
            assert_eq!(
                result.as_str(),
                expected,
                "\nnormalize_url({input:?}) expected {expected:?} but received {result:?}"
            );
        }
    }
}
