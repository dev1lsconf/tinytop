use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    pub cpu: f64,
    pub memory_mb: f64,
    pub disk_mb: f64,
    pub network_rx: u64,
    pub network_tx: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetMetrics {
    pub target: String,
    pub latency_ms: Option<f64>,
    pub http_requests: Option<u64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub timestamp: String,
    pub system: SystemMetrics,
    pub targets: Vec<TargetMetrics>,
}