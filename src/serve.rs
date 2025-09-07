//! Serve the site.

use std::{net::SocketAddr, path::Path};

use anyhow::Result;
use async_channel::Receiver;
use axum::{
    Router,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use log::info;
use tokio::runtime::Runtime;
use tower_http::services::ServeDir;

/// Serve the site.
///
/// This function creates a HTTP server that delivers static files.
pub fn serve(
    dir: impl AsRef<Path>,
    port: u16,
    sse_rx: Receiver<Result<Event>>,
    shutdown_rx: Receiver<()>,
) -> Result<()> {
    debug_assert!(dir.as_ref().is_absolute());

    let rt = Runtime::new()?;

    let router = Router::new()
        .route(
            "/_vitrine",
            get(async move || Sse::new(sse_rx).keep_alive(KeepAlive::default())),
        )
        .fallback_service(ServeDir::new(dir));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    info!("Listening on {}", addr);

    let listener = rt.block_on(tokio::net::TcpListener::bind(addr))?;

    Ok(rt.block_on(async {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.recv().await;
            })
            .await
    })?)
}
