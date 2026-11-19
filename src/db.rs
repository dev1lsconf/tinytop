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

#[derive(Debug, Clone, Serialize)]
pub struct SystemHistoryPoint {
    pub timestamp: String,
    pub origin: String, // "raw" | "rollup"
    pub bucket_seconds: Option<i64>,
    pub samples: Option<i64>,
    pub cpu_percent: f64,
    pub mem_used_mb: Option<f64>,
    pub mem_total_mb: Option<f64>,
    pub mem_used_percent: f64,
    pub disk_used_mb: Option<f64>,
    pub disk_total_mb: Option<f64>,
    pub disk_available_mb: Option<f64>,
    pub disk_used_percent: f64,
    pub net_rx_bytes: Option<i64>,
    pub net_tx_bytes: Option<i64>,
    pub net_rx_bytes_per_sec: f64,
    pub net_tx_bytes_per_sec: f64,
    pub load1: Option<f64>,
    pub load5: Option<f64>,
    pub load15: Option<f64>,
    pub swap_used_mb: Option<f64>,
    pub swap_total_mb: Option<f64>,
    pub swap_used_percent: Option<f64>,
    pub uptime_seconds: Option<i64>,
    pub top_processes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetHistoryPoint {
    pub timestamp: String,
    pub origin: String, // "raw" | "rollup"
    pub bucket_seconds: Option<i64>,
    pub samples: Option<i64>,
    pub target: String,
    pub status: Option<String>,
    pub up_ratio: Option<f64>,
    pub latency_ms: Option<f64>,
    pub http_requests: Option<i64>,
    pub cert_days_left: Option<i64>,
    pub detail: Option<String>,
    pub check_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BucketCount {
    pub bucket_start: String,
    pub key: String,
    pub count: i64,
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
        top_processes: Option<&str>,
    ) -> anyhow::Result<()> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO system_metrics (
                timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec,
                load1, load5, load15, swap_used_mb, swap_total_mb, swap_used_percent,
                uptime_seconds, top_processes
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
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
        .bind(load1)
        .bind(load5)
        .bind(load15)
        .bind(swap_used_mb)
        .bind(swap_total_mb)
        .bind(swap_used_percent)
        .bind(uptime_seconds)
        .bind(top_processes)
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
                    uptime_seconds, top_processes
             FROM system_metrics ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn system_history_points(
        &self,
        range_hours: i64,
    ) -> anyhow::Result<Vec<SystemHistoryPoint>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        // Downsample long ranges using rollups when available to keep responses small.
        if range_hours > 24 {
            let bucket: i64 = if range_hours > 72 { 3600 } else { 300 };
            type RollupRow = (String, i64, f64, f64, f64, f64, f64, Option<f64>);
            let rows: Vec<RollupRow> = sqlx::query_as(
                "SELECT bucket_start, samples,
                        COALESCE(cpu_avg,0.0), COALESCE(mem_avg,0.0), COALESCE(disk_avg,0.0),
                        COALESCE(net_rx_bps_avg,0.0), COALESCE(net_tx_bps_avg,0.0), load1_avg
                 FROM system_metrics_rollup
                 WHERE bucket_seconds = ? AND bucket_start >= ?
                 ORDER BY bucket_start ASC",
            )
            .bind(bucket)
            .bind(&cutoff)
            .fetch_all(&self.pool)
            .await?;
            if !rows.is_empty() {
                return Ok(rows
                    .into_iter()
                    .map(
                        |(timestamp, samples, cpu, mem, disk, rx, tx, load1)| SystemHistoryPoint {
                            timestamp,
                            origin: "rollup".to_string(),
                            bucket_seconds: Some(bucket),
                            samples: Some(samples),
                            cpu_percent: cpu,
                            mem_used_mb: None,
                            mem_total_mb: None,
                            mem_used_percent: mem,
                            disk_used_mb: None,
                            disk_total_mb: None,
                            disk_available_mb: None,
                            disk_used_percent: disk,
                            net_rx_bytes: None,
                            net_tx_bytes: None,
                            net_rx_bytes_per_sec: rx,
                            net_tx_bytes_per_sec: tx,
                            load1,
                            load5: None,
                            load15: None,
                            swap_used_mb: None,
                            swap_total_mb: None,
                            swap_used_percent: None,
                            uptime_seconds: None,
                            top_processes: None,
                        },
                    )
                    .collect());
            }

            // Fallback: aggregate raw samples on-the-fly so long ranges stay bounded
            // even if rollups are not yet available for this window.
            let bucket_expr = if bucket >= 3600 {
                "strftime('%Y-%m-%dT%H:00:00Z', timestamp)".to_string()
            } else {
                "strftime('%Y-%m-%dT%H:', timestamp) || printf('%02d:00Z', (CAST(strftime('%M', timestamp) AS INTEGER) / 5) * 5)".to_string()
            };
            let sql = format!(
                "SELECT {bucket_expr} as bucket_start, COUNT(*) as samples,
                        COALESCE(AVG(cpu_percent),0.0), COALESCE(AVG(mem_used_percent),0.0), COALESCE(AVG(disk_used_percent),0.0),
                        COALESCE(AVG(net_rx_bytes_per_sec),0.0), COALESCE(AVG(net_tx_bytes_per_sec),0.0), AVG(load1)
                 FROM system_metrics WHERE timestamp >= ?
                 GROUP BY bucket_start
                 ORDER BY bucket_start ASC"
            );
            let agg_rows: Vec<RollupRow> = sqlx::query_as(&sql)
                .bind(&cutoff)
                .fetch_all(&self.pool)
                .await?;
            if !agg_rows.is_empty() {
                return Ok(agg_rows
                    .into_iter()
                    .map(
                        |(timestamp, samples, cpu, mem, disk, rx, tx, load1)| SystemHistoryPoint {
                            timestamp,
                            origin: "rollup".to_string(),
                            bucket_seconds: Some(bucket),
                            samples: Some(samples),
                            cpu_percent: cpu,
                            mem_used_mb: None,
                            mem_total_mb: None,
                            mem_used_percent: mem,
                            disk_used_mb: None,
                            disk_total_mb: None,
                            disk_available_mb: None,
                            disk_used_percent: disk,
                            net_rx_bytes: None,
                            net_tx_bytes: None,
                            net_rx_bytes_per_sec: rx,
                            net_tx_bytes_per_sec: tx,
                            load1,
                            load5: None,
                            load15: None,
                            swap_used_mb: None,
                            swap_total_mb: None,
                            swap_used_percent: None,
                            uptime_seconds: None,
                            top_processes: None,
                        },
                    )
                    .collect());
            }
        }
        let rows = self.system_history(range_hours).await?;
        Ok(rows
            .into_iter()
            .map(|r| SystemHistoryPoint {
                timestamp: r.timestamp,
                origin: "raw".to_string(),
                bucket_seconds: None,
                samples: None,
                cpu_percent: r.cpu_percent,
                mem_used_mb: Some(r.mem_used_mb),
                mem_total_mb: Some(r.mem_total_mb),
                mem_used_percent: r.mem_used_percent,
                disk_used_mb: Some(r.disk_used_mb),
                disk_total_mb: Some(r.disk_total_mb),
                disk_available_mb: Some(r.disk_available_mb),
                disk_used_percent: r.disk_used_percent,
                net_rx_bytes: Some(r.net_rx_bytes),
                net_tx_bytes: Some(r.net_tx_bytes),
                net_rx_bytes_per_sec: r.net_rx_bytes_per_sec,
                net_tx_bytes_per_sec: r.net_tx_bytes_per_sec,
                load1: r.load1,
                load5: r.load5,
                load15: r.load15,
                swap_used_mb: r.swap_used_mb,
                swap_total_mb: r.swap_total_mb,
                swap_used_percent: r.swap_used_percent,
                uptime_seconds: r.uptime_seconds,
                top_processes: r.top_processes,
            })
            .collect())
    }

    pub async fn system_history(&self, range_hours: i64) -> anyhow::Result<Vec<SystemMetric>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        // Raw points for compatibility; downsampled presentation uses system_history_points.
        let rows = sqlx::query_as::<_, SystemMetric>(
            "SELECT id, timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                    disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                    net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec,
                    load1, load5, load15, swap_used_mb, swap_total_mb, swap_used_percent,
                    uptime_seconds, top_processes
             FROM system_metrics WHERE timestamp >= ? ORDER BY id ASC",
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;
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

    pub async fn target_history_points(
        &self,
        target: &str,
        range_hours: i64,
    ) -> anyhow::Result<Vec<TargetHistoryPoint>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        if range_hours > 24 {
            let bucket: i64 = if range_hours > 72 { 3600 } else { 300 };
            let bucket_expr = if bucket >= 3600 {
                "strftime('%Y-%m-%dT%H:00:00Z', timestamp)".to_string()
            } else {
                "strftime('%Y-%m-%dT%H:', timestamp) || printf('%02d:00Z', (CAST(strftime('%M', timestamp) AS INTEGER) / 5) * 5)".to_string()
            };
            let sql = format!(
                "SELECT {bucket_expr} as bucket_start, COUNT(*) as samples,
                        AVG(latency_ms) as latency_avg,
                        AVG(CASE WHEN status IN ('up','ok') THEN 1.0 ELSE 0.0 END) as up_ratio,
                        MIN(cert_days_left) as cert_min,
                        MAX(check_type) as check_type
                 FROM target_metrics WHERE target = ? AND timestamp >= ?
                 GROUP BY bucket_start
                 ORDER BY bucket_start ASC"
            );
            type Row = (
                String,
                i64,
                Option<f64>,
                Option<f64>,
                Option<i64>,
                Option<String>,
            );
            let rows: Vec<Row> = sqlx::query_as(&sql)
                .bind(target)
                .bind(&cutoff)
                .fetch_all(&self.pool)
                .await?;
            return Ok(rows
                .into_iter()
                .map(
                    |(ts, samples, latency_avg, up_ratio, cert_min, check_type)| {
                        TargetHistoryPoint {
                            timestamp: ts,
                            origin: "rollup".to_string(),
                            bucket_seconds: Some(bucket),
                            samples: Some(samples),
                            target: target.to_string(),
                            status: None,
                            up_ratio,
                            latency_ms: latency_avg,
                            http_requests: None,
                            cert_days_left: cert_min,
                            detail: None,
                            check_type,
                        }
                    },
                )
                .collect());
        }
        let rows = self.target_history(target, range_hours).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let up = if matches!(r.status.as_str(), "up" | "ok") {
                    1.0
                } else {
                    0.0
                };
                TargetHistoryPoint {
                    timestamp: r.timestamp,
                    origin: "raw".to_string(),
                    bucket_seconds: None,
                    samples: None,
                    target: r.target,
                    status: Some(r.status),
                    up_ratio: Some(up),
                    latency_ms: r.latency_ms,
                    http_requests: r.http_requests,
                    cert_days_left: r.cert_days_left,
                    detail: r.detail,
                    check_type: r.check_type,
                }
            })
            .collect())
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
        // Recompute from the latest existing bucket start (inclusive window by
        // re-aggregating that bucket) so late samples and a corrected bucket
        // boundary update the rollup instead of being skipped forever.
        let last: Option<String> = sqlx::query_scalar(
            "SELECT MAX(bucket_start) FROM system_metrics_rollup WHERE bucket_seconds=?",
        )
        .bind(bucket_seconds)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(None);
        let since = last.unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        // SQLite: strftime-based bucketing on ISO timestamps. For 5m buckets we must
        // truncate to 5-minute boundaries (not just zero seconds).
        let bucket_expr = if bucket_seconds >= 3600 {
            "strftime('%Y-%m-%dT%H:00:00Z', timestamp)".to_string()
        } else if bucket_seconds >= 300 {
            "strftime('%Y-%m-%dT%H:', timestamp) || printf('%02d:00Z', (CAST(strftime('%M', timestamp) AS INTEGER) / 5) * 5)".to_string()
        } else {
            "strftime('%Y-%m-%dT%H:%M:%SZ', timestamp)".to_string()
        };
        let sql = format!(
            "INSERT OR REPLACE INTO system_metrics_rollup
             (bucket_start, bucket_seconds, samples, cpu_avg,cpu_max, mem_avg,mem_max, disk_avg,disk_max, net_rx_bps_avg, net_tx_bps_avg, load1_avg, load1_max)
             SELECT {bucket_expr} as b, ? as bs, COUNT(*),
                    AVG(cpu_percent), MAX(cpu_percent),
                    AVG(mem_used_percent), MAX(mem_used_percent),
                    AVG(disk_used_percent), MAX(disk_used_percent),
                    AVG(net_rx_bytes_per_sec), AVG(net_tx_bytes_per_sec),
                    AVG(load1), MAX(load1)
             FROM system_metrics WHERE timestamp >= ?
             GROUP BY b"
        );
        let res = sqlx::query(&sql)
            .bind(bucket_seconds)
            .bind(since)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }

    pub async fn event_counts_by_bucket(
        &self,
        range_hours: i64,
    ) -> anyhow::Result<Vec<BucketCount>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        let bucket: i64 = if range_hours > 72 {
            3600
        } else if range_hours > 24 {
            1800
        } else {
            900
        };
        let bucket_expr = if bucket >= 3600 {
            "strftime('%Y-%m-%dT%H:00:00Z', timestamp)"
        } else {
            "strftime('%Y-%m-%dT%H:', timestamp) || printf('%02d:00Z', (CAST(strftime('%M', timestamp) AS INTEGER) / 15) * 15)"
        };
        let bucket_expr = if bucket == 1800 {
            "strftime('%Y-%m-%dT%H:', timestamp) || printf('%02d:00Z', (CAST(strftime('%M', timestamp) AS INTEGER) / 30) * 30)".to_string()
        } else {
            bucket_expr.to_string()
        };
        let sql = format!(
            "SELECT {bucket_expr} as bucket_start, level as key, COUNT(*) as count
             FROM events WHERE timestamp >= ?
             GROUP BY bucket_start, key
             ORDER BY bucket_start ASC"
        );
        let rows: Vec<(String, String, i64)> = sqlx::query_as(&sql)
            .bind(&cutoff)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(bucket_start, key, count)| BucketCount {
                bucket_start,
                key,
                count,
            })
            .collect())
    }

    pub async fn alert_counts_by_bucket(
        &self,
        range_hours: i64,
    ) -> anyhow::Result<Vec<BucketCount>> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(range_hours)).to_rfc3339();
        let bucket: i64 = if range_hours > 72 {
            3600
        } else if range_hours > 24 {
            1800
        } else {
            900
        };
        let bucket_expr = if bucket >= 3600 {
            "strftime('%Y-%m-%dT%H:00:00Z', created_at)"
        } else {
            "strftime('%Y-%m-%dT%H:', created_at) || printf('%02d:00Z', (CAST(strftime('%M', created_at) AS INTEGER) / 15) * 15)"
        };
        let bucket_expr = if bucket == 1800 {
            "strftime('%Y-%m-%dT%H:', created_at) || printf('%02d:00Z', (CAST(strftime('%M', created_at) AS INTEGER) / 30) * 30)".to_string()
        } else {
            bucket_expr.to_string()
        };
        let sql = format!(
            "SELECT {bucket_expr} as bucket_start, status as key, COUNT(*) as count
             FROM alerts WHERE created_at >= ?
             GROUP BY bucket_start, key
             ORDER BY bucket_start ASC"
        );
        let rows: Vec<(String, String, i64)> = sqlx::query_as(&sql)
            .bind(&cutoff)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(bucket_start, key, count)| BucketCount {
                bucket_start,
                key,
                count,
            })
            .collect())
    }

    // --- exports ---
    pub async fn export_system_history(
        &self,
        range_hours: i64,
    ) -> anyhow::Result<Vec<SystemMetric>> {
        self.system_history(range_hours).await
    }
    pub async fn export_system_history_points(
        &self,
        range_hours: i64,
    ) -> anyhow::Result<Vec<SystemHistoryPoint>> {
        self.system_history_points(range_hours).await
    }
    pub async fn export_target_history(
        &self,
        target: &str,
        range_hours: i64,
    ) -> anyhow::Result<Vec<TargetMetric>> {
        self.target_history(target, range_hours).await
    }
    pub async fn export_target_history_points(
        &self,
        target: &str,
        range_hours: i64,
    ) -> anyhow::Result<Vec<TargetHistoryPoint>> {
        self.target_history_points(target, range_hours).await
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::Database;
    use tempfile::TempDir;

    pub async fn open_temp_db() -> (Database, TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        let db = Database::open(path.to_str().expect("temp path utf-8"))
            .await
            .expect("open temp db");
        db.init().await.expect("init temp db");
        (db, dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::open_temp_db;

    async fn test_db() -> (Database, tempfile::TempDir) {
        open_temp_db().await
    }
    #[tokio::test]
    async fn init_and_insert_system_metric() {
        let (db, _dir) = test_db().await;
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
        )
        .await
        .unwrap();
        let latest = db.latest_system().await.unwrap().unwrap();
        assert_eq!(latest.cpu_percent, 10.0);
    }
    #[tokio::test]
    async fn insert_and_query_target_metric() {
        let (db, _dir) = test_db().await;
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
        let (db, _dir) = test_db().await;
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
        )
        .await
        .unwrap();
        db.insert_system_metric(
            20.0,
            600.0,
            1024.0,
            60.0,
            1200.0,
            2000.0,
            800.0,
            60.0,
            200,
            300,
            11.0,
            22.0,
            Some(0.8),
            Some(0.6),
            Some(0.4),
            Some(128.0),
            Some(256.0),
            Some(50.0),
            Some(200),
            None,
        )
        .await
        .unwrap();
        let created = db.compute_rollups(300).await.unwrap();
        assert!(created >= 1);
        let hist = db.system_history(48).await.unwrap();
        assert!(!hist.is_empty());
        let points = db.system_history_points(48).await.unwrap();
        assert!(!points.is_empty());
        // range > 24 with rollups returns downsampled rollup points
        assert_eq!(points[0].origin, "rollup");
        assert_eq!(points[0].bucket_seconds, Some(300));
        assert!(points[0].samples.unwrap_or(0) >= 1);
        // non-aggregated fields must not be misleading zeros
        assert!(points[0].mem_used_mb.is_none());
        assert!(points[0].disk_available_mb.is_none());
        assert!(points[0].net_rx_bytes.is_none());
        assert!(points[0].load5.is_none());
        assert!(points[0].uptime_seconds.is_none());

        let pruned = db.prune_older_than_hours(9999).await.unwrap();
        assert_eq!(pruned.system_deleted, 0);
        assert_eq!(pruned.target_deleted, 0);
    }

    #[tokio::test]
    async fn rollups_bucket_five_minutes() {
        let (db, _dir) = test_db().await;
        // Three samples within the same 5-minute window and one in the next window.
        for (ts, cpu) in [
            ("2026-11-16T10:01:00Z", 10.0),
            ("2026-11-16T10:03:00Z", 20.0),
            ("2026-11-16T10:04:30Z", 30.0),
            ("2026-11-16T10:07:00Z", 40.0),
        ] {
            sqlx::query(
                "INSERT INTO system_metrics (
                    timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                    disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                    net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec,
                    load1, load5, load15, swap_used_mb, swap_total_mb, swap_used_percent,
                    uptime_seconds, top_processes
                ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            )
            .bind(ts)
            .bind(cpu)
            .bind(512.0)
            .bind(1024.0)
            .bind(50.0)
            .bind(1000.0)
            .bind(2000.0)
            .bind(1000.0)
            .bind(50.0)
            .bind(100_i64)
            .bind(200_i64)
            .bind(10.0)
            .bind(20.0)
            .bind(Some(0.5))
            .bind(Some(0.4))
            .bind(Some(0.3))
            .bind(Some(0.0))
            .bind(Some(0.0))
            .bind(Some(0.0))
            .bind(Some(100_i64))
            .bind::<Option<String>>(None)
            .execute(&db.pool)
            .await
            .unwrap();
        }

        let created = db.compute_rollups(300).await.unwrap();
        assert_eq!(created, 2);

        let rows: Vec<(String, i64, f64)> = sqlx::query_as(
            "SELECT bucket_start, samples, cpu_avg FROM system_metrics_rollup WHERE bucket_seconds = 300 ORDER BY bucket_start ASC",
        )
        .fetch_all(&db.pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "2026-11-16T10:00:00Z");
        assert_eq!(rows[0].1, 3);
        assert!((rows[0].2 - 20.0).abs() < 1e-9);
        assert_eq!(rows[1].0, "2026-11-16T10:05:00Z");
        assert_eq!(rows[1].1, 1);
        assert!((rows[1].2 - 40.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn history_points_falls_back_to_bucket_aggregation_without_rollups() {
        let (db, _dir) = test_db().await;
        let base = chrono::Utc::now() - chrono::Duration::hours(30);
        let aligned =
            chrono::DateTime::<chrono::Utc>::from_timestamp((base.timestamp() / 300) * 300, 0)
                .unwrap();
        let t = |secs: i64| (aligned + chrono::Duration::seconds(secs)).to_rfc3339();
        // 60s, 180s, 240s -> same 5m bucket; 420s -> next 5m bucket
        for (ts, cpu) in [
            (t(60), 10.0),
            (t(180), 20.0),
            (t(240), 30.0),
            (t(420), 40.0),
        ] {
            sqlx::query(
                "INSERT INTO system_metrics (
                    timestamp, cpu_percent, mem_used_mb, mem_total_mb, mem_used_percent,
                    disk_used_mb, disk_total_mb, disk_available_mb, disk_used_percent,
                    net_rx_bytes, net_tx_bytes, net_rx_bytes_per_sec, net_tx_bytes_per_sec,
                    load1, load5, load15, swap_used_mb, swap_total_mb, swap_used_percent,
                    uptime_seconds, top_processes
                ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            )
            .bind(ts)
            .bind(cpu)
            .bind(512.0)
            .bind(1024.0)
            .bind(50.0)
            .bind(1000.0)
            .bind(2000.0)
            .bind(1000.0)
            .bind(50.0)
            .bind(100_i64)
            .bind(200_i64)
            .bind(10.0)
            .bind(20.0)
            .bind(Some(0.5_f64))
            .bind(Some(0.4_f64))
            .bind(Some(0.3_f64))
            .bind(Some(0.0_f64))
            .bind(Some(0.0_f64))
            .bind(Some(0.0_f64))
            .bind(Some(100_i64))
            .bind::<Option<String>>(None)
            .execute(&db.pool)
            .await
            .unwrap();
        }

        let points = db.system_history_points(48).await.unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].origin, "rollup");
        assert_eq!(points[0].bucket_seconds, Some(300));
        assert_eq!(points[0].samples, Some(3));
        assert!((points[0].cpu_percent - 20.0).abs() < 1e-9);
        assert!(points[0].mem_used_mb.is_none());
        assert!(points[0].disk_available_mb.is_none());
        assert!(points[0].net_rx_bytes.is_none());
        assert!(points[0].load5.is_none());
        assert!(points[0].uptime_seconds.is_none());
        assert_eq!(points[1].samples, Some(1));
        assert!((points[1].cpu_percent - 40.0).abs() < 1e-9);
    }
}
