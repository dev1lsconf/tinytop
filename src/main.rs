mod collector;
mod config;
mod db;
mod web;

use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = config::Config::load()?;
    let db = Arc::new(db::Database::open(&cfg.db_path).await?);
    db.init().await?;

    let db_for_collector = db.clone();
    let cfg_for_collector = cfg.clone();
    tokio::spawn(async move {
        loop {
            match collector::collect(&cfg_for_collector).await {
                Ok(snap) => {
                    if let Err(e) = db_for_collector
                        .insert_metric(
                            "system",
                            snap.system.cpu,
                            snap.system.memory_mb,
                            snap.system.disk_mb,
                            snap.system.network_rx,
                            snap.system.network_tx,
                            None,
                            None,
                        )
                        .await
                    {
                        tracing::error!("failed to insert system metrics: {e}");
                    }

                    for t in &snap.targets {
                        if let Err(e) = db_for_collector
                            .insert_metric(
                                &t.name,
                                snap.system.cpu,
                                snap.system.memory_mb,
                                snap.system.disk_mb,
                                snap.system.network_rx,
                                snap.system.network_tx,
                                t.latency_ms,
                                t.http_requests,
                            )
                            .await
                        {
                            tracing::error!("failed to insert metrics for {}: {e}", t.name);
                        }
                    }
                }
                Err(e) => tracing::error!("collect error: {e}"),
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(cfg_for_collector.interval_seconds)).await;
        }
    });

    let app = web::create_router(db, cfg.clone());
    let addr = format!("0.0.0.0:{}", cfg.port);
    tracing::info!("TinyTop listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}