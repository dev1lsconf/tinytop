use crate::config::Config;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, System};

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemMetrics {
    pub cpu_percent: f64,
    pub mem_used_mb: f64,
    pub mem_total_mb: f64,
    pub mem_used_percent: f64,
    pub disk_used_mb: f64,
    pub disk_total_mb: f64,
    pub disk_available_mb: f64,
    pub disk_used_percent: f64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
    pub net_rx_bytes_per_sec: f64,
    pub net_tx_bytes_per_sec: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetInfo {
    pub name: String,
    pub kind: String,
    pub url: String,
    pub status: String,
    pub latency_ms: Option<f64>,
    pub http_requests: Option<i64>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub timestamp: String,
    pub system: SystemMetrics,
    pub targets: Vec<TargetInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct CollectorState {
    last_net: Option<(u64, u64, Instant)>,
}

pub async fn collect(config: &Config, state: &mut CollectorState) -> anyhow::Result<Snapshot> {
    let system = collect_system(state);
    let targets = collect_targets(config).await;
    Ok(Snapshot {
        timestamp: chrono::Utc::now().to_rfc3339(),
        system,
        targets,
    })
}

fn http_client(timeout_secs: u64) -> reqwest::Client {
    HTTP_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(timeout_secs.max(1)))
                .pool_max_idle_per_host(4)
                .tcp_keepalive(Duration::from_secs(30))
                .user_agent(format!("tinytop/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("failed to build http client")
        })
        .clone()
}

fn collect_system(state: &mut CollectorState) -> SystemMetrics {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_percent = f64::from(sys.global_cpu_usage());
    let mem_total = sys.total_memory() as f64 / 1024.0 / 1024.0;
    let mem_used = sys.used_memory() as f64 / 1024.0 / 1024.0;
    let mem_used_percent = if mem_total > 0.0 {
        (mem_used / mem_total) * 100.0
    } else {
        0.0
    };

    let disks = Disks::new_with_refreshed_list();
    let mut disk_total = 0.0f64;
    let mut disk_available = 0.0f64;
    let mut seen_devices = HashSet::new();
    for d in &disks {
        // Deduplicate by device name to avoid double-counting bind mounts/subvolumes
        // (e.g. btrfs subvols or Docker bind mounts of the same underlying filesystem).
        let name = d.name().to_string_lossy().to_string();
        if !name.is_empty() && !seen_devices.insert(name) {
            continue;
        }
        disk_total += d.total_space() as f64 / 1024.0 / 1024.0;
        disk_available += d.available_space() as f64 / 1024.0 / 1024.0;
    }
    let disk_used = (disk_total - disk_available).max(0.0);
    let disk_used_percent = if disk_total > 0.0 {
        (disk_used / disk_total) * 100.0
    } else {
        0.0
    };

    let networks = Networks::new_with_refreshed_list();
    let mut net_rx_bytes = 0u64;
    let mut net_tx_bytes = 0u64;
    for (_name, data) in &networks {
        net_rx_bytes = net_rx_bytes.saturating_add(data.total_received());
        net_tx_bytes = net_tx_bytes.saturating_add(data.total_transmitted());
    }

    let now = Instant::now();
    let (net_rx_bytes_per_sec, net_tx_bytes_per_sec) = if let Some((prx, ptx, at)) = state.last_net
    {
        let dt = now.duration_since(at).as_secs_f64().max(0.001);
        let rx_delta = net_rx_bytes.saturating_sub(prx);
        let tx_delta = net_tx_bytes.saturating_sub(ptx);
        (rx_delta as f64 / dt, tx_delta as f64 / dt)
    } else {
        (0.0, 0.0)
    };
    state.last_net = Some((net_rx_bytes, net_tx_bytes, now));

    SystemMetrics {
        cpu_percent,
        mem_used_mb: mem_used,
        mem_total_mb: mem_total,
        mem_used_percent,
        disk_used_mb: disk_used,
        disk_total_mb: disk_total,
        disk_available_mb: disk_available,
        disk_used_percent,
        net_rx_bytes,
        net_tx_bytes,
        net_rx_bytes_per_sec,
        net_tx_bytes_per_sec,
    }
}

async fn collect_targets(config: &Config) -> Vec<TargetInfo> {
    let client = http_client(config.request_timeout_secs);
    let mut set = tokio::task::JoinSet::new();
    for target in config.targets.clone() {
        let client = client.clone();
        set.spawn(async move { probe_target(&client, &target).await });
    }

    let mut results = Vec::with_capacity(config.targets.len());
    while let Some(res) = set.join_next().await {
        match res {
            Ok(info) => results.push(info),
            Err(e) => {
                tracing::warn!(error = %e, "target probe task failed");
            }
        }
    }
    // keep stable order by configured name for UI/API consistency
    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

async fn probe_target(client: &reqwest::Client, target: &crate::config::Target) -> TargetInfo {
    let kind = target.kind.clone().unwrap_or_else(|| "generic".to_string());
    let start = Instant::now();
    match client.get(&target.url).send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            let status_code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let (status, http_requests, detail) = interpret_response(&kind, status_code, &body);
            TargetInfo {
                name: target.name.clone(),
                kind,
                url: target.url.clone(),
                status,
                latency_ms: Some(latency_ms),
                http_requests,
                detail,
            }
        }
        Err(e) => TargetInfo {
            name: target.name.clone(),
            kind,
            url: target.url.clone(),
            status: "error".to_string(),
            latency_ms: None,
            http_requests: None,
            detail: Some(shorten(&e.to_string(), 180)),
        },
    }
}

