//! Ignore input files or paths.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::{Config, Error};

/// Path pattern matcher for ignored files.
pub(super) struct Matcher {
    /// Group of globs.
    glob_set: GlobSet,
}

impl Matcher {
    /// Create a path pattern matcher.
    pub(super) fn new(config: &Config) -> Result<Self, Error> {
        Ok(Self {
            glob_set: config
                .ignore
                .iter()
                .try_fold(GlobSetBuilder::new(), |mut builder, pattern| {
                    builder.add(Glob::new(pattern)?);
                    Ok(builder)
                })
                .and_then(|builder| builder.build())
                .map_err(|error| Error::NewIgnoreMatcher {
                    source: error.into(),
                })?,
        })
    }

    /// Check if a file is ignored.
    pub(super) fn is_match<P>(&self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        self.glob_set.is_match(path)
    }
}
