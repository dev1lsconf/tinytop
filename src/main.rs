mod collector;
mod config;
mod db;
mod tui;
mod web;

use collector::CollectorState;
use config::Config;
use std::path::PathBuf;
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
    fn new(cfg: &Config) -> Self {
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
    fn update_from_cfg(&mut self, cfg: &Config) {
        self.targets_configured = cfg.targets.len();
        self.retention_hours = cfg.retention_hours;
        self.interval_seconds = cfg.interval_seconds;
        self.db_path = cfg.db_path.clone();
    }
}

#[derive(Debug, Clone)]
struct Cli {
    tui: bool,
    check_config: bool,
    help: bool,
    version: bool,
    config: Option<PathBuf>,
}

fn parse_cli() -> Cli {
    let mut cli = Cli {
        tui: false,
        check_config: false,
        help: false,
        version: false,
        config: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--tui" => cli.tui = true,
            "--check-config" | "--check" => cli.check_config = true,
            "--help" | "-h" => cli.help = true,
            "--version" | "-V" => cli.version = true,
            "--config" | "-c" => {
                cli.config = it.next().map(PathBuf::from);
            }
            s if s.starts_with("--config=") => {
                cli.config = Some(PathBuf::from(s.trim_start_matches("--config=")));
            }
            _ => {}
        }
    }
    cli
}

fn print_help() {
    println!(
        "TinyTop v{} - lightweight system & target monitor",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Usage: tinytop [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --tui                 Run interactive terminal UI (instead of web server)");
    println!(
        "  --config <PATH>       Path to config file (overrides TINYTOP_CONFIG; default: config.toml)"
    );
    println!("  --check-config        Validate configuration and exit");
    println!("  -h, --help            Print help");
    println!("  -V, --version         Print version");
    println!();
    println!(
        "Environment/config: see config.toml and TINYTOP_* variables. SIGHUP reloads config (Unix)."
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = parse_cli();
    if cli.help {
        print_help();
        return Ok(());
    }
    if cli.version {
        println!("tinytop {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if !cli.tui {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_target(false)
            .compact()
            .init();
    }

    let cfg = Config::load_with_path(cli.config.clone())?;
    if cli.check_config {
        println!(
            "config ok: bind={}:{} db={} targets={} interval={}s retention={}h",
            cfg.bind,
            cfg.port,
            cfg.db_path,
            cfg.targets.len(),
            cfg.interval_seconds,
            cfg.retention_hours
        );
        return Ok(());
    }

    if !cli.tui {
        tracing::info!(version = env!("CARGO_PKG_VERSION"), bind = %cfg.bind, port = cfg.port, db_path = %cfg.db_path,
            interval_seconds = cfg.interval_seconds, retention_hours = cfg.retention_hours, targets = cfg.targets.len(), "starting tinytop");
    }

    let db = Arc::new(db::Database::open(&cfg.db_path).await?);
    db.init().await?;
    if let Err(e) = db.prune_older_than_hours(cfg.retention_hours).await
        && !cli.tui
    {
        tracing::warn!("initial retention prune failed: {e}");
    }

    let cfg_shared = Arc::new(RwLock::new(cfg.clone()));
    let runtime = Arc::new(RwLock::new(RuntimeStatus::new(&cfg)));
    let shutdown = Arc::new(AtomicBool::new(false));

    {
        let db = db.clone();
        let cfg_s = cfg_shared.clone();
        let rt = runtime.clone();
        let sd = shutdown.clone();
        tokio::spawn(async move {
            run_collector(db, cfg_s, rt, sd).await;
        });
    }
    {
        let db = db.clone();
        let cfg_s = cfg_shared.clone();
        let sd = shutdown.clone();
        tokio::spawn(async move {
            config_reload_loop(db, cfg_s, sd).await;
        });
    }

    if cli.tui {
        tui::run(
            db.clone(),
            cfg_shared.clone(),
            runtime.clone(),
            shutdown.clone(),
        )
        .await?;
        db.pool.close().await;
        return Ok(());
    }

    let app = web::create_router(
        db.clone(),
        cfg_shared.clone(),
        runtime.clone(),
        shutdown.clone(),
    );
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
    cfg_shared: Arc<RwLock<Config>>,
    runtime: Arc<RwLock<RuntimeStatus>>,
    shutdown: Arc<AtomicBool>,
) {
    let mut state = CollectorState::default();
    let mut last_prune = Instant::now() - Duration::from_secs(3600);
    let mut last_rollup = Instant::now() - Duration::from_secs(3600);

    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        let cfg = cfg_shared.read().await.clone();
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
                        snap.system.load1,
                        snap.system.load5,
                        snap.system.load15,
                        snap.system.swap_used_mb,
                        snap.system.swap_total_mb,
                        snap.system.swap_used_percent,
                        snap.system.uptime_seconds,
                        snap.system.disk_read_bytes,
                        snap.system.disk_write_bytes,
                        snap.system.disk_read_bytes_per_sec,
                        snap.system.disk_write_bytes_per_sec,
                        snap.system.temp_celsius,
                        snap.system.top_processes.as_deref(),
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
                            t.cert_days_left,
                            Some(&t.check_type),
                        )
                        .await
                    {
                        db_ok = false;
                        tracing::error!(target = %t.name, "failed to persist target metrics: {e}");
                    }
                }

                // alerts evaluation
                if cfg.alerts.enabled {
                    evaluate_alerts(&db, &cfg, &snap).await;
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
                rt.update_from_cfg(&cfg);
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
                rt.update_from_cfg(&cfg);
            }
        }

        if last_prune.elapsed() >= Duration::from_secs(3600) {
            let cfgp = cfg_shared.read().await.clone();
            match db.prune_older_than_hours(cfgp.retention_hours).await {
                Ok(r) => {
                    if r.system_deleted > 0 || r.target_deleted > 0 {
                        tracing::info!(
                            system_deleted = r.system_deleted,
                            target_deleted = r.target_deleted,
                            "retention prune"
                        );
                    }
                }
                Err(e) => tracing::warn!("retention prune failed: {e}"),
            }
            last_prune = Instant::now();
        }
        if last_rollup.elapsed() >= Duration::from_secs(300) {
            let _ = db.compute_rollups(300).await; // 5m
            if last_rollup.elapsed() >= Duration::from_secs(3600) {
                let _ = db.compute_rollups(3600).await;
            }
            last_rollup = Instant::now();
        }

        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        let interval = Duration::from_secs(cfg_shared.read().await.interval_seconds.max(1));
        tokio::select! { _ = tokio::time::sleep(interval) => {}, _ = shutdown_wait(shutdown.clone()) => break, }
    }
}