fn interpret_response(
    kind: &str,
    status_code: reqwest::StatusCode,
    body: &str,
) -> (String, Option<i64>, Option<String>) {
    let ok_http = status_code.is_success();
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();

    let mut http_requests = extract_requests_count(parsed.as_ref());
    let mut detail = None;

    let status = match kind.to_ascii_lowercase().as_str() {
        "spring-boot" | "spring" | "actuator" => {
            interpret_spring(ok_http, parsed.as_ref(), &mut detail)
        }
        "quarkus" | "micronaut" | "micrometer" => {
            interpret_micrometer_like(ok_http, parsed.as_ref(), &mut detail)
        }
        _ => interpret_generic(ok_http, parsed.as_ref(), &mut detail),
    };

    // if parser found request counters in plain-text metrics payloads
    if http_requests.is_none() {
        http_requests = extract_text_metrics_requests(body);
    }

    if detail.is_none() && !ok_http {
        detail = Some(format!("http {}", status_code.as_u16()));
    }

    (status, http_requests, detail)
}

fn interpret_spring(
    ok_http: bool,
    parsed: Option<&serde_json::Value>,
    detail: &mut Option<String>,
) -> String {
    if let Some(v) = parsed {
        if let Some(s) = v.get("status").and_then(|x| x.as_str()) {
            *detail = Some(format!("status={s}"));
            return match s.to_ascii_uppercase().as_str() {
                "UP" => "up".to_string(),
                "DOWN" | "OUT_OF_SERVICE" => "down".to_string(),
                _ if ok_http => "up".to_string(),
                _ => "degraded".to_string(),
            };
        }
        // actuator metrics endpoint can return {"name":"...","measurements":[{"statistic":"COUNT","value":N}]}
        if v.get("name").is_some() && v.get("measurements").is_some() {
            *detail = v
                .get("name")
                .and_then(|x| x.as_str())
                .map(|n| format!("metric={n}"));
            return if ok_http {
                "up".to_string()
            } else {
                "down".to_string()
            };
        }
    }
    interpret_generic(ok_http, parsed, detail)
}

fn interpret_micrometer_like(
    ok_http: bool,
    parsed: Option<&serde_json::Value>,
    detail: &mut Option<String>,
) -> String {
    if let Some(v) = parsed {
        if let Some(status) = v.get("status").and_then(|x| x.as_str()) {
            *detail = Some(format!("status={status}"));
            return if status.eq_ignore_ascii_case("up") || status.eq_ignore_ascii_case("ok") {
                "up".to_string()
            } else if ok_http {
                "degraded".to_string()
            } else {
                "down".to_string()
            };
        }
        if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
            *detail = Some(format!("metric={name}"));
        }
    }
    if ok_http {
        "up".to_string()
    } else {
        "down".to_string()
    }
}

