mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::RwLock;
use tower::ServiceExt;

async fn seeded_app() -> (axum::Router, tempfile::TempDir) {
    let (db, dir) = common::open_temp_db().await;
    let cfg = tinytop::config::Config {
        db_path: dir.path().join("test.db").to_str().unwrap().to_string(),
        targets: vec![tinytop::config::Target {
            name: "t1".into(),
            url: "http://127.0.0.1:9/".into(),
            kind: Some("generic".into()),
            check_type: "http".into(),
            host: None,
            port: None,
            latency_warn_ms: None,
            cert_expiry_warn_days: None,
        }],
        ..Default::default()
    };
    cfg.validate().unwrap();

    db.insert_system_metric(
        10.0,
        100.0,
        1000.0,
        10.0,
        10.0,
        100.0,
        90.0,
        10.0,
        1,
        2,
        3.0,
        4.0,
        Some(0.5),
        Some(0.4),
        Some(0.3),
        Some(0.0),
        Some(0.0),
        Some(0.0),
        Some(1234),
        None,
    )
    .await
    .unwrap();
    db.insert_target_metric(
        "t1",
        "generic",
        "http://127.0.0.1:9/",
        "up",
        Some(12.0),
        Some(1),
        Some("ok"),
        None,
        Some("http"),
    )
    .await
    .unwrap();
    db.insert_event("info", "test", Some("test"), "hello", None)
        .await
        .unwrap();
    db.fire_alert(
        "warn",
        "system",
        "cpu_high",
        "cpu high",
        Some(99.0),
        Some(90.0),
    )
    .await
    .unwrap();

    let runtime = Arc::new(RwLock::new(tinytop::RuntimeStatus::new(&cfg)));
    {
        let mut rt = runtime.write().await;
        rt.last_collection_ok = Some(true);
        rt.collections_total = 1;
    }
    let cfgs = Arc::new(RwLock::new(cfg));
    let shutdown = Arc::new(AtomicBool::new(false));
    let app = tinytop::web::create_router(db, cfgs, runtime, shutdown);
    (app, dir)
}

#[tokio::test]
async fn api_endpoints_and_export_prometheus() {
    let (app, _dir) = seeded_app().await;

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["system"]["cpu_percent"].is_number());
    assert!(!v["targets"].as_array().unwrap().is_empty());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/alerts?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!v["alerts"].as_array().unwrap().is_empty());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/events?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!v["events"].as_array().unwrap().is_empty());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/export?target=system&format=csv&range=24")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(ct.contains("text/csv"), "content-type was {ct}");
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains(
        "timestamp,origin,bucket_seconds,samples,cpu_percent,mem_used_percent,disk_used_percent"
    ));
    assert!(s.contains(",raw,"));

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/history?target=system&range=24")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["target"], "system");
    assert_eq!(v["downsampled"], false);
    assert!(v["bucket_seconds"].is_null());
    let pts = v["points"].as_array().unwrap();
    assert!(!pts.is_empty());
    assert_eq!(pts[0]["origin"], "raw");
    assert!(pts[0]["mem_used_mb"].is_number());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/export?target=system&format=json&range=24")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["target"], "system");
    assert_eq!(v["downsampled"], false);
    assert!(v["points"].as_array().is_some());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/export?target=t1&format=json&range=24")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["target"], "t1");
    assert!(!v["points"].as_array().unwrap().is_empty());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/export?target=t1&format=csv&range=24")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains("timestamp,target,check_type,status"));

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/export?target=missing&format=json&range=24")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/export?target=system&format=xml&range=24")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"]["code"], "invalid_format");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/export?target=%20&format=json&range=24")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"]["code"], "invalid_target");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains("tinytop_up"));
    assert!(s.contains("tinytop_target_up"));

    // dashboard y assets (CSP endurecida, sin unsafe-inline)
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let csp = res
        .headers()
        .get("content-security-policy")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(csp.contains("script-src 'self'"));
    assert!(!csp.contains("unsafe-inline"));
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("/static/app.js"));
    assert!(html.contains("/static/app.css"));
    assert!(html.contains("authBtn"));

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/app.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/app.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

async fn auth_app() -> (axum::Router, tempfile::TempDir) {
    common::auth_app().await
}

