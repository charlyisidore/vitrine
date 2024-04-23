//! Watch for file changes.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use notify_debouncer_full::{
    new_debouncer,
    notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher},
    Debouncer, FileIdMap,
};
use thiserror::Error;

use crate::config::Config;

/// List of watcher errors.
#[derive(Debug, Error)]
pub enum WatchError {
    /// Boxed error.
    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error + Send + Sync>),
    /// Notify error.
    #[error(transparent)]
    Notify(#[from] notify_debouncer_full::notify::Error),
}

/// Watch for file changes.
///
/// Call a given function when a file has been created, modified or deleted in
/// input directories.
pub async fn watch<F>(config: &Config, callback: F) -> Result<(), WatchError>
where
    F: Fn() -> Result<(), WatchError>,
{
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let event_handler = move |result| {
        if let Err(error) = sender.send(result) {
            eprintln!("Error: {:?}", error);
        }
    };

    let mut debouncer = new_debouncer(Duration::from_secs(1), None, event_handler)?;

    add_watch_path(&mut debouncer, &config.input_dir)?;

    if let Some(layout_dir) = &config.layout_dir {
        add_watch_path(&mut debouncer, layout_dir)?;
    }

    println!("Watching for file changes");

    let mut last_callback_time = Instant::now();

    while let Some(result) = receiver.recv().await {
        match result {
            Ok(events) => {
                let events: Vec<_> = events
                    .iter()
                    .filter(|event| event.time > last_callback_time)
                    .filter(|event| {
                        matches!(
                            event.kind,
                            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                        )
                    })
                    .filter(|event| {
                        // Do not watch output directory
                        config
                            .output_dir
                            .as_ref()
                            .map(|output_dir| {
                                event.paths.iter().any(|path| !path.starts_with(output_dir))
                            })
                            .unwrap_or(true)
                    })
                    .collect();

                if events.is_empty() {
                    continue;
                }

                let mut paths: Vec<_> = events
                    .iter()
                    .flat_map(|event| &event.paths)
                    .filter_map(|path| path.to_str())
                    .collect();

                paths.sort();
                paths.dedup();

                println!("Files changed: {}", paths.join(", "));

                last_callback_time = Instant::now();

                if let Some(error) = (callback)().err() {
                    eprintln!("{:?}", error);
                }
            },
            Err(errors) => eprintln!("Error: {errors:?}"),
        }
    }

    Ok(())
}

fn add_watch_path(
    debouncer: &mut Debouncer<RecommendedWatcher, FileIdMap>,
    path: impl AsRef<Path>,
) -> Result<(), WatchError> {
    let path = path.as_ref();

    debouncer.watcher().watch(path, RecursiveMode::Recursive)?;

    debouncer.cache().add_root(path, RecursiveMode::Recursive);

    Ok(())
}
