use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::web::AppState;

#[derive(Deserialize)]
struct HistoryParams {
    target: Option<String>,
    range: Option<i64>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/static/vendor/chart.umd.min.js", get(serve_chart_js))
        .route("/api/health", get(health))
        .route("/api/metrics/latest", get(latest))
        .route("/api/metrics/history", get(history))
        .route("/api/config", get(get_config))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn security_headers(req: axum::http::Request<axum::body::Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' data:; img-src 'self' data:; connect-src 'self'; font-src 'self' data:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'",
        ),
    );
    res
}

async fn serve_index() -> impl IntoResponse {
    Html(include_str!("../../static/index.html"))
}

async fn serve_chart_js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../../static/vendor/chart.umd.min.js"),
    )
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let runtime = state.runtime.read().await.clone();
    let db_ok = state.db.ping().await.is_ok();
    let shutting_down = state.is_shutting_down();
    let ready = db_ok && !shutting_down && runtime.last_collection_ok.unwrap_or(false);

    let status = if ready {
        "ok"
    } else if db_ok && !shutting_down {
        "starting"
    } else {
        "degraded"
    };

    let http = if ready || status == "starting" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        http,
        Json(json!({
            "status": status,
            "ready": ready,
            "version": env!("CARGO_PKG_VERSION"),
            "db": { "ok": db_ok },
            "shutting_down": shutting_down,
            "runtime": runtime,
        })),
    )
}

async fn latest(State(state): State<AppState>) -> impl IntoResponse {
    let mut system = Value::Null;
    match state.db.latest_system().await {
        Ok(Some(row)) => {
            system = json!({
                "type": "system",
                "timestamp": row.timestamp,
                "cpu_percent": row.cpu_percent,
                "mem_used_mb": row.mem_used_mb,
                "mem_total_mb": row.mem_total_mb,
                "mem_used_percent": row.mem_used_percent,
                "disk_used_mb": row.disk_used_mb,
                "disk_total_mb": row.disk_total_mb,
                "disk_available_mb": row.disk_available_mb,
                "disk_used_percent": row.disk_used_percent,
                "net_rx_bytes": row.net_rx_bytes,
                "net_tx_bytes": row.net_tx_bytes,
                "net_rx_bytes_per_sec": row.net_rx_bytes_per_sec,
                "net_tx_bytes_per_sec": row.net_tx_bytes_per_sec,
            });
        }
        Ok(None) => {}
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_error",
                &e.to_string(),
            );
        }
    }

    let mut targets = Vec::with_capacity(state.cfg.targets.len());
    for t in &state.cfg.targets {
        match state.db.latest_target(&t.name).await {
            Ok(Some(row)) => targets.push(json!({
                "type": "target",
                "name": row.target,
                "kind": row.kind,
                "url": row.url,
                "status": row.status,
                "latency_ms": row.latency_ms,
                "http_requests": row.http_requests,
                "detail": row.detail,
                "timestamp": row.timestamp,
            })),
            Ok(None) => targets.push(json!({
                "type": "target",
                "name": t.name,
                "kind": t.kind.clone().unwrap_or_else(|| "generic".to_string()),
                "url": t.url,
                "status": "unknown",
                "latency_ms": Value::Null,
                "http_requests": Value::Null,
                "detail": "no data yet",
                "timestamp": Value::Null,
            })),
            Err(e) => {
                return api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "db_error",
                    &e.to_string(),
                );
            }
        }
    }

    (
        StatusCode::OK,
        Json(json!({ "system": system, "targets": targets })),
    )
        .into_response()
}

async fn history(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> impl IntoResponse {
    let target = params.target.unwrap_or_else(|| "system".to_string());
    let max_range = state
        .cfg
        .max_history_hours
        .max(1)
        .min(state.cfg.retention_hours as i64);
    let range = params.range.unwrap_or(24).clamp(1, max_range);

    if target.trim().is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "invalid_target",
            "target must not be empty",
        );
    }

    if target == "system" {
        match state.db.system_history(range).await {
            Ok(rows) => {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "target": "system",
                        "range_hours": range,
                        "points": rows,
                    })),
                )
                    .into_response();
            }
            Err(e) => {
                return api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "db_error",
                    &e.to_string(),
                );
            }
        }
    }

    if !state.cfg.targets.iter().any(|t| t.name == target) {
        return api_error(
            StatusCode::NOT_FOUND,
            "target_not_found",
            &format!("unknown target: {target}"),
        );
    }

    match state.db.target_history(&target, range).await {
        Ok(rows) => (
            StatusCode::OK,
            Json(json!({
                "target": target,
                "range_hours": range,
                "points": rows,
            })),
        )
            .into_response(),
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "db_error",
            &e.to_string(),
        ),
    }
}

async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "version": env!("CARGO_PKG_VERSION"),
            "bind": state.cfg.bind,
            "port": state.cfg.port,
            "db_path": state.cfg.db_path,
            "interval_seconds": state.cfg.interval_seconds,
            "retention_hours": state.cfg.retention_hours,
            "max_history_hours": state.cfg.max_history_hours,
            "request_timeout_secs": state.cfg.request_timeout_secs,
            "targets": state.cfg.targets,
        })),
    )
}

fn api_error(status: StatusCode, code: &str, message: &str) -> axum::response::Response {
    (
        status,
        Json(json!({
            "error": {
                "code": code,
                "message": message,
            }
        })),
    )
        .into_response()
}
