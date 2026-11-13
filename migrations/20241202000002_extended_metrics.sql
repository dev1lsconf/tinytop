-- Extended system metrics, rollups, alerts/incidents, and events

ALTER TABLE system_metrics ADD COLUMN load1 REAL;
ALTER TABLE system_metrics ADD COLUMN load5 REAL;
ALTER TABLE system_metrics ADD COLUMN load15 REAL;
ALTER TABLE system_metrics ADD COLUMN swap_used_mb REAL;
ALTER TABLE system_metrics ADD COLUMN swap_total_mb REAL;
ALTER TABLE system_metrics ADD COLUMN swap_used_percent REAL;
ALTER TABLE system_metrics ADD COLUMN uptime_seconds INTEGER;
ALTER TABLE system_metrics ADD COLUMN disk_read_bytes INTEGER;
ALTER TABLE system_metrics ADD COLUMN disk_write_bytes INTEGER;
ALTER TABLE system_metrics ADD COLUMN disk_read_bytes_per_sec REAL;
ALTER TABLE system_metrics ADD COLUMN disk_write_bytes_per_sec REAL;
ALTER TABLE system_metrics ADD COLUMN temp_celsius REAL;
ALTER TABLE system_metrics ADD COLUMN top_processes TEXT;

ALTER TABLE target_metrics ADD COLUMN cert_days_left INTEGER;
ALTER TABLE target_metrics ADD COLUMN check_type TEXT;

CREATE TABLE IF NOT EXISTS system_metrics_rollup (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bucket_start TEXT NOT NULL,
    bucket_seconds INTEGER NOT NULL,
    samples INTEGER NOT NULL,
    cpu_avg REAL, cpu_max REAL,
    mem_avg REAL, mem_max REAL,
    disk_avg REAL, disk_max REAL,
    net_rx_bps_avg REAL, net_tx_bps_avg REAL,
    load1_avg REAL, load1_max REAL,
    temp_avg REAL, temp_max REAL,
    UNIQUE(bucket_start, bucket_seconds)
);

CREATE INDEX IF NOT EXISTS idx_system_rollup_bucket ON system_metrics_rollup(bucket_seconds, bucket_start);

CREATE TABLE IF NOT EXISTS alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL,
    resolved_at TEXT,
    status TEXT NOT NULL, -- firing, resolved
    severity TEXT NOT NULL,
    source TEXT NOT NULL, -- system, target:<name>
    name TEXT NOT NULL,
    message TEXT NOT NULL,
    value REAL,
    threshold REAL
);

CREATE INDEX IF NOT EXISTS idx_alerts_status_created ON alerts(status, created_at);
CREATE INDEX IF NOT EXISTS idx_alerts_source_created ON alerts(source, created_at);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    level TEXT NOT NULL,
    kind TEXT NOT NULL,
    source TEXT,
    message TEXT NOT NULL,
    detail TEXT
);

CREATE INDEX IF NOT EXISTS idx_events_ts ON events(timestamp);
