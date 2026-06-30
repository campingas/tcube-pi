use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::config::AdminConfig;
use crate::db::admin::schema;

pub mod handler;
pub mod pages;
pub mod routes;

pub async fn run(config: AdminConfig) -> Result<()> {
    schema::open_admin_database(&config).context("failed to initialize Pi admin database")?;
    let state = Arc::new(config);
    let addr: SocketAddr = state
        .bind
        .parse()
        .with_context(|| format!("invalid Pi admin bind address {}", state.bind))?;
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind Pi admin service at {}", state.bind))?;
    println!("T-Cube Pi admin service listening at http://{}", state.bind);

    let app = routes::router()
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    axum::serve(listener, app)
        .await
        .context("Pi admin service failed")
}
