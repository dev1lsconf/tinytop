CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    cpu REAL NOT NULL,
    memory_mb REAL NOT NULL,
    disk_mb REAL NOT NULL,
    network_rx INTEGER NOT NULL,
    network_tx INTEGER NOT NULL,
    latency_ms REAL,
    http_requests INTEGER
);

CREATE INDEX IF NOT EXISTS idx_metrics_target_ts ON metrics(target, timestamp);