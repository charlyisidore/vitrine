//! Watch for file changes.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc::channel,
};

use anyhow::Result;
use notify::{EventKind, RecursiveMode, Watcher};

/// Watch for file changes.
///
/// Call a given function when a file has been created, modified or deleted in
/// input directories.
pub fn watch(
    watch_paths: impl IntoIterator<Item = impl AsRef<Path>>,
    ignore_paths: impl IntoIterator<Item = impl AsRef<Path>>,
    callback: impl Fn(HashSet<PathBuf>) -> Result<bool>,
) -> Result<()> {
    let (tx, rx) = channel();

    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })?;

    watch_paths
        .into_iter()
        .inspect(|path| debug_assert!(path.as_ref().is_absolute()))
        .try_for_each(|path| watcher.watch(path.as_ref(), RecursiveMode::Recursive))?;

    let ignore_paths: HashSet<_> = ignore_paths
        .into_iter()
        .inspect(|path| debug_assert!(path.as_ref().is_absolute()))
        .map(|path| path.as_ref().to_path_buf())
        .collect();

    loop {
        let event = rx.recv()?;

        std::thread::sleep(std::time::Duration::from_millis(500));

        let paths = std::iter::once(event)
            .chain(rx.try_iter())
            .map(|result| result.map_err(Into::into))
            .collect::<Result<HashSet<_>>>()?
            .into_iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    EventKind::Create(..) | EventKind::Modify(..) | EventKind::Remove(..)
                )
            })
            .flat_map(|event| event.paths)
            .filter(|path| {
                !ignore_paths
                    .iter()
                    .any(|ignore_path| path.starts_with(ignore_path))
            })
            .collect::<HashSet<_>>();

        if !paths.is_empty() && (callback)(paths)? {
            break;
        }
    }

    Ok(())
}
