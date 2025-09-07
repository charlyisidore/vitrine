//! Extract page metadata.

use anyhow::{Result, anyhow};
use async_channel::{Receiver, Sender};
use serde::de::DeserializeOwned;

use crate::{Page, ReceiverExt};

/// Delimiter used for TOML front matters.
const TOML_DELIMITER: &str = "+++";

/// Delimiter used for YAML front matters.
const YAML_DELIMITER: &str = "---";

/// Extract page metadata.
pub fn run(page_rx: Receiver<Page>, page_tx: Sender<Page>) -> Result<()> {
    for page in page_rx.into_iter() {
        let page = if let Some((data_path, format)) = page.file.as_ref().and_then(|file| {
            ["json", "toml", "yaml"]
                .into_iter()
                .map(|extension| (file.path().with_extension(extension), extension))
                .find(|(path, _)| path.exists())
        }) {
            let data = std::fs::read_to_string(data_path)?;
            let data = parse_data(&data, format)?;
            Page { data, ..page }
        } else if let Some((data, content)) = parse_front_matter(&page.content)? {
            Page {
                content: content.to_string(),
                data,
                ..page
            }
        } else {
            page
        };

        // Overwrite page date
        let date = page
            .data
            .get("date")
            .and_then(|v| v.as_str())
            .map(|s| s.try_into())
            .transpose()?
            .unwrap_or(page.date);

        // Overwrite page language
        let lang = page
            .data
            .get("lang")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(page.lang);

        // Overwrite page URL
        let url = page
            .data
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.try_into())
            .transpose()?
            .unwrap_or(page.url);

        page_tx.send_blocking(Page {
            date,
            url,
            lang,
            ..page
        })?;
    }

    Ok(())
}

/// Deserialize data from a string and a format.
fn parse_data<T>(data: &str, format: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    Ok(match format {
        "json" => serde_json::from_str(data)?,
        "toml" => toml::from_str(data)?,
        "yaml" => serde_norway::from_str(data)?,
        _ => return Err(anyhow!("unknown format {:?}", format)),
    })
}

/// Extract and deserialize front matter data from a string.
///
/// When a front matter is detected, this function returns a tuple `(data,
/// content)`, where:
/// - `data` is the deserialized front matter data
/// - `content` is the content without the front matter.
///
/// Otherwise, it returns [`None`].
fn parse_front_matter<T>(source: &str) -> Result<Option<(T, &str)>>
where
    T: DeserializeOwned,
{
    let Some((format, data, content)) = extract_front_matter(source) else {
        return Ok(None);
    };

    let data = parse_data(data, format)?;

    Ok(Some((data, content)))
}

/// Extract front matter and content from a string.
///
/// When a front matter is detected, this function returns a tuple `(format,
/// data, content)`, where:
/// - `format` is the expected data format.
/// - `data` is the front matter string.
/// - `content` is the content without front matter.
///
/// Otherwise, it returns [`None`].
fn extract_front_matter(source: &str) -> Option<(&str, &str, &str)> {
    // Source must start with known delimiter
    let delimiter = [TOML_DELIMITER, YAML_DELIMITER]
        .into_iter()
        .find(|delimiter| source.starts_with(delimiter))?;

    // Extract the rest of the first line in `format`
    let (format, source) = source[delimiter.len()..].split_once('\n')?;

    // Check if a format is specified, otherwise determine according to the
    // delimiter
    let format = format.trim();
    let format = if format.is_empty() {
        match delimiter {
            TOML_DELIMITER => "toml",
            YAML_DELIMITER => "yaml",
            _ => unreachable!(),
        }
    } else {
        format
    };

    // Find the second delimiter
    let data_end = source.find(&format!("\n{delimiter}"))?;

    let data = &source[..data_end + 1];
    let content = &source[data_end + 1 + delimiter.len()..];

    // The second delimiter must end with eof or new line
    let content = content
        .is_empty()
        .then_some(content)
        .or_else(|| content.strip_prefix('\n'))
        .or_else(|| content.strip_prefix("\r\n"))?;

    Some((format, data, content))
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Data {
        layout: String,
    }

    #[test]
    fn extract_no_front_matter() {
        const INPUT: &str = concat!(
            "foo\n", //
            "---\n", //
            "bar\n", //
            "---\n", //
            "baz\n"
        );

        let result = extract_front_matter(INPUT);

        assert!(result.is_none());
    }

    #[test]
    fn extract_no_second_delimiter() {
        const INPUT: &str = concat!(
            "---\n", //
            "layout: \"post\"\n"
        );

        let result = extract_front_matter(INPUT);

        assert!(result.is_none());
    }

    #[test]
    fn extract_invalid_second_delimiter() {
        const INPUT: &str = concat!(
            "---\n",              //
            "layout: \"post\"\n", //
            "----\n",             //
            "foo\n"
        );

        let result = extract_front_matter(INPUT);

        assert!(result.is_none());
    }

    #[test]
    fn extract_eof() {
        const INPUT: &str = concat!(
            "---\n",              //
            "layout: \"post\"\n", //
            "---"
        );

        let (_, data, content) = extract_front_matter(INPUT).unwrap();

        assert_eq!(data, "layout: \"post\"\n");
        assert_eq!(content, "");
    }

    #[test]
    fn extract_crlf() {
        const INPUT: &str = concat!(
            "---\r\n",              //
            "layout: \"post\"\r\n", //
            "---\r\n",              //
            "foo\r\n"
        );

        let (_, data, content) = extract_front_matter(INPUT).unwrap();

        assert_eq!(data, "layout: \"post\"\r\n");
        assert_eq!(content, "foo\r\n");
    }

    #[test]
    fn parse_toml() {
        const INPUT: &str = concat!(
            "+++\n",               //
            "layout = \"post\"\n", //
            "+++\n",               //
            "foo\n"
        );

        let (data, content) = parse_front_matter::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }

    #[test]
    fn parse_yaml() {
        const INPUT: &str = concat!(
            "---\n",              //
            "layout: \"post\"\n", //
            "---\n",              //
            "foo\n"
        );

        let (data, content) = parse_front_matter::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }

    #[test]
    fn parse_format_json() {
        const INPUT: &str = concat!(
            "---json\n",                //
            "{\n",                      //
            "  \"layout\": \"post\"\n", //
            "}\n",                      //
            "---\n",                    //
            "foo\n"
        );

        let (data, content) = parse_front_matter::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }

    #[test]
    fn parse_format_toml() {
        const INPUT: &str = concat!(
            "---toml\n",           //
            "layout = \"post\"\n", //
            "---\n",               //
            "foo\n"
        );

        let (data, content) = parse_front_matter::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }

    #[test]
    fn parse_format_yaml() {
        const INPUT: &str = concat!(
            "+++yaml\n",          //
            "layout: \"post\"\n", //
            "+++\n",              //
            "foo\n"
        );

        let (data, content) = parse_front_matter::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }

    #[test]
    fn parse_format_trim() {
        const INPUT: &str = concat!(
            "--- toml \n",         //
            "layout = \"post\"\n", //
            "---\n",               //
            "foo\n"
        );

        let (data, content) = parse_front_matter::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }
}
