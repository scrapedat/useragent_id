use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

mod api;
mod core;
mod training;
mod wasm;

use crate::api::routes::create_api_router;
use crate::wasm::orchestrator::WasmOrchestrator;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create and initialize the WASM orchestrator
    let orchestrator = WasmOrchestrator::new()?;
    let app_state = Arc::new(orchestrator);

    // Create the API router
    let app = create_api_router(app_state).await;

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("User Automation Learning System listening on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
