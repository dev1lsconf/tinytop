use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, sqlite::SqliteConnectOptions};
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
    // extended
    pub load1: Option<f64>,
    pub load5: Option<f64>,
    pub load15: Option<f64>,
    pub swap_used_mb: Option<f64>,
    pub swap_total_mb: Option<f64>,
    pub swap_used_percent: Option<f64>,
    pub uptime_seconds: Option<i64>,
    pub disk_read_bytes: Option<i64>,
    pub disk_write_bytes: Option<i64>,
    pub disk_read_bytes_per_sec: Option<f64>,
    pub disk_write_bytes_per_sec: Option<f64>,
    pub temp_celsius: Option<f64>,
    pub top_processes: Option<String>,
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
    pub cert_days_left: Option<i64>,
    pub check_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertRow {
    pub id: i64,
    pub created_at: String,
    pub resolved_at: Option<String>,
    pub status: String,
    pub severity: String,
    pub source: String,
    pub name: String,
    pub message: String,
    pub value: Option<f64>,
    pub threshold: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EventRow {
    pub id: i64,
    pub timestamp: String,
    pub level: String,
    pub kind: String,
    pub source: Option<String>,
    pub message: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetentionResult {
    pub system_deleted: u64,
    pub target_deleted: u64,
    pub rollups_deleted: u64,
    pub alerts_deleted: u64,
    pub events_deleted: u64,
}

impl Database {
    pub async fn open(path: &str) -> anyhow::Result<Self> {
        if let Some(parent) = Path::new(path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
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
        load1: Option<f64>,
        load5: Option<f64>,
        load15: Option<f64>,
        swap_used_mb: Option<f64>,
        swap_total_mb: Option<f64>,
        swap_used_percent: Option<f64>,
        uptime_seconds: Option<i64>,
        disk_read_bytes: Option<u64>,
        disk_write_bytes: Option<u64>,
        disk_read_bytes_per_sec: Option<f64>,
        disk_write_bytes_per_sec: Option<f64>,
        temp_celsius: Option<f64>,
        top_processes: Option<&str>,
    ) -> anyhow::Result<()> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO system_metrics (
                timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec,
                load1, load5, load15, swap_used_mb, swap_total_mb, swap_used_percent,
                uptime_seconds, disk_read_bytes, disk_write_bytes, disk_read_bytes_per_sec, disk_write_bytes_per_sec,
                temp_celsius, top_processes
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(timestamp)
        .bind(cpu_percent).bind(mem_used_mb).bind(mem_total_mb).bind(mem_used_percent)
        .bind(disk_used_mb).bind(disk_total_mb).bind(disk_available_mb).bind(disk_used_percent)
        .bind(net_rx_bytes as i64).bind(net_tx_bytes as i64)
        .bind(net_rx_bytes_per_sec).bind(net_tx_bytes_per_sec)
        .bind(load1).bind(load5).bind(load15)
        .bind(swap_used_mb).bind(swap_total_mb).bind(swap_used_percent)
        .bind(uptime_seconds)
        .bind(disk_read_bytes.map(|v| v as i64)).bind(disk_write_bytes.map(|v| v as i64))
        .bind(disk_read_bytes_per_sec).bind(disk_write_bytes_per_sec)
        .bind(temp_celsius).bind(top_processes)
        .execute(&self.pool).await?;
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
        cert_days_left: Option<i64>,
        check_type: Option<&str>,
    ) -> anyhow::Result<()> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO target_metrics (
                timestamp, target, kind, url, status, latency_ms, http_requests, detail, cert_days_left, check_type
            ) VALUES (?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(timestamp).bind(target).bind(kind).bind(url).bind(status)
        .bind(latency_ms).bind(http_requests).bind(detail)
        .bind(cert_days_left).bind(check_type)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn latest_system(&self) -> anyhow::Result<Option<SystemMetric>> {
        let row = sqlx::query_as::<_, SystemMetric>(
            "SELECT id, timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                    disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                    net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec,
                    load1, load5, load15, swap_used_mb, swap_total_mb, swap_used_percent,
                    uptime_seconds, disk_read_bytes, disk_write_bytes, disk_read_bytes_per_sec, disk_write_bytes_per_sec,
                    temp_celsius, top_processes
             FROM system_metrics ORDER BY id DESC LIMIT 1",
        ).fetch_optional(&self.pool).await?;
        Ok(row)
    }

    pub async fn system_history(&self, range_hours: i64) -> anyhow::Result<Vec<SystemMetric>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        // Downsample long ranges using rollups when available to keep responses small.
        if range_hours > 24 {
            let bucket: i64 = if range_hours > 72 { 3600 } else { 300 };
            let rows = sqlx::query_as::<_, SystemMetric>(
                "SELECT 0 as id, bucket_start as timestamp,
                        COALESCE(cpu_avg,0.0) as cpu_percent,
                        0.0 as mem_used_mb, 0.0 as mem_total_mb, COALESCE(mem_avg,0.0) as mem_used_percent,
                        0.0 as disk_used_mb, 0.0 as disk_total_mb, 0.0 as disk_available_mb, COALESCE(disk_avg,0.0) as disk_used_percent,
                        0 as net_rx_bytes, 0 as net_tx_bytes, COALESCE(net_rx_bps_avg,0.0) as net_rx_bytes_per_sec, COALESCE(net_tx_bps_avg,0.0) as net_tx_bytes_per_sec,
                        load1_avg as load1, NULL as load5, NULL as load15,
                        NULL as swap_used_mb, NULL as swap_total_mb, NULL as swap_used_percent,
                        NULL as uptime_seconds, NULL as disk_read_bytes, NULL as disk_write_bytes, NULL as disk_read_bytes_per_sec, NULL as disk_write_bytes_per_sec,
                        temp_avg as temp_celsius, NULL as top_processes
                 FROM system_metrics_rollup
                 WHERE bucket_seconds = ? AND bucket_start >= ?
                 ORDER BY bucket_start ASC",
            )
            .bind(bucket)
            .bind(&cutoff)
            .fetch_all(&self.pool)
            .await?;
            if !rows.is_empty() {
                return Ok(rows);
            }
        }
        let rows = sqlx::query_as::<_, SystemMetric>(
            "SELECT id, timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                    disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                    net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec,
                    load1, load5, load15, swap_used_mb, swap_total_mb, swap_used_percent,
                    uptime_seconds, disk_read_bytes, disk_write_bytes, disk_read_bytes_per_sec, disk_write_bytes_per_sec,
                    temp_celsius, top_processes
             FROM system_metrics WHERE timestamp >= ? ORDER BY id ASC",
        ).bind(cutoff).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn latest_target(&self, target: &str) -> anyhow::Result<Option<TargetMetric>> {
        let row = sqlx::query_as::<_, TargetMetric>(
            "SELECT id, timestamp, target, kind, url, status, latency_ms, http_requests, detail, cert_days_left, check_type
             FROM target_metrics WHERE target = ? ORDER BY id DESC LIMIT 1",
        ).bind(target).fetch_optional(&self.pool).await?;
        Ok(row)
    }

    pub async fn target_history(
        &self,
        target: &str,
        range_hours: i64,
    ) -> anyhow::Result<Vec<TargetMetric>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        let rows = sqlx::query_as::<_, TargetMetric>(
            "SELECT id, timestamp, target, kind, url, status, latency_ms, http_requests, detail, cert_days_left, check_type
             FROM target_metrics WHERE target = ? AND timestamp >= ? ORDER BY id ASC",
        ).bind(target).bind(cutoff).fetch_all(&self.pool).await?;
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
        let r = sqlx::query("DELETE FROM system_metrics_rollup WHERE bucket_start < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .ok();
        let a = sqlx::query("DELETE FROM alerts WHERE created_at < ? AND status = 'resolved'")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .ok();
        let e = sqlx::query("DELETE FROM events WHERE timestamp < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await
            .ok();
        Ok(RetentionResult {
            system_deleted: s.rows_affected(),
            target_deleted: t.rows_affected(),
            rollups_deleted: r.map(|x| x.rows_affected()).unwrap_or(0),
            alerts_deleted: a.map(|x| x.rows_affected()).unwrap_or(0),
            events_deleted: e.map(|x| x.rows_affected()).unwrap_or(0),
        })
    }

    pub async fn ping(&self) -> anyhow::Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    // --- alerts/events ---
    pub async fn insert_event(
        &self,
        level: &str,
        kind: &str,
        source: Option<&str>,
        message: &str,
        detail: Option<&str>,
    ) -> anyhow::Result<()> {
        let ts = chrono::Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO events (timestamp, level, kind, source, message, detail) VALUES (?,?,?,?,?,?)")
            .bind(ts).bind(level).bind(kind).bind(source).bind(message).bind(detail)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_events(&self, limit: i64) -> anyhow::Result<Vec<EventRow>> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT id, timestamp, level, kind, source, message, detail FROM events ORDER BY id DESC LIMIT ?",
        ).bind(limit).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn fire_alert(
        &self,
        severity: &str,
        source: &str,
        name: &str,
        message: &str,
        value: Option<f64>,
        threshold: Option<f64>,
    ) -> anyhow::Result<bool> {
        let existing = sqlx::query(
            "SELECT id FROM alerts WHERE status='firing' AND source=? AND name=? LIMIT 1",
        )
        .bind(source)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        if existing.is_none() {
            let ts = chrono::Utc::now().to_rfc3339();
            sqlx::query("INSERT INTO alerts (created_at, status, severity, source, name, message, value, threshold) VALUES (?,?,?,?,?,?,?,?)")
                .bind(ts).bind("firing").bind(severity).bind(source).bind(name).bind(message).bind(value).bind(threshold)
                .execute(&self.pool).await?;
            self.insert_event("warn", "alert_firing", Some(source), message, None)
                .await
                .ok();
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn resolve_alert(&self, source: &str, name: &str) -> anyhow::Result<bool> {
        let ts = chrono::Utc::now().to_rfc3339();
        let res = sqlx::query("UPDATE alerts SET status='resolved', resolved_at=? WHERE status='firing' AND source=? AND name=?")
            .bind(ts).bind(source).bind(name).execute(&self.pool).await?;
        let changed = res.rows_affected() > 0;
        if changed {
            self.insert_event(
                "info",
                "alert_resolved",
                Some(source),
                &format!("{name} resolved"),
                None,
            )
            .await
            .ok();
        }
        Ok(changed)
    }

    pub async fn list_alerts(
        &self,
        status: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<AlertRow>> {
        let rows = if let Some(st) = status {
            sqlx::query_as::<_, AlertRow>("SELECT id, created_at, resolved_at, status, severity, source, name, message, value, threshold FROM alerts WHERE status=? ORDER BY id DESC LIMIT ?")
                .bind(st).bind(limit).fetch_all(&self.pool).await?
        } else {
            sqlx::query_as::<_, AlertRow>("SELECT id, created_at, resolved_at, status, severity, source, name, message, value, threshold FROM alerts ORDER BY id DESC LIMIT ?")
                .bind(limit).fetch_all(&self.pool).await?
        };
        Ok(rows)
    }

    // --- rollups (simple bucketed aggregation) ---
    pub async fn compute_rollups(&self, bucket_seconds: i64) -> anyhow::Result<u64> {
        // aggregate raw system_metrics into buckets newer than latest rollup bucket for this size
        let last: Option<String> = sqlx::query_scalar(
            "SELECT MAX(bucket_start) FROM system_metrics_rollup WHERE bucket_seconds=?",
        )
        .bind(bucket_seconds)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(None);
        let since = last.unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        // SQLite: strftime-based bucketing on ISO timestamps
        let fmt = if bucket_seconds >= 3600 {
            "%Y-%m-%dT%H:00:00Z"
        } else if bucket_seconds >= 300 {
            "%Y-%m-%dT%H:%M:00Z"
        } else {
            "%Y-%m-%dT%H:%M:%SZ"
        };
        let res = sqlx::query(
            "INSERT OR REPLACE INTO system_metrics_rollup
             (bucket_start, bucket_seconds, samples, cpu_avg,cpu_max, mem_avg,mem_max, disk_avg,disk_max, net_rx_bps_avg, net_tx_bps_avg, load1_avg, load1_max, temp_avg, temp_max)
             SELECT strftime(?, timestamp) as b, ? as bs, COUNT(*),
                    AVG(cpu_percent), MAX(cpu_percent),
                    AVG(mem_used_percent), MAX(mem_used_percent),
                    AVG(disk_used_percent), MAX(disk_used_percent),
                    AVG(net_rx_bytes_per_sec), AVG(net_tx_bytes_per_sec),
                    AVG(load1), MAX(load1),
                    AVG(temp_celsius), MAX(temp_celsius)
             FROM system_metrics WHERE timestamp > ?
             GROUP BY b"
        ).bind(fmt).bind(bucket_seconds).bind(since).execute(&self.pool).await?;
        Ok(res.rows_affected())
    }

    // --- exports ---
    pub async fn export_system_history(
        &self,
        range_hours: i64,
    ) -> anyhow::Result<Vec<SystemMetric>> {
        self.system_history(range_hours).await
    }
    pub async fn export_target_history(
        &self,
        target: &str,
        range_hours: i64,
    ) -> anyhow::Result<Vec<TargetMetric>> {
        self.target_history(target, range_hours).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn test_db() -> Database {
        let path = std::env::temp_dir().join(format!("tinytop-test-{}.db", uuid_like()));
        let db = Database::open(path.to_str().expect("temp path utf-8"))
            .await
            .unwrap();
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
            10.0,
            512.0,
            1024.0,
            50.0,
            1000.0,
            2000.0,
            1000.0,
            50.0,
            100,
            200,
            10.0,
            20.0,
            Some(0.5),
            Some(0.4),
            Some(0.3),
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some(100),
            None,
            None,
            None,
            None,
            Some(40.0),
            None,
        )
        .await
        .unwrap();
        let latest = db.latest_system().await.unwrap().unwrap();
        assert_eq!(latest.cpu_percent, 10.0);
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
            None,
            Some("http"),
        )
        .await
        .unwrap();
        let latest = db.latest_target("app").await.unwrap().unwrap();
        assert_eq!(latest.status, "up");
    }

    #[tokio::test]
    async fn alerts_events_and_rollups_work() {
        let db = test_db().await;
        assert!(
            db.fire_alert("warn", "system", "cpu", "high", Some(95.0), Some(90.0))
                .await
                .unwrap()
        );
        // firing again should not duplicate
        assert!(
            !db.fire_alert("warn", "system", "cpu", "high", Some(96.0), Some(90.0))
                .await
                .unwrap()
        );
        let alerts = db.list_alerts(Some("firing"), 10).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert!(db.resolve_alert("system", "cpu").await.unwrap());
        let firing = db.list_alerts(Some("firing"), 10).await.unwrap();
        assert!(firing.is_empty());
        db.insert_event("info", "test", Some("src"), "msg", None)
            .await
            .unwrap();
        let events = db.list_events(10).await.unwrap();
        assert!(!events.is_empty());
        db.insert_system_metric(
            10.0,
            512.0,
            1024.0,
            50.0,
            1000.0,
            2000.0,
            1000.0,
            50.0,
            100,
            200,
            10.0,
            20.0,
            Some(0.5),
            Some(0.4),
            Some(0.3),
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some(100),
            None,
            None,
            None,
            None,
            Some(40.0),
            None,
        )
        .await
        .unwrap();
        let _ = db.compute_rollups(300).await.unwrap();
    }
}
