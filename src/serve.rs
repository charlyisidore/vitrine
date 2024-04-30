//! Serve the site.

use std::{net::SocketAddr, path::Path};

use axum::{handler::HandlerWithoutStateExt, http::StatusCode, Router};
use thiserror::Error;
use tower_http::services::ServeDir;

/// List of server errors.
#[derive(Debug, Error)]
pub enum ServeError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Show 404 error.
async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
}

/// Serve the site.
///
/// This function creates a HTTP server that delivers files from `dir`.
pub async fn serve(dir: impl AsRef<Path>, port: u16) -> Result<(), ServeError> {
    let dir = dir.as_ref();

    let serve_dir = ServeDir::new(dir);

    let router =
        Router::new().nest_service("/", serve_dir.not_found_service(handle_404.into_service()));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    Ok(axum::serve(listener, router.into_make_service()).await?)
}
