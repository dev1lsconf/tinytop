use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, FromRow, SqlitePool};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemMetric {
    pub id: i64,
    pub timestamp: String,
    pub cpu_percent: f64,
    pub mem_used_mb: f64,
    pub mem_total_mb: f64,
    pub mem_used_percent: f64,
    pub disk_used_mb: f64,
    pub disk_total_mb: f64,
    pub disk_available_mb: f64,
    pub disk_used_percent: f64,
    pub net_rx_bytes: i64,
    pub net_tx_bytes: i64,
    pub net_rx_bytes_per_sec: f64,
    pub net_tx_bytes_per_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TargetMetric {
    pub id: i64,
    pub timestamp: String,
    pub target: String,
    pub kind: String,
    pub url: String,
    pub status: String,
    pub latency_ms: Option<f64>,
    pub http_requests: Option<i64>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetentionResult {
    pub system_deleted: u64,
    pub target_deleted: u64,
}

impl Database {
    pub async fn open(path: &str) -> anyhow::Result<Self> {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(std::time::Duration::from_secs(5));

        let pool = SqlitePool::connect_with(opts).await?;
        sqlx::query("PRAGMA temp_store = MEMORY")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        sqlx::query("PRAGMA optimize")
            .execute(&self.pool)
            .await
            .ok();
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_system_metric(
        &self,
        cpu_percent: f64,
        mem_used_mb: f64,
        mem_total_mb: f64,
        mem_used_percent: f64,
        disk_used_mb: f64,
        disk_total_mb: f64,
        disk_available_mb: f64,
        disk_used_percent: f64,
        net_rx_bytes: u64,
        net_tx_bytes: u64,
        net_rx_bytes_per_sec: f64,
        net_tx_bytes_per_sec: f64,
    ) -> anyhow::Result<()> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO system_metrics (
                timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(timestamp)
        .bind(cpu_percent)
        .bind(mem_used_mb)
        .bind(mem_total_mb)
        .bind(mem_used_percent)
        .bind(disk_used_mb)
        .bind(disk_total_mb)
        .bind(disk_available_mb)
        .bind(disk_used_percent)
        .bind(net_rx_bytes as i64)
        .bind(net_tx_bytes as i64)
        .bind(net_rx_bytes_per_sec)
        .bind(net_tx_bytes_per_sec)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_target_metric(
        &self,
        target: &str,
        kind: &str,
        url: &str,
        status: &str,
        latency_ms: Option<f64>,
        http_requests: Option<i64>,
        detail: Option<&str>,
    ) -> anyhow::Result<()> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO target_metrics (
                timestamp, target, kind, url, status, latency_ms, http_requests, detail
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(timestamp)
        .bind(target)
        .bind(kind)
        .bind(url)
        .bind(status)
        .bind(latency_ms)
        .bind(http_requests)
        .bind(detail)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn latest_system(&self) -> anyhow::Result<Option<SystemMetric>> {
        let row = sqlx::query_as::<_, SystemMetric>(
            "SELECT id, timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                    disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                    net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec
             FROM system_metrics ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn system_history(&self, range_hours: i64) -> anyhow::Result<Vec<SystemMetric>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        let rows = sqlx::query_as::<_, SystemMetric>(
            "SELECT id, timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                    disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                    net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec
             FROM system_metrics WHERE timestamp >= ? ORDER BY id ASC",
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn latest_target(&self, target: &str) -> anyhow::Result<Option<TargetMetric>> {
        let row = sqlx::query_as::<_, TargetMetric>(
            "SELECT id, timestamp, target, kind, url, status, latency_ms, http_requests, detail
             FROM target_metrics WHERE target = ? ORDER BY id DESC LIMIT 1",
        )
        .bind(target)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn target_history(
        &self,
        target: &str,
        range_hours: i64,
    ) -> anyhow::Result<Vec<TargetMetric>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        let rows = sqlx::query_as::<_, TargetMetric>(
            "SELECT id, timestamp, target, kind, url, status, latency_ms, http_requests, detail
             FROM target_metrics WHERE target = ? AND timestamp >= ? ORDER BY id ASC",
        )
        .bind(target)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn prune_older_than_hours(&self, hours: u64) -> anyhow::Result<RetentionResult> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(hours as i64)).to_rfc3339();
        let s = sqlx::query("DELETE FROM system_metrics WHERE timestamp < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;
        let t = sqlx::query("DELETE FROM target_metrics WHERE timestamp < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;
        Ok(RetentionResult {
            system_deleted: s.rows_affected(),
            target_deleted: t.rows_affected(),
        })
    }

    pub async fn ping(&self) -> anyhow::Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> Database {
        let name = format!("/tmp/tinytop-test-{}.db", uuid_like());
        let db = Database::open(&name).await.unwrap();
        db.init().await.unwrap();
        db
    }

    fn uuid_like() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{}-{}", std::process::id(), nanos)
    }

    #[tokio::test]
    async fn init_and_insert_system_metric() {
        let db = test_db().await;
        db.insert_system_metric(
            10.0, 512.0, 1024.0, 50.0, 1000.0, 2000.0, 1000.0, 50.0, 100, 200, 10.0, 20.0,
        )
        .await
        .unwrap();
        let latest = db.latest_system().await.unwrap().unwrap();
        assert_eq!(latest.cpu_percent, 10.0);
        assert_eq!(latest.mem_used_mb, 512.0);
    }

    #[tokio::test]
    async fn insert_and_query_target_metric() {
        let db = test_db().await;
        db.insert_target_metric(
            "app",
            "generic",
            "http://x",
            "up",
            Some(12.3),
            Some(7),
            None,
        )
        .await
        .unwrap();
        let latest = db.latest_target("app").await.unwrap().unwrap();
        assert_eq!(latest.status, "up");
        assert_eq!(latest.http_requests, Some(7));
    }
}
