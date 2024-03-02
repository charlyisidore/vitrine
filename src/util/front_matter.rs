//! Extract and parse front matters.
//!
//! A front matter is a block of metadata located at the top of a file.

use serde::de::DeserializeOwned;
use thiserror::Error;

/// Delimiter used for TOML front matters.
pub const TOML_DELIMITER: &str = "+++";

/// Delimiter used for YAML front matters.
pub const YAML_DELIMITER: &str = "---";

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
pub fn parse<T>(source: &str) -> Result<Option<(T, &str)>, FrontMatterError>
where
    T: DeserializeOwned,
{
    let Some((format, data, content)) = extract(source) else {
        return Ok(None);
    };

    let data = match format {
        "toml" => crate::util::eval::toml::from_str(data)?,
        "yaml" => crate::util::eval::yaml::from_str(data)?,
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
pub fn extract(source: &str) -> Option<(&str, &str, &str)> {
    // Source must start with known delimiter
    let Some(delimiter) = [TOML_DELIMITER, YAML_DELIMITER]
        .into_iter()
        .find(|delimiter| source.starts_with(delimiter))
    else {
        return None;
    };

    // Extract the rest of the first line in `format`
    let Some((format, source)) = source[delimiter.len()..].split_once('\n') else {
        return None;
    };

    // The `+++` delimiter is for TOML
    // The `---` delimiter can specify the format in the rest of the line
    let format = match delimiter {
        TOML_DELIMITER => "toml",
        YAML_DELIMITER => {
            let format = format.trim();
            if format.is_empty() {
                "yaml"
            } else {
                format
            }
        },
        _ => unreachable!("unsupported delimiter"),
    };

    // Find the second delimiter
    let Some((data, content)) = source.split_once(&format!("\n{delimiter}")) else {
        return None;
    };

    // Ignore the rest of the line
    let Some((_, content)) = content.split_once('\n') else {
        return None;
    };

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
        const INPUT: &str = concat!(
            "foo\n", //
            "---\n", //
            "bar\n", //
            "---\n", //
            "baz\n"
        );

        let result = parse::<Data>(INPUT).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn parse_toml() {
        const INPUT: &str = concat!(
            "+++\n",               //
            "layout = \"post\"\n", //
            "+++\n",               //
            "foo\n"
        );

        let (data, content) = parse::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }

    #[test]
    fn parse_toml_ignore() {
        const INPUT: &str = concat!(
            "+++yaml\n",           //
            "layout = \"post\"\n", //
            "+++\n",               //
            "foo\n"
        );

        let (data, content) = parse::<Data>(INPUT).unwrap().unwrap();

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

        let (data, content) = parse::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }

    #[test]
    fn parse_format() {
        const INPUT: &str = concat!(
            "---toml\n",           //
            "layout = \"post\"\n", //
            "---\n",               //
            "foo\n"
        );

        let (data, content) = parse::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }

    #[test]
    fn parse_format_trim() {
        const INPUT: &str = concat!(
            "---  toml  \n",       //
            "layout = \"post\"\n", //
            "---\n",               //
            "foo\n"
        );

        let (data, content) = parse::<Data>(INPUT).unwrap().unwrap();

        assert_eq!(data.layout, "post");
        assert_eq!(content, "foo\n");
    }
}
