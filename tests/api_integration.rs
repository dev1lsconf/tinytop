use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::sync::{Arc, atomic::AtomicBool};
use tempfile::tempdir;
use tokio::sync::RwLock;
use tower::ServiceExt;

async fn test_app() -> (axum::Router, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let cfg = tinytop::config::Config {
        db_path: db_path.to_str().unwrap().to_string(),
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

    let db = Arc::new(tinytop::db::Database::open(&cfg.db_path).await.unwrap());
    db.init().await.unwrap();
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
        None,
        None,
        None,
        Some(40.0),
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
    let (app, _dir) = test_app().await;

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
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains("timestamp"));

    let res = app
        .clone()
        .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains("tinytop_up"));
    assert!(s.contains("tinytop_target_up"));
}
