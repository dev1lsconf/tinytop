pub mod collector;
pub mod config;
pub mod db;
pub mod tui;
pub mod web;

pub use collector::CollectorState;
pub use config::Config;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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
    pub fn new(cfg: &Config) -> Self {
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
    pub fn update_from_cfg(&mut self, cfg: &Config) {
        self.targets_configured = cfg.targets.len();
        self.retention_hours = cfg.retention_hours;
        self.interval_seconds = cfg.interval_seconds;
        self.db_path = cfg.db_path.clone();
    }
}

#[derive(Debug, Default)]
pub struct NotifyState {
    pub last_sent: Option<Instant>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AlertTransition {
    pub status: String, // "firing" | "resolved"
    pub severity: String,
    pub source: String,
    pub name: String,
    pub message: String,
}

pub async fn run_collector(
    db: Arc<db::Database>,
    cfg_shared: Arc<RwLock<Config>>,
    runtime: Arc<RwLock<RuntimeStatus>>,
    shutdown: Arc<AtomicBool>,
) {
    let mut state = CollectorState::default();
    let mut notify_state = NotifyState::default();
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

                if cfg.alerts.enabled {
                    evaluate_alerts(&db, &cfg, &snap, &mut notify_state).await;
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
            let _ = db.compute_rollups(300).await;
            if last_rollup.elapsed() >= Duration::from_secs(3600) {
                let _ = db.compute_rollups(3600).await;
            }
            last_rollup = Instant::now();
        }

        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        let interval = Duration::from_secs(cfg_shared.read().await.interval_seconds.max(1));
        tokio::select! {
            _ = tokio::time::sleep(interval) => {},
            _ = shutdown_wait(shutdown.clone()) => break,
        }
    }
}

