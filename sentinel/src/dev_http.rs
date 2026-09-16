#![cfg(feature = "dev-http")]

use crate::Sentinel;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct PerformBody {
    action: String,
    intent: String,
    cost_usd: f64,
    input: crate::input::InputAction,
}

pub async fn serve(sentinel: Arc<Sentinel>, addr: std::net::SocketAddr) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/dev/perform", post(perform))
        .route("/dev/ledger_head", get(head))
        .with_state(sentinel);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn perform(
    State(s): State<Arc<Sentinel>>,
    Json(b): Json<PerformBody>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    s.perform(&b.action, &b.intent, b.cost_usd, b.input)
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))
}

async fn head(State(s): State<Arc<Sentinel>>) -> Json<(u64, String)> {
    Json(s.ledger.head())
}
