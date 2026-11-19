use axum::Router;
use std::sync::{Arc, atomic::AtomicBool};
use tempfile::TempDir;
use tokio::sync::RwLock;

#[allow(dead_code)]
pub async fn open_temp_db() -> (Arc<tinytop::db::Database>, TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = tinytop::db::Database::open(db_path.to_str().expect("db path utf-8"))
        .await
        .expect("open db");
    db.init().await.expect("init db");
    (Arc::new(db), dir)
}

#[allow(dead_code)]
pub async fn base_app() -> (Router, TempDir) {
    let (db, dir) = open_temp_db().await;
    let cfg = tinytop::config::Config {
        db_path: dir
            .path()
            .join("test.db")
            .to_str()
            .expect("utf-8")
            .to_string(),
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
    cfg.validate().expect("valid cfg");
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

#[allow(dead_code)]
pub async fn auth_app() -> (Router, TempDir) {
    let (db, dir) = open_temp_db().await;
    let cfg = tinytop::config::Config {
        db_path: dir
            .path()
            .join("test.db")
            .to_str()
            .expect("utf-8")
            .to_string(),
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
    cfg.validate().expect("valid cfg");
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

#[allow(dead_code)]
pub async fn basic_auth_app() -> (Router, TempDir) {
    let (db, dir) = open_temp_db().await;
    let cfg = tinytop::config::Config {
        db_path: dir
            .path()
            .join("test.db")
            .to_str()
            .expect("utf-8")
            .to_string(),
        auth: Some(tinytop::config::AuthConfig {
            token: None,
            basic_user: Some("user".into()),
            basic_password: Some("pass".into()),
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
    cfg.validate().expect("valid cfg");
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

#[allow(dead_code)]
pub async fn auth_app_health_protected() -> (Router, TempDir) {
    let (db, dir) = open_temp_db().await;
    let cfg = tinytop::config::Config {
        db_path: dir
            .path()
            .join("test.db")
            .to_str()
            .expect("utf-8")
            .to_string(),
        auth: Some(tinytop::config::AuthConfig {
            token: Some("secret".into()),
            basic_user: None,
            basic_password: None,
            allow_health_unauth: false,
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
    cfg.validate().expect("valid cfg");
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
