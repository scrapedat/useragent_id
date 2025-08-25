use crate::{
    core::types::{AgentSpec, DOMEvent},
    training::recorder::TaskRecorder,
    wasm::orchestrator::WasmOrchestrator
};
use serde_json::Value;
use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};

pub async fn create_api_router(orchestrator: Arc<WasmOrchestrator>) -> Router {
    Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/agents", post(deploy_agent_handler))
        .route("/learn_and_execute", post(learn_and_execute_handler))
        .with_state(orchestrator)
}

async fn deploy_agent_handler(
    State(state): State<Arc<WasmOrchestrator>>, 
    Json(payload): Json<AgentSpec>
) -> StatusCode {
    match state.deploy_agent(payload).await {
        Ok(_) => StatusCode::CREATED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn learn_and_execute_handler(
    State(orchestrator): State<Arc<WasmOrchestrator>>,
    Json(events): Json<Vec<DOMEvent>>, // Simulate receiving a recorded session
) -> (StatusCode, Json<Value>) {
    let mut recorder = TaskRecorder::new();
    for event in events {
        recorder.record_event(event);
    }
    
    let plan = recorder.generate_training_plan();
    let orchestrator = orchestrator.as_ref();
    match orchestrator.execute_training_plan(plan).await {
        Ok(results) => (StatusCode::OK, Json(results)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR, 
            Json(serde_json::json!({"error": e.to_string()}))
        ),
    }
}