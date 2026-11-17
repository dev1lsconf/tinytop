use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

use tinytop::{RuntimeStatus, config::Config, db, tui, web};

#[derive(Debug, Clone)]
struct Cli {
    tui: bool,
    check_config: bool,
    help: bool,
    version: bool,
    config: Option<PathBuf>,
}

fn parse_cli_from<I, S>(args: I) -> anyhow::Result<Cli>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut cli = Cli {
        tui: false,
        check_config: false,
        help: false,
        version: false,
        config: None,
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let a = a.as_ref();
        match a {
            "--tui" => cli.tui = true,
            "--check-config" | "--check" => cli.check_config = true,
            "--help" | "-h" => cli.help = true,
            "--version" | "-V" => cli.version = true,
            "--config" | "-c" => {
                let val = it
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--config requires a path"))?;
                let val = val.as_ref();
                if val.starts_with('-') {
                    anyhow::bail!("--config requires a path");
                }
                cli.config = Some(PathBuf::from(val));
            }
            s if s.starts_with("--config=") => {
                let val = s.trim_start_matches("--config=");
                if val.is_empty() {
                    anyhow::bail!("--config requires a path");
                }
                cli.config = Some(PathBuf::from(val));
            }
            other => {
                anyhow::bail!("unknown argument: {other}");
            }
        }
    }
    Ok(cli)
}

fn parse_cli() -> anyhow::Result<Cli> {
    parse_cli_from(std::env::args().skip(1))
}

fn print_help() {
    println!(
        "TinyTop v{} - lightweight system & target monitor",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Usage: tinytop [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --tui                 Run interactive terminal UI (instead of web server)");
    println!(
        "  --config <PATH>       Path to config file (overrides TINYTOP_CONFIG; default: config.toml)"
    );
    println!("  --check-config        Validate configuration and exit");
    println!("  -h, --help            Print help");
    println!("  -V, --version         Print version");
    println!();
    println!(
        "Environment/config: see config.toml and TINYTOP_* variables. SIGHUP reloads config (Unix)."
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = match parse_cli() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("Try 'tinytop --help' for usage.");
            std::process::exit(2);
        }
    };
    if cli.help {
        print_help();
        return Ok(());
    }
    if cli.version {
        println!("tinytop {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if !cli.tui {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_target(false)
            .compact()
            .init();
    }

    let cfg = Config::load_with_path(cli.config.clone())?;
    if cli.check_config {
        println!(
            "config ok: bind={}:{} db={} targets={} interval={}s retention={}h",
            cfg.bind,
            cfg.port,
            cfg.db_path,
            cfg.targets.len(),
            cfg.interval_seconds,
            cfg.retention_hours
        );
        return Ok(());
    }

    if !cli.tui {
        tracing::info!(version = env!("CARGO_PKG_VERSION"), bind = %cfg.bind, port = cfg.port, db_path = %cfg.db_path,
            interval_seconds = cfg.interval_seconds, retention_hours = cfg.retention_hours, targets = cfg.targets.len(), "starting tinytop");
    }

    let db = Arc::new(db::Database::open(&cfg.db_path).await?);
    db.init().await?;
    if let Err(e) = db.prune_older_than_hours(cfg.retention_hours).await
        && !cli.tui
    {
        tracing::warn!("initial retention prune failed: {e}");
    }

    let cfg_shared = Arc::new(RwLock::new(cfg.clone()));
    let runtime = Arc::new(RwLock::new(RuntimeStatus::new(&cfg)));
    let shutdown = Arc::new(AtomicBool::new(false));

    {
        let db = db.clone();
        let cfg_s = cfg_shared.clone();
        let rt = runtime.clone();
        let sd = shutdown.clone();
        tokio::spawn(async move {
            tinytop::run_collector(db, cfg_s, rt, sd).await;
        });
    }
    {
        let db = db.clone();
        let cfg_s = cfg_shared.clone();
        let sd = shutdown.clone();
        tokio::spawn(async move {
            tinytop::config_reload_loop(db, cfg_s, sd).await;
        });
    }

    if cli.tui {
        tui::run(
            db.clone(),
            cfg_shared.clone(),
            runtime.clone(),
            shutdown.clone(),
        )
        .await?;
        db.pool.close().await;
        return Ok(());
    }

    let app = web::create_router(
        db.clone(),
        cfg_shared.clone(),
        runtime.clone(),
        shutdown.clone(),
    );
    let addr = format!("{}:{}", cfg.bind, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(tinytop::shutdown_signal(shutdown.clone()))
        .await?;

    db.pool.close().await;
    tracing::info!("shutdown complete");
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cli_rejects_unknown_argument() {
        let err = parse_cli_from(["--bogus"]).unwrap_err().to_string();
        assert!(err.contains("unknown argument"));
        assert!(err.contains("--bogus"));
    }

    #[test]
    fn parse_cli_requires_config_value() {
        let err = parse_cli_from(["--config"]).unwrap_err().to_string();
        assert!(err.contains("--config requires a path"));

        let err = parse_cli_from(["--config", "--tui"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("--config requires a path"));

        let err = parse_cli_from(["--config="]).unwrap_err().to_string();
        assert!(err.contains("--config requires a path"));
    }

    #[test]
    fn parse_cli_accepts_config_and_flags() {
        let cli = parse_cli_from(["--config", "foo.toml", "--check-config", "--tui"]).unwrap();
        assert_eq!(cli.config.as_deref(), Some(std::path::Path::new("foo.toml")));
        assert!(cli.check_config);
        assert!(cli.tui);

        let cli = parse_cli_from(["--config=bar.toml"]).unwrap();
        assert_eq!(cli.config.as_deref(), Some(std::path::Path::new("bar.toml")));
    }
}
