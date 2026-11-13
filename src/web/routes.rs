use axum::{
    Json, Router,
    extract::{Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
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
    format: Option<String>,
}

#[derive(Deserialize)]
struct ListParams {
    status: Option<String>,
    limit: Option<i64>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/static/vendor/chart.umd.min.js", get(serve_chart_js))
        .route("/api/health", get(health))
        .route("/api/metrics/latest", get(latest))
        .route("/api/metrics/history", get(history))
        .route("/api/config", get(get_config))
        .route("/api/alerts", get(list_alerts))
        .route("/api/events", get(list_events))
        .route("/api/export", get(export_data))
        .route("/metrics", get(prometheus_metrics))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn security_headers(req: Request, next: Next) -> Response {
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
    headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' data:; img-src 'self' data:; connect-src 'self'; font-src 'self' data:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'",
    ));
    res
}

async fn auth_middleware(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let cfg = state.cfg.read().await.clone();
    let path = req.uri().path().to_string();
    let required = cfg
        .auth
        .as_ref()
        .and_then(|a| a.token.as_ref())
        .map(|t| !t.is_empty())
        .unwrap_or(false)
        || cfg
            .auth
            .as_ref()
            .map(|a| a.basic_user.is_some() && a.basic_password.is_some())
            .unwrap_or(false);
    if !required {
        return next.run(req).await;
    }
    // allow health unauth if configured
    if path == "/api/health"
        && cfg
            .auth
            .as_ref()
            .map(|a| a.allow_health_unauth)
            .unwrap_or(true)
    {
        return next.run(req).await;
    }
    // static vendor/js/css used by UI - allow (UI will prompt/fail on API calls) but better allow same-origin assets
    if path.starts_with("/static/") || path == "/" {
        return next.run(req).await;
    }
    let headers = req.headers();
    if is_authorized(headers, req.uri().query(), &cfg) {
        return next.run(req).await;
    }
    let mut res = api_error(
        StatusCode::UNAUTHORIZED,
        "unauthorized",
        "authentication required",
    );
    res.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Bearer, Basic realm=\"tinytop\""),
    );
    res
}

fn is_authorized(headers: &HeaderMap, query: Option<&str>, cfg: &crate::config::Config) -> bool {
    if let Some(auth) = &cfg.auth {
        if let Some(token) = &auth.token {
            if let Some(h) = headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                && h.strip_prefix("Bearer ")
                    .map(|t| t == token)
                    .unwrap_or(false)
            {
                return true;
            }
            if let Some(h) = headers.get("x-api-key").and_then(|v| v.to_str().ok())
                && h == token
            {
                return true;
            }
            if let Some(q) = query
                && q.split('&').any(|p| {
                    p.strip_prefix("token=")
                        .map(|v| v == token)
                        .unwrap_or(false)
                })
            {
                return true;
            }
        }
        if let (Some(u), Some(pw)) = (&auth.basic_user, &auth.basic_password)
            && let Some(h) = headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
            && let Some(b64) = h.strip_prefix("Basic ")
        {
            use base64::Engine;
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64)
                && let Ok(s) = String::from_utf8(decoded)
                && s == format!("{u}:{pw}")
            {
                return true;
            }
        }
    }
    false
}

async fn serve_index() -> impl IntoResponse {
    Html(include_str!("../../static/index.html"))
}

async fn serve_chart_js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
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
        Json(
            json!({"status":status,"ready":ready,"version":env!("CARGO_PKG_VERSION"),"db":{"ok":db_ok},"shutting_down":shutting_down,"runtime":runtime}),
        ),
    )
}