#[tokio::test]
async fn api_auth_and_config_redaction() {
    let (app, _dir) = auth_app().await;

    // health allowed unauth
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // protected endpoint requires auth
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // with bearer token ok
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/latest")
                .header("Authorization", "Bearer secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // config is redacted
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .header("X-API-Key", "secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["auth_enabled"], true);

    // legacy query token is accepted when explicitly enabled for compatibility (discouraged)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/latest?token=secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // wrong token is rejected
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/latest")
                .header("Authorization", "Bearer wrong")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_basic_auth_and_health_exemption() {
    let (app, _dir) = common::basic_auth_app().await;

    // health allowed unauth
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // protected endpoint requires auth
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // valid basic auth ok
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/latest")
                .header("Authorization", "Basic dXNlcjpwYXNz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // invalid basic auth rejected
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/latest")
                .header("Authorization", "Basic dXNlcjp3cm9uZw==")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_system_history_downsampled_and_export() {
    let (db, dir) = common::open_temp_db().await;
    let cfg = tinytop::config::Config {
        db_path: dir.path().join("test.db").to_str().unwrap().to_string(),
        targets: vec![],
        ..Default::default()
    };
    cfg.validate().unwrap();

    // Two samples in the same 5-minute bucket, ~30h ago (range > 24h -> downsampled)
    let base = chrono::Utc::now() - chrono::Duration::hours(30);
    let aligned =
        chrono::DateTime::<chrono::Utc>::from_timestamp((base.timestamp() / 300) * 300, 0).unwrap();
    let t = |secs: i64| (aligned + chrono::Duration::seconds(secs)).to_rfc3339();
    for (ts, cpu) in [(t(60), 10.0_f64), (t(180), 30.0_f64)] {
        sqlx::query(
            "INSERT INTO system_metrics (
                timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec,
                load1, load5, load15, swap_used_mb, swap_total_mb, swap_used_percent,
                uptime_seconds, top_processes
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(ts)
        .bind(cpu)
        .bind(512.0_f64)
        .bind(1024.0_f64)
        .bind(50.0_f64)
        .bind(1000.0_f64)
        .bind(2000.0_f64)
        .bind(1000.0_f64)
        .bind(50.0_f64)
        .bind(100_i64)
        .bind(200_i64)
        .bind(10.0_f64)
        .bind(20.0_f64)
        .bind(Some(0.5_f64))
        .bind(Some(0.4_f64))
        .bind(Some(0.3_f64))
        .bind(Some(0.0_f64))
        .bind(Some(0.0_f64))
        .bind(Some(0.0_f64))
        .bind(Some(100_i64))
        .bind::<Option<String>>(None)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    let created = db.compute_rollups(300).await.unwrap();
    assert!(created >= 1);

    let runtime = Arc::new(RwLock::new(tinytop::RuntimeStatus::new(&cfg)));
    {
        let mut rt = runtime.write().await;
        rt.last_collection_ok = Some(true);
        rt.collections_total = 1;
    }
    let cfgs = Arc::new(RwLock::new(cfg));
    let shutdown = Arc::new(AtomicBool::new(false));
    let app = tinytop::web::create_router(db, cfgs, runtime, shutdown);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/metrics/history?target=system&range=48")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["target"], "system");
    assert_eq!(v["downsampled"], true);
    assert_eq!(v["bucket_seconds"], 300);
    let pts = v["points"].as_array().unwrap();
    assert!(!pts.is_empty());
    assert_eq!(pts[0]["origin"], "rollup");
    assert!(pts[0]["samples"].as_i64().unwrap_or(0) >= 2);
    assert!(pts[0]["mem_used_mb"].is_null());
    assert!(pts[0]["net_rx_bytes"].is_null());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/export?target=system&format=json&range=48")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["downsampled"], true);
    assert_eq!(v["bucket_seconds"], 300);
    assert_eq!(v["points"].as_array().unwrap()[0]["origin"], "rollup");

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/export?target=system&format=csv&range=48")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains("timestamp,origin,bucket_seconds,samples"));
    assert!(s.contains(",rollup,300,"));
}

#[tokio::test]
async fn api_health_requires_auth_when_not_exempt() {
    let (app, _dir) = common::auth_app_health_protected().await;

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .header("Authorization", "Bearer secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Dashboard and static assets remain accessible so the token panel can be shown
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/app.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn api_alerts_and_events_filters() {
    let (app, _dir) = seeded_app().await;

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/alerts?status=firing&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let alerts = v["alerts"].as_array().unwrap();
    assert!(!alerts.is_empty());
    assert!(alerts.iter().all(|a| a["status"] == "firing"));

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/alerts?status=resolved&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["alerts"].as_array().unwrap().is_empty());

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/events?limit=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let events = v["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn api_target_history_points() {
    let (app, _dir) = seeded_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/history?target=t1&range=24")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["target"], "t1");
    assert_eq!(v["downsampled"], false);
    let pts = v["points"].as_array().unwrap();
    assert!(!pts.is_empty());
    assert_eq!(pts[0]["origin"], "raw");
    assert!(pts[0]["latency_ms"].is_number());
    assert!(pts[0]["up_ratio"].is_number());
    assert!(v["bucket_seconds"].is_null());
}
