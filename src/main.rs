mod collector;
mod config;
mod db;
mod web;

use collector::CollectorState;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimeStatus {
    pub version: String,
    pub started_at: String,
    pub last_collection_at: Option<String>,
    pub last_collection_ok: Option<bool>,
    pub last_collection_error: Option<String>,
    pub last_collection_duration_ms: Option<u64>,
    pub collections_total: u64,
    pub collection_errors_total: u64,
    pub targets_configured: usize,
    pub retention_hours: u64,
    pub interval_seconds: u64,
    pub db_path: String,
}

impl RuntimeStatus {
    fn new(cfg: &config::Config) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            last_collection_at: None,
            last_collection_ok: None,
            last_collection_error: None,
            last_collection_duration_ms: None,
            collections_total: 0,
            collection_errors_total: 0,
            targets_configured: cfg.targets.len(),
            retention_hours: cfg.retention_hours,
            interval_seconds: cfg.interval_seconds,
            db_path: cfg.db_path.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    let cfg = config::Config::load()?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        bind = %cfg.bind,
        port = cfg.port,
        db_path = %cfg.db_path,
        interval_seconds = cfg.interval_seconds,
        retention_hours = cfg.retention_hours,
        targets = cfg.targets.len(),
        "starting tinytop"
    );

    let db = Arc::new(db::Database::open(&cfg.db_path).await?);
    db.init().await?;
    if let Err(e) = db.prune_older_than_hours(cfg.retention_hours).await {
        tracing::warn!("initial retention prune failed: {e}");
    }

    let runtime = Arc::new(RwLock::new(RuntimeStatus::new(&cfg)));
    let shutdown = Arc::new(AtomicBool::new(false));

    let db_for_collector = db.clone();
    let cfg_for_collector = cfg.clone();
    let runtime_for_collector = runtime.clone();
    let shutdown_for_collector = shutdown.clone();
    tokio::spawn(async move {
        run_collector(
            db_for_collector,
            cfg_for_collector,
            runtime_for_collector,
            shutdown_for_collector,
        )
        .await;
    });

    let app = web::create_router(db.clone(), cfg.clone(), runtime.clone(), shutdown.clone());
    let addr = format!("{}:{}", cfg.bind, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown.clone()))
        .await?;

    db.pool.close().await;
    tracing::info!("shutdown complete");
    Ok(())
}

async fn run_collector(
    db: Arc<db::Database>,
    cfg: config::Config,
    runtime: Arc<RwLock<RuntimeStatus>>,
    shutdown: Arc<AtomicBool>,
) {
    let mut state = CollectorState::default();
    let mut last_prune = Instant::now() - Duration::from_secs(3600);
    let interval = Duration::from_secs(cfg.interval_seconds.max(1));

    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        let started = Instant::now();
        let result = collector::collect(&cfg, &mut state).await;
        let duration_ms = started.elapsed().as_millis() as u64;

        match result {
            Ok(snap) => {
                let mut db_ok = true;
                if let Err(e) = db
                    .insert_system_metric(
                        snap.system.cpu_percent,
                        snap.system.mem_used_mb,
                        snap.system.mem_total_mb,
                        snap.system.mem_used_percent,
                        snap.system.disk_used_mb,
                        snap.system.disk_total_mb,
                        snap.system.disk_available_mb,
                        snap.system.disk_used_percent,
                        snap.system.net_rx_bytes,
                        snap.system.net_tx_bytes,
                        snap.system.net_rx_bytes_per_sec,
                        snap.system.net_tx_bytes_per_sec,
                    )
                    .await
                {
                    db_ok = false;
                    tracing::error!("failed to persist system metrics: {e}");
                }

                for t in &snap.targets {
                    if let Err(e) = db
                        .insert_target_metric(
                            &t.name,
                            &t.kind,
                            &t.url,
                            &t.status,
                            t.latency_ms,
                            t.http_requests,
                            t.detail.as_deref(),
                        )
                        .await
                    {
                        db_ok = false;
                        tracing::error!(target = %t.name, "failed to persist target metrics: {e}");
                    }
                }

                let mut rt = runtime.write().await;
                rt.collections_total += 1;
                rt.last_collection_at = Some(snap.timestamp.clone());
                rt.last_collection_duration_ms = Some(duration_ms);
                rt.last_collection_ok = Some(db_ok);
                rt.last_collection_error = if db_ok {
                    None
                } else {
                    Some("database write failure".to_string())
                };
                if !db_ok {
                    rt.collection_errors_total += 1;
                }
            }
            Err(e) => {
                tracing::error!("collection failed: {e:#}");
                let mut rt = runtime.write().await;
                rt.collections_total += 1;
                rt.collection_errors_total += 1;
                rt.last_collection_at = Some(chrono::Utc::now().to_rfc3339());
                rt.last_collection_duration_ms = Some(duration_ms);
                rt.last_collection_ok = Some(false);
                rt.last_collection_error = Some(e.to_string());
            }
        }

        if last_prune.elapsed() >= Duration::from_secs(3600) {
            match db.prune_older_than_hours(cfg.retention_hours).await {
                Ok(r) => {
                    if r.system_deleted > 0 || r.target_deleted > 0 {
                        tracing::info!(
                            system_deleted = r.system_deleted,
                            target_deleted = r.target_deleted,
                            "retention prune complete"
                        );
                    }
                }
                Err(e) => tracing::warn!("retention prune failed: {e}"),
            }
            last_prune = Instant::now();
        }

        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        tokio::select! {
            _ = tokio::time::sleep(interval) => {},
            _ = shutdown_wait(shutdown.clone()) => break,
        }
    }
}

async fn shutdown_signal(shutdown: Arc<AtomicBool>) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    shutdown.store(true, Ordering::Relaxed);
    tracing::info!("shutdown signal received");
}

async fn shutdown_wait(shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
