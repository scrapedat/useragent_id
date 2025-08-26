use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::{net::SocketAddr, time::Instant};
use chrono::Utc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    ts: String,
}

#[derive(Serialize)]
struct Metrics {
    version: &'static str,
    build: &'static str,
    uptime_s: u64,
    pending_feedback: u64,
    failed_feedback: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let started = Instant::now();

    let app = Router::new()
        .route("/health", get({
            move || {
                async move {
                    Json(Health { status: "ok", ts: Utc::now().to_rfc3339() })
                }
            }
        }))
        .route("/metrics", get({
            let started = started.clone();
            move || async move {
                let uptime = started.elapsed().as_secs();
                Json(Metrics {
                    version: env!("CARGO_PKG_VERSION"),
                    build: env!("CARGO_PKG_NAME"),
                    uptime_s: uptime,
                    // Wire real values later; placeholders for now.
                    pending_feedback: 0,
                    failed_feedback: 0,
                })
            }
        }));

    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