fn interpret_generic(
    ok_http: bool,
    parsed: Option<&serde_json::Value>,
    detail: &mut Option<String>,
) -> String {
    if let Some(v) = parsed {
        if let Some(s) = v.get("status").and_then(|x| x.as_str()) {
            *detail = Some(format!("status={s}"));
            let up = matches!(
                s.to_ascii_lowercase().as_str(),
                "up" | "ok" | "healthy" | "pass" | "available"
            );
            let down = matches!(
                s.to_ascii_lowercase().as_str(),
                "down" | "error" | "fail" | "unhealthy"
            );
            return if up {
                "up".to_string()
            } else if down {
                "down".to_string()
            } else if ok_http {
                "degraded".to_string()
            } else {
                "down".to_string()
            };
        }
    }
    if ok_http {
        "up".to_string()
    } else {
        "down".to_string()
    }
}

fn extract_requests_count(v: Option<&serde_json::Value>) -> Option<i64> {
    let v = v?;
    // Actuator metric: measurements[{statistic:"COUNT", value: N}] or find any COUNT
    if let Some(measurements) = v.get("measurements").and_then(|m| m.as_array()) {
        for m in measurements {
            let stat = m.get("statistic").and_then(|s| s.as_str()).unwrap_or("");
            if stat.eq_ignore_ascii_case("count") || stat.eq_ignore_ascii_case("value") {
                if let Some(n) = m.get("value").and_then(as_i64) {
                    return Some(n);
                }
            }
        }
        if let Some(n) = measurements
            .first()
            .and_then(|m| m.get("value"))
            .and_then(as_i64)
        {
            return Some(n);
        }
    }

    for key in [
        "requests",
        "request_count",
        "requestCount",
        "http_requests",
        "httpRequests",
        "count",
        "total",
    ] {
        if let Some(n) = v.get(key).and_then(as_i64) {
            return Some(n);
        }
    }

    // nested common shapes
    if let Some(n) = v
        .get("metrics")
        .and_then(|m| m.get("requests"))
        .and_then(as_i64)
    {
        return Some(n);
    }
    None
}

fn extract_text_metrics_requests(body: &str) -> Option<i64> {
    // very small heuristic for text metrics formats
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.contains("http_server_requests_seconds_count")
            || line.contains("http_requests_total")
            || line.contains("requests_count")
        {
            if let Some(val) = line.split_whitespace().last() {
                if let Ok(f) = val.parse::<f64>() {
                    if f.is_finite() && f >= 0.0 {
                        return Some(f as i64);
                    }
                }
            }
        }
    }
    None
}

fn as_i64(v: &serde_json::Value) -> Option<i64> {
    match v {
        serde_json::Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().map(|u| u.min(i64::MAX as u64) as i64))
            .or_else(|| n.as_f64().map(|f| f as i64)),
        serde_json::Value::String(s) => s
            .parse::<i64>()
            .ok()
            .or_else(|| s.parse::<f64>().ok().map(|f| f as i64)),
        _ => None,
    }
}

fn shorten(s: &str, max: usize) -> String {
    let s = s.trim().replace('\n', " ");
    if s.chars().count() <= max {
        s
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_actuator_count() {
        let v: serde_json::Value = serde_json::json!({
            "name":"http.server.requests",
            "measurements":[{"statistic":"COUNT","value":42.0},{"statistic":"TOTAL_TIME","value":1.0}]
        });
        assert_eq!(extract_requests_count(Some(&v)), Some(42));
    }

    #[test]
    fn interprets_generic_up() {
        let (st, _, _) =
            interpret_response("generic", reqwest::StatusCode::OK, "{\"status\":\"ok\"}");
        assert_eq!(st, "up");
    }
}
