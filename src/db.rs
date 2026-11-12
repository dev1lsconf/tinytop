use sqlx::{sqlite::SqliteConnectOptions, FromRow, SqlitePool};
use std::str::FromStr;

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn open(path: &str) -> anyhow::Result<Self> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePool::connect_with(opts).await?;
        Ok(Self { pool })
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS metrics (
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
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_metrics_target_ts ON metrics(target, timestamp)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_metric(
        &self,
        target: &str,
        cpu: f64,
        memory_mb: f64,
        disk_mb: f64,
        network_rx: u64,
        network_tx: u64,
        latency_ms: Option<f64>,
        http_requests: Option<i64>,
    ) -> anyhow::Result<()> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO metrics (target, timestamp, cpu, memory_mb, disk_mb, network_rx, network_tx, latency_ms, http_requests)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(target)
        .bind(timestamp)
        .bind(cpu)
        .bind(memory_mb)
        .bind(disk_mb)
        .bind(network_rx as i64)
        .bind(network_tx as i64)
        .bind(latency_ms)
        .bind(http_requests)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn latest(&self, target: &str) -> anyhow::Result<Option<MetricsRow>> {
        let row = sqlx::query_as::<_, MetricsRow>(
            "SELECT id, target, timestamp, cpu, memory_mb, disk_mb, network_rx, network_tx, latency_ms, http_requests
             FROM metrics WHERE target = ? ORDER BY id DESC LIMIT 1",
        )
        .bind(target)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn history(&self, target: &str, range_hours: i64) -> anyhow::Result<Vec<MetricsRow>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        let rows = sqlx::query_as::<_, MetricsRow>(
            "SELECT id, target, timestamp, cpu, memory_mb, disk_mb, network_rx, network_tx, latency_ms, http_requests
             FROM metrics WHERE target = ? AND timestamp >= ? ORDER BY id ASC",
        )
        .bind(target)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[derive(Debug, Clone, FromRow, serde::Serialize, serde::Deserialize)]
pub struct MetricsRow {
    pub id: i64,
    pub target: String,
    pub timestamp: String,
    pub cpu: f64,
    pub memory_mb: f64,
    pub disk_mb: f64,
    pub network_rx: i64,
    pub network_tx: i64,
    pub latency_ms: Option<f64>,
    pub http_requests: Option<i64>,
}