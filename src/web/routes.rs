use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::web::AppState;

#[derive(Deserialize)]
struct HistoryParams {
    target: Option<String>,
    range: Option<i64>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/api/health", get(health))
        .route("/api/metrics/latest", get(latest))
        .route("/api/metrics/history", get(history))
        .route("/api/config", get(get_config))
        .with_state(state)
}

async fn serve_index() -> impl IntoResponse {
    let html = include_str!("../../static/index.html");
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "targets": state.cfg.targets.len(),
    }))
}

async fn latest(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let mut out = Vec::new();

    // include a "system" snapshot from the most recent system row if present
    if let Ok(Some(row)) = state.db.latest("system").await {
        out.push(json!({
            "type": "system",
            "target": "system",
            "cpu": row.cpu,
            "memory_mb": row.memory_mb,
            "disk_mb": row.disk_mb,
            "network_rx": row.network_rx,
            "network_tx": row.network_tx,
            "timestamp": row.timestamp,
        }));
    }

    for t in &state.cfg.targets {
        if let Ok(Some(row)) = state.db.latest(&t.name).await {
            out.push(json!({
                "type": "target",
                "target": row.target,
                "name": row.target,
                "status": if row.latency_ms.is_some() { "up" } else { "unknown" },
                "latency_ms": row.latency_ms,
                "http_requests": row.http_requests,
                "cpu": row.cpu,
                "memory_mb": row.memory_mb,
                "disk_mb": row.disk_mb,
                "network_rx": row.network_rx,
                "network_tx": row.network_tx,
                "timestamp": row.timestamp,
            }));
        } else {
            out.push(json!({
                "type": "target",
                "target": t.name,
                "name": t.name,
                "status": "unknown",
                "latency_ms": null,
                "http_requests": null,
            }));
        }
    }

    Json(out)
}

async fn history(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> Json<serde_json::Value> {
    let target = params.target.unwrap_or_else(|| "system".to_string());
    let range = params.range.unwrap_or(24).clamp(1, 168);

    match state.db.history(&target, range).await {
        Ok(rows) => Json(json!(rows)),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn get_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "port": state.cfg.port,
        "db_path": state.cfg.db_path,
        "interval_seconds": state.cfg.interval_seconds,
        "targets": state.cfg.targets,
    }))
}