CREATE TABLE IF NOT EXISTS system_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    cpu_percent REAL NOT NULL,
    mem_used_mb REAL NOT NULL,
    mem_total_mb REAL NOT NULL,
    mem_used_percent REAL NOT NULL,
    disk_used_mb REAL NOT NULL,
    disk_total_mb REAL NOT NULL,
    disk_available_mb REAL NOT NULL,
    disk_used_percent REAL NOT NULL,
    net_rx_bytes INTEGER NOT NULL,
    net_tx_bytes INTEGER NOT NULL,
    net_rx_bytes_per_sec REAL NOT NULL,
    net_tx_bytes_per_sec REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS target_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    target TEXT NOT NULL,
    kind TEXT NOT NULL,
    url TEXT NOT NULL,
    status TEXT NOT NULL,
    latency_ms REAL,
    http_requests INTEGER,
    detail TEXT
);

CREATE INDEX IF NOT EXISTS idx_system_metrics_ts ON system_metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_target_metrics_target_ts ON target_metrics(target, timestamp);
CREATE INDEX IF NOT EXISTS idx_target_metrics_ts ON target_metrics(timestamp);