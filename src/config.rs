use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_retention_hours")]
    pub retention_hours: u64,
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,
    #[serde(default = "default_max_history_hours")]
    pub max_history_hours: i64,
    #[serde(default)]
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Target {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub kind: Option<String>,
}

fn default_bind() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    9090
}
fn default_db_path() -> String {
    "tinytop.db".to_string()
}
fn default_interval() -> u64 {
    30
}
fn default_retention_hours() -> u64 {
    168
}
fn default_request_timeout_secs() -> u64 {
    5
}
fn default_max_history_hours() -> i64 {
    168
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            port: default_port(),
            db_path: default_db_path(),
            interval_seconds: default_interval(),
            retention_hours: default_retention_hours(),
            request_timeout_secs: default_request_timeout_secs(),
            max_history_hours: default_max_history_hours(),
            targets: vec![Target {
                name: "localhost".to_string(),
                url: "http://localhost:8080/actuator/health".to_string(),
                kind: Some("generic".to_string()),
            }],
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = std::env::var("TINYTOP_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("config.toml"));

        let mut cfg = if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read config file {path:?}"))?;
            toml::from_str::<Config>(&contents)
                .with_context(|| format!("failed to parse config file {path:?}"))?
        } else {
            tracing::warn!("Config file {:?} not found, using defaults", path);
            Config::default()
        };

        cfg.apply_env_overrides()?;
        if cfg.targets.is_empty() {
            cfg.targets = Config::default().targets;
        }
        cfg.validate()?;
        Ok(cfg)
    }

    fn apply_env_overrides(&mut self) -> anyhow::Result<()> {
        if let Ok(v) = std::env::var("TINYTOP_BIND") {
            self.bind = v;
        }
        if let Ok(v) = std::env::var("TINYTOP_PORT") {
            self.port = v.parse().context("invalid TINYTOP_PORT")?;
        }
        if let Ok(v) = std::env::var("TINYTOP_DB_PATH") {
            self.db_path = v;
        }
        if let Ok(v) = std::env::var("TINYTOP_INTERVAL") {
            self.interval_seconds = v.parse().context("invalid TINYTOP_INTERVAL")?;
        }
        if let Ok(v) = std::env::var("TINYTOP_RETENTION_HOURS") {
            self.retention_hours = v.parse().context("invalid TINYTOP_RETENTION_HOURS")?;
        }
        if let Ok(v) = std::env::var("TINYTOP_REQUEST_TIMEOUT_SECS") {
            self.request_timeout_secs =
                v.parse().context("invalid TINYTOP_REQUEST_TIMEOUT_SECS")?;
        }
        if let Ok(v) = std::env::var("TINYTOP_MAX_HISTORY_HOURS") {
            self.max_history_hours = v.parse().context("invalid TINYTOP_MAX_HISTORY_HOURS")?;
        }
        Ok(())
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.bind.trim().is_empty() {
            bail!("bind must not be empty");
        }
        if self.port == 0 {
            bail!("port must be greater than 0");
        }
        if self.interval_seconds == 0 {
            bail!("interval_seconds must be greater than 0");
        }
        if self.request_timeout_secs == 0 {
            bail!("request_timeout_secs must be greater than 0");
        }
        if self.retention_hours == 0 {
            bail!("retention_hours must be greater than 0");
        }
        if self.max_history_hours <= 0 {
            bail!("max_history_hours must be greater than 0");
        }
        if self.db_path.trim().is_empty() {
            bail!("db_path must not be empty");
        }

        let mut names = HashSet::new();
        for t in &self.targets {
            let name = t.name.trim();
            if name.is_empty() {
                bail!("target name must not be empty");
            }
            if !names.insert(name.to_string()) {
                bail!("duplicate target name: {}", t.name);
            }
            let url = t.url.trim();
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                bail!("target '{}' has invalid url: {}", t.name, t.url);
            }
            if let Some(kind) = &t.kind {
                if kind.trim().is_empty() {
                    bail!("target '{}' has empty kind", t.name);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        let cfg = Config::default();
        assert!(cfg.validate().is_ok());
        assert!(cfg.port > 0);
        assert!(!cfg.targets.is_empty());
    }

    #[test]
    fn rejects_duplicate_names() {
        let cfg = Config {
            targets: vec![
                Target {
                    name: "a".into(),
                    url: "http://localhost".into(),
                    kind: None,
                },
                Target {
                    name: "a".into(),
                    url: "http://localhost".into(),
                    kind: None,
                },
            ],
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_invalid_url() {
        let cfg = Config {
            targets: vec![Target {
                name: "a".into(),
                url: "ftp://localhost".into(),
                kind: None,
            }],
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }
}