async fn evaluate_alerts(db: &db::Database, cfg: &Config, snap: &collector::Snapshot) {
    let s = &snap.system;
    let a = &cfg.alerts;
    let mut changed = false;
    macro_rules! check {
        ($cond:expr, $src:expr, $name:expr, $sev:expr, $msg:expr, $val:expr, $th:expr) => {
            if $cond {
                if matches!(
                    db.fire_alert($sev, $src, $name, $msg, Some($val), Some($th))
                        .await,
                    Ok(true)
                ) {
                    changed = true;
                }
            } else if matches!(db.resolve_alert($src, $name).await, Ok(true)) {
                changed = true;
            }
        };
    }
    check!(
        s.cpu_percent >= a.cpu_percent_warn,
        "system",
        "cpu_high",
        "warn",
        &format!("CPU {:.1}% >= {:.1}%", s.cpu_percent, a.cpu_percent_warn),
        s.cpu_percent,
        a.cpu_percent_warn
    );
    check!(
        s.mem_used_percent >= a.mem_percent_warn,
        "system",
        "mem_high",
        "warn",
        &format!(
            "Mem {:.1}% >= {:.1}%",
            s.mem_used_percent, a.mem_percent_warn
        ),
        s.mem_used_percent,
        a.mem_percent_warn
    );
    check!(
        s.disk_used_percent >= a.disk_percent_warn,
        "system",
        "disk_high",
        "warn",
        &format!(
            "Disk {:.1}% >= {:.1}%",
            s.disk_used_percent, a.disk_percent_warn
        ),
        s.disk_used_percent,
        a.disk_percent_warn
    );
    if let Some(t) = s.temp_celsius {
        check!(
            t >= a.temp_celsius_warn,
            "system",
            "temp_high",
            "warn",
            &format!("Temp {:.1}C >= {:.1}C", t, a.temp_celsius_warn),
            t,
            a.temp_celsius_warn
        );
    }
    for tgt in &snap.targets {
        let src = format!("target:{}", tgt.name);
        let down = matches!(tgt.status.as_str(), "down" | "error");
        check!(
            down,
            &src,
            "down",
            "critical",
            &format!("Target {} is {}", tgt.name, tgt.status),
            1.0,
            1.0
        );
        if let Some(lat) = tgt.latency_ms {
            let th = cfg
                .targets
                .iter()
                .find(|x| x.name == tgt.name)
                .and_then(|x| x.latency_warn_ms)
                .unwrap_or(a.target_latency_ms_warn);
            check!(
                lat >= th,
                &src,
                "latency_high",
                "warn",
                &format!("Target {} latency {:.1}ms >= {:.1}ms", tgt.name, lat, th),
                lat,
                th
            );
        }
        if let Some(days) = tgt.cert_days_left {
            let th = cfg
                .targets
                .iter()
                .find(|x| x.name == tgt.name)
                .and_then(|x| x.cert_expiry_warn_days)
                .unwrap_or(a.cert_expiry_days_warn);
            check!(
                days <= th,
                &src,
                "cert_expiry",
                "warn",
                &format!(
                    "Target {} cert expires in {} days (<= {})",
                    tgt.name, days, th
                ),
                days as f64,
                th as f64
            );
        }
    }
    if changed {
        dispatch_notifications(db, cfg).await;
    }
}

