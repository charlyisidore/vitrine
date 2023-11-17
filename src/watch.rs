//! Watch for file changes.

use std::{path::Path, time::Duration};

use notify_debouncer_full::{
    new_debouncer,
    notify::{EventKind, INotifyWatcher, RecursiveMode, Watcher},
    Debouncer, FileIdMap,
};

use crate::{config::Config, error::Error};

/// Watch for file changes.
///
/// Call a given function when a file has been created, modified or deleted in
/// input, data, or layout directory.
pub(super) async fn watch<F>(config: &Config, callback: F) -> Result<(), Error>
where
    F: Fn() -> Result<(), Error>,
{
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let event_handler = move |result| {
        if let Err(error) = sender.send(result) {
            tracing::error!("Error: {:?}", error);
        }
    };

    let mut debouncer =
        new_debouncer(Duration::from_secs(1), None, event_handler).map_err(|error| {
            Error::Watch {
                source: error.into(),
            }
        })?;

    add_watch_path(&mut debouncer, &config.input_dir)?;

    if let Some(data_dir) = config.data_dir.as_ref() {
        add_watch_path(&mut debouncer, data_dir)?;
    }

    if let Some(layout_dir) = config.layout_dir.as_ref() {
        add_watch_path(&mut debouncer, layout_dir)?;
    }

    tracing::info!("Watching for file changes");

    while let Some(result) = receiver.recv().await {
        match result {
            Ok(events) => {
                let events: Vec<_> = events
                    .iter()
                    .filter_map(|event| match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                            Some(event)
                        },
                        _ => None,
                    })
                    .filter(|event| {
                        // Do not watch output directory
                        config
                            .output_dir
                            .as_ref()
                            .map(|output_dir| {
                                event
                                    .paths
                                    .iter()
                                    .find(|path| !path.starts_with(output_dir))
                                    .is_some()
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

                tracing::info!("Files changed: {}", paths.join(", "));

                if let Some(error) = (callback)().err() {
                    tracing::error!("{:?}", error);
                }
            },
            Err(errors) => tracing::error!("Error: {errors:?}"),
        }
    }

    Ok(())
}

fn add_watch_path<P>(
    debouncer: &mut Debouncer<INotifyWatcher, FileIdMap>,
    path: P,
) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    tracing::debug!("Watch: {:?}", path);

    debouncer
        .watcher()
        .watch(path, RecursiveMode::Recursive)
        .map_err(|error| Error::Watch {
            source: error.into(),
        })?;

    debouncer.cache().add_root(path, RecursiveMode::Recursive);

    Ok(())
}
