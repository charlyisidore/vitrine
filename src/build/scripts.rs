//! Bundle TSX, JSX, TypeScript, JavaScript.
//!
//! This module uses [`rolldown`] under the hood.

use anyhow::{Result, anyhow};
use async_channel::{Receiver, Sender};
use rolldown::{Bundler, BundlerOptions, SourceMapType};
use tokio::runtime::Runtime;

use crate::{Config, ReceiverExt, Script};

/// Bundle JS.
pub fn run(config: &Config, script_rx: Receiver<Script>, script_tx: Sender<Script>) -> Result<()> {
    let rt = Runtime::new()?;

    for script in script_rx.into_iter() {
        let mut bundler = Bundler::new(BundlerOptions {
            input: Some(vec![script.path.to_string_lossy().to_string().into()]),
            sourcemap: config.debug.then_some(SourceMapType::File),
            minify: Some((!config.debug).into()),
            ..Default::default()
        });

        let output = rt.block_on(bundler.generate()).map_err(|batch| {
            batch
                .into_vec()
                .into_iter()
                .fold(anyhow!("encountered multiple errors"), |error, e| {
                    error.context(e)
                })
        })?;

        for asset in output.assets {
            let mut segments = script.url.path_str().split('/').collect::<Vec<_>>();
            segments.pop();
            segments.push(asset.filename());
            script_tx.send_blocking(Script {
                content: String::from_utf8_lossy(asset.content_as_bytes()).to_string(),
                url: segments.join("/").try_into()?,
                path: script.path.clone(),
            })?;
        }
    }

    Ok(())
}
