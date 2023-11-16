//! Serve the site.

use std::net::SocketAddr;

use axum::{Router, Server};
use tower_http::services::{ServeDir, ServeFile};

use crate::{config::Config, error::Error};

/// Relative path to the file to send for 404 errors.
const NOT_FOUND_PATH: &str = "404/index.html";

/// Serve the site.
pub(super) async fn serve(config: &Config) -> Result<(), Error> {
    let Some(output_dir) = config.output_dir.as_ref() else {
        return Err(Error::Serve {
            source: anyhow::anyhow!("No output directory specified"),
        });
    };

    let serve_dir = ServeDir::new(output_dir)
        .not_found_service(ServeFile::new(output_dir.join(NOT_FOUND_PATH)));

    let router = Router::new().nest_service("/", serve_dir);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.serve_port));

    tracing::info!("Listening on {}", addr);

    Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .map_err(|error| Error::Serve {
            source: error.into(),
        })
}
