use crate::config::Config;
use serde::Serialize;
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemMetrics {
    pub cpu: f64,
    pub memory_mb: f64,
    pub disk_mb: f64,
    pub network_rx: u64,
    pub network_tx: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetInfo {
    pub name: String,
    pub status: String,
    pub latency_ms: Option<f64>,
    pub http_requests: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub system: SystemMetrics,
    pub targets: Vec<TargetInfo>,
}

pub async fn collect(config: &Config) -> anyhow::Result<Snapshot> {
    let system = collect_system();
    let targets = collect_targets(config).await;
    Ok(Snapshot { system, targets })
}

fn collect_system() -> SystemMetrics {
    let mut sys = System::new_all();
    sys.refresh_all();

    // sysinfo 0.33: global CPU usage requires refreshed CPUs
    let cpu = sys.global_cpu_usage() as f64;

    // Report used memory (MB) for UI progress bars
    let memory_mb = sys.used_memory() as f64 / 1024.0 / 1024.0;

    let disks = Disks::new_with_refreshed_list();
    let disk_mb = disks
        .iter()
        .map(|d| d.available_space() as f64 / 1024.0 / 1024.0)
        .sum::<f64>();

    let networks = Networks::new_with_refreshed_list();
    let mut network_rx = 0u64;
    let mut network_tx = 0u64;
    for (_name, data) in &networks {
        network_rx = network_rx.saturating_add(data.total_received());
        network_tx = network_tx.saturating_add(data.total_transmitted());
    }

    SystemMetrics {
        cpu,
        memory_mb,
        disk_mb,
        network_rx,
        network_tx,
    }
}

async fn collect_targets(config: &Config) -> Vec<TargetInfo> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let mut results = Vec::with_capacity(config.targets.len());
    for target in &config.targets {
        let start = std::time::Instant::now();
        match client.get(&target.url).send().await {
            Ok(resp) => {
                let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
                let status = if resp.status().is_success() {
                    "up".to_string()
                } else {
                    format!("down:{}", resp.status().as_u16())
                };
                results.push(TargetInfo {
                    name: target.name.clone(),
                    status,
                    latency_ms: Some(latency_ms),
                    http_requests: None,
                });
            }
            Err(e) => {
                results.push(TargetInfo {
                    name: target.name.clone(),
                    status: format!("error:{}", e),
                    latency_ms: None,
                    http_requests: None,
                });
            }
        }
    }
    results
}