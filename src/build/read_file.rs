//! Read source files.

use super::{Entry, Error};

/// Read content of a [`Entry`] from a file.
///
/// This function reads the file located at `input_file.path()` and stores its
/// content in the `content` property.
pub(super) fn read_entry(entry: Entry) -> Result<Entry, Error> {
    if let Some(input_file) = entry.input_file.as_ref() {
        let content =
            std::fs::read_to_string(input_file.path()).map_err(|error| Error::ReadInput {
                input_path: Some(input_file.path().to_owned()),
                source: error.into(),
            })?;

        return Ok(Entry {
            content: Some(content),
            ..entry
        });
    }

    Ok(entry)
}
