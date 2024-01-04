//! Write destination files.

use super::{Config, Entry, Error};

/// Write content of a [`Entry`] to a file.
///
/// This function writes the `content` property to a file which location is
/// determined according to the `format` and `url` properties. For example, if
/// the format is `html` and the URL is `/blog`, the output file will be located
/// at `/blog/index.html`.
pub(super) fn write_entry(entry: Entry, config: &Config) -> Result<Entry, Error> {
    debug_assert!(entry.url.starts_with("/"));

    // Prepend base_url
    let url_path = format!("{}{}", config.base_url, entry.url);

    // All entry URLs should start with `/`
    let url_path = url_path.strip_prefix("/").unwrap();

    let mut output_path = config
        .output_dir
        .as_ref()
        .ok_or_else(|| Error::WriteOutput {
            output_path: "".into(),
            source: anyhow::anyhow!("Invalid output path"),
        })?
        .join(url_path);

    if entry.format == "html" {
        output_path.push("index.html")
    };

    tracing::info!("Writing {:?}", output_path);

    let output_dir = output_path.parent().ok_or_else(|| Error::WriteOutput {
        output_path: output_path.to_owned(),
        source: anyhow::anyhow!("Invalid output path: {output_path:?}"),
    })?;

    // Create directories recursively
    std::fs::create_dir_all(&output_dir).map_err(|error| Error::WriteOutput {
        output_path: output_dir.to_owned(),
        source: error.into(),
    })?;

    if let Some(content) = entry.content.as_ref() {
        // Write processed content
        std::fs::write(&output_path, &content).map_err(|error| Error::WriteOutput {
            output_path: output_path.to_owned(),
            source: error.into(),
        })?;
    } else if let Some(input_file) = entry.input_file.as_ref() {
        // Direct file copy
        std::fs::copy(input_file.path(), &output_path).map_err(|error| Error::WriteOutput {
            output_path: output_path.to_owned(),
            source: error.into(),
        })?;
    } else {
        unreachable!();
    }

    Ok(entry)
}
