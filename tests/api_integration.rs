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
    let ct = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(ct.contains("text/csv"), "content-type was {ct}");
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains("timestamp,cpu_percent,mem_used_percent"));

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
}

async fn test_app_with_auth() -> (axum::Router, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let cfg = tinytop::config::Config {
        db_path: db_path.to_str().unwrap().to_string(),
        auth: Some(tinytop::config::AuthConfig {
            token: Some("secret".into()),
            basic_user: None,
            basic_password: None,
            allow_health_unauth: true,
        }),
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
async fn api_auth_and_config_redaction() {
    let (app, _dir) = test_app_with_auth().await;

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
}
