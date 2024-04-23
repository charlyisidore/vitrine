//! Serve the site.

use std::{net::SocketAddr, path::Path};

use axum::Router;
use thiserror::Error;
use tower_http::services::{ServeDir, ServeFile};

/// List of server errors.
#[derive(Debug, Error)]
pub enum ServeError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Serve the site.
///
/// This function creates a HTTP server that delivers files from `dir`.
pub async fn serve(dir: impl AsRef<Path>, port: u16) -> Result<(), ServeError> {
    let dir = dir.as_ref();

    let not_found_path = dir.join("404").join("index.html");

    let serve_dir = ServeDir::new(dir);

    let router = Router::new();

    let router = if not_found_path.exists() {
        router.nest_service(
            "/",
            serve_dir.not_found_service(ServeFile::new(not_found_path)),
        )
    } else {
        router.nest_service("/", serve_dir)
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    Ok(axum::serve(listener, router.into_make_service()).await?)
}
