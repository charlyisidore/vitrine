//! Read values from JSON data.

use std::path::Path;

use serde::de::DeserializeOwned;
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum JsonError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Parse error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Read value from a JSON data file.
pub fn from_file<T>(path: impl AsRef<Path>) -> Result<T, JsonError>
where
    T: DeserializeOwned,
{
    from_str(std::fs::read_to_string(path)?)
}

/// Read value from a JSON data string.
pub fn from_str<T>(source: impl AsRef<str>) -> Result<T, JsonError>
where
    T: DeserializeOwned,
{
    Ok(serde_json::from_str(source.as_ref())?)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::from_str;

    #[derive(Deserialize)]
    struct Data {
        foo: String,
    }

    #[test]
    fn parse_from_str() {
        const INPUT: &str = r#"{ "foo": "bar" }"#;
        let result: Data = from_str(INPUT).unwrap();
        assert_eq!(result.foo, "bar");
    }
}
