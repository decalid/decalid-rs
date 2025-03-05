//! API module for REST and CalDAV endpoints
//!
//! This module contains the REST and CalDAV endpoints for the application.

use crate::caldav;
use crate::db::Db;
use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;

mod well_known;
mod rest;
mod routes;

use crate::config::Config;

/// Initialize the unified router with both REST and CalDAV endpoints
pub fn init_router(config: &Config, db: Db) -> Router {
    // Wrap the Db in an Arc to allow sharing
    let db = Arc::new(db);

    // Merge the routers
    // The REST router is mounted at /api
    // The CalDAV router is mounted at /caldav
    Router::new()
    .nest("/.well-known", well_known::init_wellknown_router())
        .nest("/api", rest::init_rest_router(config, db.clone()))
        .nest("/caldav", caldav::server::register_routes(db))
        .route("/", get(http_index))
}

async fn http_index() -> &'static str {
    "Congratulations, you found me."
}

/// Start the unified API server
pub async fn start_server(config: &Config, db: Db) -> anyhow::Result<()> {
    let app = init_router(&config, db);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind((config.server.host.as_str(), config.server.port)).await?;

    println!("Server listening on {}", addr);
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