async fn latest(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.cfg.read().await.clone();
    let mut system = Value::Null;
    match state.db.latest_system().await {
        Ok(Some(row)) => {
            system = serde_json::to_value(&row).unwrap_or(Value::Null);
            if let Value::Object(ref mut m) = system {
                m.insert("type".into(), json!("system"));
            }
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
    let mut targets = Vec::with_capacity(cfg.targets.len());
    for t in &cfg.targets {
        match state.db.latest_target(&t.name).await {
            Ok(Some(row)) => { let mut v = serde_json::to_value(&row).unwrap_or(Value::Null); if let Value::Object(ref mut m)=v { m.insert("type".into(), json!("target")); m.insert("name".into(), json!(row.target)); } targets.push(v); }
            Ok(None) => targets.push(json!({"type":"target","name":t.name,"kind":t.kind.clone().unwrap_or_else(||"generic".into()),"url":t.url,"status":"unknown","latency_ms":Value::Null,"http_requests":Value::Null,"detail":"no data yet","timestamp":Value::Null,"check_type":t.check_type,"cert_days_left":Value::Null})),
            Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error", &e.to_string()),
        }
    }
    (
        StatusCode::OK,
        Json(json!({"system":system,"targets":targets})),
    )
        .into_response()
}

async fn history(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> impl IntoResponse {
    let cfg = state.cfg.read().await.clone();
    let target = params.target.unwrap_or_else(|| "system".into());
    let max_range = cfg.max_history_hours.max(1).min(cfg.retention_hours as i64);
    let range = params.range.unwrap_or(24).clamp(1, max_range);
    if target.trim().is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "invalid_target",
            "target must not be empty",
        );
    }
    if target == "system" {
        return match state.db.system_history(range).await {
            Ok(rows) => (
                StatusCode::OK,
                Json(json!({"target":"system","range_hours":range,"points":rows})),
            )
                .into_response(),
            Err(e) => api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_error",
                &e.to_string(),
            ),
        };
    }
    if !cfg.targets.iter().any(|t| t.name == target) {
        return api_error(
            StatusCode::NOT_FOUND,
            "target_not_found",
            &format!("unknown target: {target}"),
        );
    }
    match state.db.target_history(&target, range).await {
        Ok(rows) => (
            StatusCode::OK,
            Json(json!({"target":target,"range_hours":range,"points":rows})),
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
    let cfg = state.cfg.read().await.clone();
    let redacted = cfg.redacted();
    (
        StatusCode::OK,
        Json(json!({
            "version": env!("CARGO_PKG_VERSION"), "bind": redacted.bind, "port": redacted.port, "db_path": redacted.db_path,
            "interval_seconds": redacted.interval_seconds, "retention_hours": redacted.retention_hours,
            "max_history_hours": redacted.max_history_hours, "request_timeout_secs": redacted.request_timeout_secs,
            "targets": redacted.targets, "alerts": redacted.alerts,
            "auth_enabled": redacted.auth.as_ref().map(|a| a.token.is_some() || a.basic_user.is_some()).unwrap_or(false),
        })),
    )
}

async fn list_alerts(
    State(state): State<AppState>,
    Query(p): Query<ListParams>,
) -> impl IntoResponse {
    let limit = p.limit.unwrap_or(100).clamp(1, 1000);
    match state.db.list_alerts(p.status.as_deref(), limit).await {
        Ok(rows) => (StatusCode::OK, Json(json!({"alerts": rows}))).into_response(),
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "db_error",
            &e.to_string(),
        ),
    }
}

async fn list_events(
    State(state): State<AppState>,
    Query(p): Query<ListParams>,
) -> impl IntoResponse {
    let limit = p.limit.unwrap_or(100).clamp(1, 1000);
    match state.db.list_events(limit).await {
        Ok(rows) => (StatusCode::OK, Json(json!({"events": rows}))).into_response(),
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "db_error",
            &e.to_string(),
        ),
    }
}

async fn export_data(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> impl IntoResponse {
    let cfg = state.cfg.read().await.clone();
    let target = params.target.unwrap_or_else(|| "system".into());
    let max_range = cfg.max_history_hours.max(1).min(cfg.retention_hours as i64);
    let range = params.range.unwrap_or(24).clamp(1, max_range);
    let format = params.format.unwrap_or_else(|| "json".into());
    if !format.eq_ignore_ascii_case("json") && !format.eq_ignore_ascii_case("csv") {
        return api_error(
            StatusCode::BAD_REQUEST,
            "invalid_format",
            "format must be 'json' or 'csv'",
        );
    }
    if target == "system" {
        match state.db.export_system_history(range).await {
            Ok(rows) => export_rows_system(rows, &format),
            Err(e) => api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_error",
                &e.to_string(),
            ),
        }
    } else {
        if !cfg.targets.iter().any(|t| t.name == target) {
            return api_error(
                StatusCode::NOT_FOUND,
                "target_not_found",
                &format!("unknown target: {target}"),
            );
        }
        match state.db.export_target_history(&target, range).await {
            Ok(rows) => export_rows_target(&target, rows, &format),
            Err(e) => api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "db_error",
                &e.to_string(),
            ),
        }
    }
}

