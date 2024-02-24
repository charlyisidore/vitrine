//! Read values from TOML data.

use std::path::Path;

use serde::de::DeserializeOwned;
use thiserror::Error;

/// List of errors for this module.
#[derive(Debug, Error)]
pub enum TomlError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

/// Read value from a TOML data file.
pub fn from_file<T, P>(path: P) -> Result<T, TomlError>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let s = std::fs::read_to_string(path)?;
    from_str(s)
}

/// Read value from a TOML data string.
pub fn from_str<T, S>(s: S) -> Result<T, TomlError>
where
    T: DeserializeOwned,
    S: AsRef<str>,
{
    let s = s.as_ref();
    Ok(toml::from_str(s)?)
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
        const INPUT: &str = r#"foo = "bar""#;
        let result: Data = from_str(INPUT).unwrap();
        assert_eq!(result.foo, "bar");
    }
}
