mod app;
pub mod controller;
pub mod cron;
pub mod repository;
mod schema;
pub mod service;

use std::{env, net::SocketAddr, sync::Arc};

use anyhow::Result;
use diesel::{SqliteConnection, r2d2::ConnectionManager, r2d2::Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tracing::info;

use app::{AppState, DbPool};
use axum::Router;
use axum::routing::{get, post};
use controller::*;
use cron::*;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "server.db".to_string());
    let bind_addr = env::var("SERVER_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let lease_timeout_secs: i64 = env::var("LEASE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let manager = ConnectionManager::<SqliteConnection>::new(db_url);
    let db_pool = Pool::builder().max_size(16).build(manager)?;
    info!(
        bind_addr = %bind_addr,
        lease_timeout_secs,
        "server configuration loaded"
    );
    run_migrations(&db_pool)?;

    let state = Arc::new(AppState {
        db_pool,
        lease_timeout_secs,
    });

    timeout_cron::spawn_timeout_job(state.clone());
    retry_cron::spawn_retry_cron(state.clone());
    let app = Router::new()
        .route("/v1/tasks/claim", post(task_controller::claim_task))
        .route(
            "/v1/tasks/result",
            post(task_controller::submit_task_result),
        )
        .route("/version", get(version_controller::handle_version))
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse()?;
    info!("server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn run_migrations(pool: &DbPool) -> Result<()> {
    let mut conn = pool.get()?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    info!("database migrations ensured");
    Ok(())
}