fn export_rows_system(rows: Vec<crate::db::SystemMetric>, format: &str) -> Response {
    if format.eq_ignore_ascii_case("csv") {
        let mut out = String::from(
            "timestamp,cpu_percent,mem_used_percent,disk_used_percent,load1,swap_used_percent,temp_celsius,net_rx_bps,net_tx_bps\n",
        );
        for r in rows {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                r.timestamp,
                r.cpu_percent,
                r.mem_used_percent,
                r.disk_used_percent,
                r.load1.unwrap_or(f64::NAN),
                r.swap_used_percent.unwrap_or(f64::NAN),
                r.temp_celsius.unwrap_or(f64::NAN),
                r.net_rx_bytes_per_sec,
                r.net_tx_bytes_per_sec
            ));
        }
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/csv; charset=utf-8")],
            out,
        )
            .into_response()
    } else {
        (
            StatusCode::OK,
            Json(json!({"target":"system","points":rows})),
        )
            .into_response()
    }
}
fn export_rows_target(target: &str, rows: Vec<crate::db::TargetMetric>, format: &str) -> Response {
    if format.eq_ignore_ascii_case("csv") {
        let mut out = String::from(
            "timestamp,target,check_type,status,latency_ms,http_requests,cert_days_left,detail\n",
        );
        for r in rows {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                r.timestamp,
                r.target,
                r.check_type.unwrap_or_default(),
                r.status,
                r.latency_ms.unwrap_or(f64::NAN),
                r.http_requests.unwrap_or(-1),
                r.cert_days_left.unwrap_or(-1),
                r.detail.unwrap_or_default().replace(',', ";")
            ));
        }
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/csv; charset=utf-8")],
            out,
        )
            .into_response()
    } else {
        (
            StatusCode::OK,
            Json(json!({"target": target, "points": rows})),
        )
            .into_response()
    }
}

async fn prometheus_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let mut out = String::new();
    out.push_str(
        "# HELP tinytop_up Whether TinyTop is running.\n# TYPE tinytop_up gauge\ntinytop_up 1\n",
    );
    if let Ok(Some(s)) = state.db.latest_system().await {
        macro_rules! g {
            ($name:expr,$help:expr,$val:expr) => {
                out.push_str(&format!(
                    "# HELP {} {}\n# TYPE {} gauge\n{} {}\n",
                    $name, $help, $name, $name, $val
                ));
            };
        }
        g!("tinytop_cpu_percent", "CPU percent", s.cpu_percent);
        g!(
            "tinytop_mem_used_percent",
            "Memory used percent",
            s.mem_used_percent
        );
        g!(
            "tinytop_disk_used_percent",
            "Disk used percent",
            s.disk_used_percent
        );
        g!(
            "tinytop_net_rx_bytes_per_sec",
            "Network RX bytes/sec",
            s.net_rx_bytes_per_sec
        );
        g!(
            "tinytop_net_tx_bytes_per_sec",
            "Network TX bytes/sec",
            s.net_tx_bytes_per_sec
        );
        if let Some(v) = s.load1 {
            g!("tinytop_load1", "Load average 1m", v);
        }
        if let Some(v) = s.swap_used_percent {
            g!("tinytop_swap_used_percent", "Swap used percent", v);
        }
        if let Some(v) = s.temp_celsius {
            g!("tinytop_temp_celsius", "Temperature celsius", v);
        }
    }
    let cfg = state.cfg.read().await.clone();
    for t in &cfg.targets {
        if let Ok(Some(m)) = state.db.latest_target(&t.name).await {
            let up = if matches!(m.status.as_str(), "up" | "ok") {
                1
            } else {
                0
            };
            out.push_str(&format!(
                "tinytop_target_up{{name=\"{}\",check_type=\"{}\"}} {}\n",
                esc_label(&t.name),
                esc_label(m.check_type.as_deref().unwrap_or("http")),
                up
            ));
            if let Some(lat) = m.latency_ms {
                out.push_str(&format!(
                    "tinytop_target_latency_ms{{name=\"{}\"}} {}\n",
                    esc_label(&t.name),
                    lat
                ));
            }
            if let Some(days) = m.cert_days_left {
                out.push_str(&format!(
                    "tinytop_target_cert_days_left{{name=\"{}\"}} {}\n",
                    esc_label(&t.name),
                    days
                ));
            }
        }
    }
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.1.0; charset=utf-8",
        )],
        out,
    )
}
fn esc_label(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

fn api_error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({"error":{"code":code,"message":message}})),
    )
        .into_response()
}
