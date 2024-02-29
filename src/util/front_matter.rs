//! Extract and parse front matters.
//!
//! A front matter is a block of metadata located at the top of a file.

use serde::de::DeserializeOwned;
use thiserror::Error;

/// Delimiter used for TOML front matters.
const TOML_DELIMITER: &str = "+++";

/// Delimiter used for YAML front matters.
const YAML_DELIMITER: &str = "---";

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum FrontMatterError {
    /// TOML parse error.
    #[error(transparent)]
    Toml(#[from] crate::util::eval::toml::TomlError),
    /// YAML parse error.
    #[error(transparent)]
    Yaml(#[from] crate::util::eval::yaml::YamlError),
}

/// Extract and deserialize front matter data from a string.
///
/// When a front matter is detected, this function returns a tuple `(data,
/// content)`, where `data` is the deserialized front matter data, and `content`
/// is the content without the front matter. Otherwise, it returns [`None`].
pub fn parse<T>(content: impl AsRef<str>) -> Result<Option<(T, String)>, FrontMatterError>
where
    T: DeserializeOwned,
{
    let Some((format, data, content)) = extract(content) else {
        return Ok(None);
    };

    let data = match format.as_str() {
        "toml" => crate::util::eval::toml::from_str(&data)?,
        "yaml" => crate::util::eval::yaml::from_str(&data)?,
        _ => return Ok(None),
    };

    Ok(Some((data, content)))
}

/// Extract front matter and content from a string.
///
/// When a front matter is detected, this function returns a tuple `(format,
/// data, content)`, where `format` is the expected data format, `data` is the
/// front matter string, and `content` is the content without front matter.
/// Otherwise, it returns [`None`].
pub fn extract(content: impl AsRef<str>) -> Option<(String, String, String)> {
    let content = content.as_ref();

    // Use the `std::str::Lines` trait to read the content line by line
    let mut lines = content.lines().peekable();

    // Skip if the first line does not contain exactly the delimiter
    let Some(delimiter) = lines.next_if(|line| [TOML_DELIMITER, YAML_DELIMITER].contains(line))
    else {
        return None;
    };

    // Read until the second delimiter or the end
    let data = lines
        .by_ref()
        .take_while(|&line| line != delimiter)
        .collect::<Vec<&str>>()
        .join("\n");

    // Read until the end
    let content = lines.by_ref().collect::<Vec<&str>>().join("\n");

    // Detect the format according to the delimiter
    let format = match delimiter {
        TOML_DELIMITER => "toml",
        YAML_DELIMITER => "yaml",
        _ => unreachable!(),
    }
    .to_string();

    Some((format, data, content))
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::parse;

    #[derive(Deserialize)]
    struct Data {
        layout: String,
    }

    #[test]
    fn no_front_matter() {
        const CONTENT: &str = concat!(
            "foo\n", //
            "---\n", //
            "bar\n", //
            "---\n", //
            "baz\n"
        );

        let result = parse::<Data>(CONTENT).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn parse_toml() {
        const CONTENT: &str = concat!(
            "+++\n",               //
            "layout = \"post\"\n", //
            "+++\n",               //
            "foo\n"
        );

        let (data, content) = parse::<Data>(CONTENT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo");
    }

    #[test]
    fn parse_yaml() {
        const CONTENT: &str = concat!(
            "---\n",              //
            "layout: \"post\"\n", //
            "---\n",              //
            "foo\n"
        );

        let (data, content) = parse::<Data>(CONTENT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo");
    }
}