pub async fn evaluate_alerts(
    db: &db::Database,
    cfg: &Config,
    snap: &collector::Snapshot,
    notify_state: &mut NotifyState,
) {
    let s = &snap.system;
    let a = &cfg.alerts;
    let mut transitions: Vec<AlertTransition> = Vec::new();
    macro_rules! check {
        ($cond:expr, $src:expr, $name:expr, $sev:expr, $msg:expr, $val:expr, $th:expr) => {
            if $cond {
                if matches!(
                    db.fire_alert($sev, $src, $name, $msg, Some($val), Some($th))
                        .await,
                    Ok(true)
                ) {
                    transitions.push(AlertTransition {
                        status: "firing".to_string(),
                        severity: $sev.to_string(),
                        source: $src.to_string(),
                        name: $name.to_string(),
                        message: $msg.to_string(),
                    });
                }
            } else if matches!(db.resolve_alert($src, $name).await, Ok(true)) {
                transitions.push(AlertTransition {
                    status: "resolved".to_string(),
                    severity: $sev.to_string(),
                    source: $src.to_string(),
                    name: $name.to_string(),
                    message: format!("{} (resolved)", $msg),
                });
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
    if !transitions.is_empty() {
        dispatch_notifications(db, cfg, &transitions, notify_state).await;
    }
}

pub fn filter_notification_transitions(
    cfg: &Config,
    transitions: &[AlertTransition],
) -> Vec<AlertTransition> {
    transitions
        .iter()
        .filter(|t| match t.status.as_str() {
            "firing" => cfg.notifications.notify_on_firing,
            "resolved" => cfg.notifications.notify_on_resolve,
            _ => true,
        })
        .cloned()
        .collect()
}

pub fn notification_cooldown_allows(cfg: &Config, state: &NotifyState, now: Instant) -> bool {
    let cooldown = cfg.notifications.cooldown_secs;
    if cooldown == 0 {
        return true;
    }
    match state.last_sent {
        None => true,
        Some(last) => now.duration_since(last) >= Duration::from_secs(cooldown),
    }
}

fn format_notification_message(transitions: &[AlertTransition]) -> String {
    transitions
        .iter()
        .map(|t| {
            let status = t.status.to_ascii_uppercase();
            format!("[{status}] [{}] {}: {}", t.severity, t.source, t.message)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn dispatch_notifications(
    _db: &db::Database,
    cfg: &Config,
    transitions: &[AlertTransition],
    notify_state: &mut NotifyState,
) {
    let relevant = filter_notification_transitions(cfg, transitions);
    if relevant.is_empty() {
        return;
    }
    let now = Instant::now();
    if !notification_cooldown_allows(cfg, notify_state, now) {
        return;
    }
    let msg = format_notification_message(&relevant);
    let mut sent = false;
    if let Some(url) = &cfg.notifications.webhook_url {
        let is_discord = url.contains("discord.com/api/webhooks")
            || url.contains("discordapp.com/api/webhooks");
        let payload = if is_discord {
            serde_json::json!({"content": msg})
        } else {
            serde_json::json!({"text": msg, "transitions": relevant})
        };
        let _ = reqwest::Client::new().post(url).json(&payload).send().await;
        sent = true;
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
        sent = true;
    }
    if let Some(base) = &cfg.notifications.ntfy_url {
        let topic = cfg
            .notifications
            .ntfy_topic
            .clone()
            .unwrap_or_else(|| "tinytop".into());
        let url = format!("{}/{}", base.trim_end_matches('/'), topic);
        let _ = reqwest::Client::new().post(url).body(msg).send().await;
        sent = true;
    }
    if sent {
        notify_state.last_sent = Some(now);
    }
}

pub async fn config_reload_loop(
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
                            let (msg, changed_restart_fields) = {
                                let mut shared = cfg_shared.write().await;
                                let mut non_reloadable = Vec::new();
                                if newc.bind != shared.bind {
                                    non_reloadable.push("bind");
                                }
                                if newc.port != shared.port {
                                    non_reloadable.push("port");
                                }
                                if newc.db_path != shared.db_path {
                                    non_reloadable.push("db_path");
                                }
                                *shared = newc;
                                if non_reloadable.is_empty() {
                                    ("config reloaded via SIGHUP".to_string(), false)
                                } else {
                                    (
                                        format!(
                                            "config reloaded via SIGHUP (restart required for: {})",
                                            non_reloadable.join(", ")
                                        ),
                                        true,
                                    )
                                }
                            };
                            let _ = db
                                .insert_event(
                                    "info",
                                    "config_reload",
                                    None,
                                    &msg,
                                    if changed_restart_fields {
                                        Some("bind/port/db_path changes require process restart")
                                    } else {
                                        None
                                    },
                                )
                                .await;
                            if changed_restart_fields {
                                tracing::warn!("{msg}");
                            } else {
                                tracing::info!("{msg}");
                            }
                        }
                        Err(e) => {
                            tracing::warn!("config reload failed: {e}");
                            let _ = db.insert_event("error","config_reload",None,"config reload failed",Some(&e.to_string())).await;
                        }
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

pub async fn shutdown_signal(shutdown: Arc<AtomicBool>) {
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

pub async fn shutdown_wait(shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::{Snapshot, SystemMetrics, TargetInfo};
    use crate::config::{AlertConfig, Config, Target};
    use tempfile::tempdir;

    async fn test_db() -> (db::Database, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.db");
        let db = db::Database::open(path.to_str().unwrap()).await.unwrap();
        db.init().await.unwrap();
        (db, dir)
    }

    fn base_cfg() -> Config {
        Config {
            alerts: AlertConfig {
                enabled: true,
                cpu_percent_warn: 90.0,
                mem_percent_warn: 90.0,
                disk_percent_warn: 90.0,
                target_latency_ms_warn: 500.0,
                cert_expiry_days_warn: 14,
            },
            targets: vec![Target {
                name: "svc".into(),
                url: "http://127.0.0.1:9".into(),
                kind: Some("generic".into()),
                check_type: "http".into(),
                host: None,
                port: None,
                latency_warn_ms: None,
                cert_expiry_warn_days: None,
            }],
            ..Default::default()
        }
    }

    fn snap(sys: SystemMetrics, targets: Vec<TargetInfo>) -> Snapshot {
        Snapshot {
            timestamp: chrono::Utc::now().to_rfc3339(),
            system: sys,
            targets,
        }
    }

    fn ok_system() -> SystemMetrics {
        SystemMetrics {
            cpu_percent: 10.0,
            mem_used_mb: 100.0,
            mem_total_mb: 1000.0,
            mem_used_percent: 10.0,
            disk_used_mb: 10.0,
            disk_total_mb: 100.0,
            disk_available_mb: 90.0,
            disk_used_percent: 10.0,
            net_rx_bytes: 0,
            net_tx_bytes: 0,
            net_rx_bytes_per_sec: 0.0,
            net_tx_bytes_per_sec: 0.0,
            load1: None,
            load5: None,
            load15: None,
            swap_used_mb: None,
            swap_total_mb: None,
            swap_used_percent: None,
            uptime_seconds: None,
            top_processes: None,
        }
    }

    #[tokio::test]
    async fn evaluate_alerts_fires_resolves_and_dedups() {
        let (db, _dir) = test_db().await;
        let cfg = base_cfg();
        let mut notify_state = NotifyState::default();

        let mut sys = ok_system();
        sys.cpu_percent = 95.0;
        sys.mem_used_percent = 95.0;
        sys.disk_used_percent = 95.0;
        evaluate_alerts(&db, &cfg, &snap(sys, vec![]), &mut notify_state).await;
        let firing = db.list_alerts(Some("firing"), 50).await.unwrap();
        assert!(firing.iter().any(|a| a.name == "cpu_high"));
        assert!(firing.iter().any(|a| a.name == "mem_high"));
        assert!(firing.iter().any(|a| a.name == "disk_high"));

        // duplicate condition should not create duplicate firing alerts
        let mut sys2 = ok_system();
        sys2.cpu_percent = 96.0;
        sys2.mem_used_percent = 96.0;
        sys2.disk_used_percent = 96.0;
        evaluate_alerts(&db, &cfg, &snap(sys2, vec![]), &mut notify_state).await;
        let firing2 = db.list_alerts(Some("firing"), 50).await.unwrap();
        let cpu_count = firing2.iter().filter(|a| a.name == "cpu_high").count();
        assert_eq!(cpu_count, 1);

        // recover -> resolves
        evaluate_alerts(&db, &cfg, &snap(ok_system(), vec![]), &mut notify_state).await;
        let firing3 = db.list_alerts(Some("firing"), 50).await.unwrap();
        assert!(firing3.is_empty());
        let resolved = db.list_alerts(Some("resolved"), 50).await.unwrap();
        assert!(resolved.len() >= 3);
    }

    #[tokio::test]
    async fn evaluate_alerts_target_down_latency_and_cert() {
        let (db, _dir) = test_db().await;
        let cfg = base_cfg();
        let sys = ok_system();
        let mut notify_state = NotifyState::default();

        let t_down = TargetInfo {
            name: "svc".into(),
            kind: "generic".into(),
            url: "http://127.0.0.1:9".into(),
            status: "down".into(),
            latency_ms: Some(800.0),
            http_requests: None,
            detail: None,
            cert_days_left: Some(5),
            check_type: "http".into(),
        };
        evaluate_alerts(
            &db,
            &cfg,
            &snap(sys.clone(), vec![t_down]),
            &mut notify_state,
        )
        .await;
        let firing = db.list_alerts(Some("firing"), 50).await.unwrap();
        assert!(
            firing
                .iter()
                .any(|a| a.name == "down" && a.source == "target:svc")
        );
        assert!(
            firing
                .iter()
                .any(|a| a.name == "latency_high" && a.source == "target:svc")
        );
        assert!(
            firing
                .iter()
                .any(|a| a.name == "cert_expiry" && a.source == "target:svc")
        );

        let t_ok = TargetInfo {
            name: "svc".into(),
            kind: "generic".into(),
            url: "http://127.0.0.1:9".into(),
            status: "up".into(),
            latency_ms: Some(10.0),
            http_requests: None,
            detail: None,
            cert_days_left: Some(90),
            check_type: "http".into(),
        };
        evaluate_alerts(&db, &cfg, &snap(sys, vec![t_ok]), &mut notify_state).await;
        let firing2 = db.list_alerts(Some("firing"), 50).await.unwrap();
        assert!(firing2.is_empty());
    }

    #[test]
    fn notification_transitions_filtering_and_format() {
        let mut cfg = base_cfg();
        cfg.notifications.notify_on_firing = true;
        cfg.notifications.notify_on_resolve = false;
        let transitions = vec![
            AlertTransition {
                status: "firing".into(),
                severity: "warn".into(),
                source: "system".into(),
                name: "cpu_high".into(),
                message: "CPU high".into(),
            },
            AlertTransition {
                status: "resolved".into(),
                severity: "warn".into(),
                source: "system".into(),
                name: "cpu_high".into(),
                message: "CPU high (resolved)".into(),
            },
        ];
        let filtered = filter_notification_transitions(&cfg, &transitions);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status, "firing");
        let msg = format_notification_message(&filtered);
        assert!(msg.contains("[FIRING]"));
        assert!(msg.contains("cpu_high") || msg.contains("CPU"));
    }

    #[test]
    fn notification_cooldown_allows_respects_window() {
        let mut cfg = base_cfg();
        cfg.notifications.cooldown_secs = 60;
        let now = Instant::now();
        let mut st = NotifyState { last_sent: None };
        assert!(notification_cooldown_allows(&cfg, &st, now));
        st.last_sent = Some(now);
        assert!(!notification_cooldown_allows(&cfg, &st, now + Duration::from_secs(30)));
        assert!(notification_cooldown_allows(&cfg, &st, now + Duration::from_secs(61)));

        cfg.notifications.cooldown_secs = 0;
        assert!(notification_cooldown_allows(&cfg, &st, now));
    }
}