async fn dispatch_notifications(db: &db::Database, cfg: &Config) {
    // simple: send unresolved firing alerts summary occasionally? to avoid spam, only send if there is a webhook configured and we have firing alerts.
    let firing = match db.list_alerts(Some("firing"), 20).await {
        Ok(v) => v,
        Err(_) => return,
    };
    if firing.is_empty() {
        return;
    }
    let msg = firing
        .iter()
        .map(|a| format!("[{}] {}: {}", a.severity, a.source, a.message))
        .collect::<Vec<_>>()
        .join("\n");
    if let Some(url) = &cfg.notifications.webhook_url {
        let _ = reqwest::Client::new()
            .post(url)
            .json(&serde_json::json!({"text": msg, "alerts": firing}))
            .send()
            .await;
    }
    if let (Some(token), Some(chat)) = (
        &cfg.notifications.telegram_bot_token,
        &cfg.notifications.telegram_chat_id,
    ) {
        let u = format!("https://api.telegram.org/bot{token}/sendMessage");
        let _ = reqwest::Client::new()
            .post(u)
            .form(&[("chat_id", chat.as_str()), ("text", msg.as_str())])
            .send()
            .await;
    }
    if let Some(base) = &cfg.notifications.ntfy_url {
        let topic = cfg
            .notifications
            .ntfy_topic
            .clone()
            .unwrap_or_else(|| "tinytop".into());
        let url = format!("{}/{}", base.trim_end_matches('/'), topic);
        let _ = reqwest::Client::new().post(url).body(msg).send().await;
    }
}

async fn config_reload_loop(
    db: Arc<db::Database>,
    cfg_shared: Arc<RwLock<Config>>,
    shutdown: Arc<AtomicBool>,
) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut hup = match signal(SignalKind::hangup()) {
            Ok(s) => s,
            Err(_) => return,
        };
        loop {
            tokio::select! {
                _ = hup.recv() => {
                    let path = std::env::var("TINYTOP_CONFIG").ok().map(std::path::PathBuf::from);
                    match Config::load_with_path(path) {
                        Ok(newc) => {
                            *cfg_shared.write().await = newc;
                            let _ = db.insert_event("info","config_reload",None,"config reloaded via SIGHUP",None).await;
                            tracing::info!("config reloaded via SIGHUP");
                        }
                        Err(e) => { tracing::warn!("config reload failed: {e}"); let _ = db.insert_event("error","config_reload",None,"config reload failed",Some(&e.to_string())).await; }
                    }
                }
                _ = shutdown_wait(shutdown.clone()) => break,
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (db, cfg_shared, shutdown);
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
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {}, }
    shutdown.store(true, Ordering::Relaxed);
    tracing::info!("shutdown signal received");
}

async fn shutdown_wait(shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
