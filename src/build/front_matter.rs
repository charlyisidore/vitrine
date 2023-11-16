//! Parse front matter data.
//!
//! A front matter is a block of metadata located at the top of a file. Front
//! matters are enclosed in delimiters which determine the format: `---` for
//! YAML, and `+++` for TOML.

use serde::de::DeserializeOwned;

use super::{Entry, Error};

/// Delimiter used for TOML front matters.
const TOML_DELIMITER: &str = "+++";

/// Delimiter used for YAML front matters.
const YAML_DELIMITER: &str = "---";

/// Extract front matter data in a [`Entry`].
///
/// Extract and deserialize the front matter from the `content` property and
/// store it in the `data` property. The front matter is removed from the
/// `content` property. When no front matter is found, `data` is `None`.
pub(super) fn parse_entry(entry: Entry) -> Result<Entry, Error> {
    let Some(content) = entry.content.as_ref() else {
        return Ok(entry);
    };

    let (content, data) = parse(content).map_err(|error| Error::ParseFrontMatter {
        input_path: entry.input_path_buf(),
        source: error,
    })?;

    Ok(Entry {
        content: Some(content),
        data,
        ..entry
    })
}

/// Extract and deserialize front matter data from a string.
///
/// Returns a tuple (`content`, `data`), where `content` is the content without
/// the front matter, and `data` is the deserialized front matter data (or
/// `None` if no front matter has been found).
fn parse<T, S>(content: S) -> Result<(String, Option<T>), anyhow::Error>
where
    T: DeserializeOwned,
    S: AsRef<str>,
{
    let content = content.as_ref();

    // Use the `std::str::Lines` trait to read the content line by line
    let mut lines = content.lines().peekable();

    // Skip if the first line does not contain exactly the delimiter
    if let Some(delimiter) = lines.next_if(|line| [TOML_DELIMITER, YAML_DELIMITER].contains(line)) {
        // Read until the second delimiter or the end
        let data = lines
            .by_ref()
            .take_while(|&line| line != delimiter)
            .collect::<Vec<&str>>()
            .join("\n");

        // Read until the end
        let content = lines.by_ref().collect::<Vec<&str>>().join("\n");

        // Choose the parser according to the delimiter
        let data = match delimiter {
            TOML_DELIMITER => Some(toml::from_str(&data)?),
            YAML_DELIMITER => Some(serde_yaml::from_str(&data)?),
            _ => None,
        };

        return Ok((content, data));
    }

    Ok((content.to_owned(), None))
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Data {
        layout: String,
    }

    #[test]
    fn no_front_matter() {
        const CONTENT: &str = concat!(
            "hello\n", //
            "---"
        );
        let (content, data) = super::parse::<Data, _>(CONTENT).unwrap();
        assert_eq!(content, "hello\n---");
        assert_eq!(data.is_none(), true);
    }

    #[test]
    fn parse_toml() {
        const CONTENT: &str = concat!(
            "+++\n",                    //
            "layout = \"post.tera\"\n", //
            "+++\n",                    //
            "hello"
        );
        let (content, data) = super::parse::<Data, _>(CONTENT).unwrap();
        assert_eq!(content, "hello");
        assert_eq!(data.unwrap().layout, "post.tera");
    }

    #[test]
    fn parse_yaml() {
        const CONTENT: &str = concat!(
            "---\n",                   //
            "layout: \"post.tera\"\n", //
            "---\n",                   //
            "hello"
        );
        let (content, data) = super::parse::<Data, _>(CONTENT).unwrap();
        assert_eq!(content, "hello");
        assert_eq!(data.unwrap().layout, "post.tera");
    }
}
