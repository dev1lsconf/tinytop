use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
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

fn default_port() -> u16 {
    9090
}

fn default_db_path() -> String {
    "tinytop.db".to_string()
}

fn default_interval() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: default_port(),
            db_path: default_db_path(),
            interval_seconds: default_interval(),
            targets: vec![Target {
                name: "localhost".to_string(),
                url: "http://localhost:8080/actuator/metrics".to_string(),
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

        if path.exists() {
            let contents = std::fs::read_to_string(&path)?;
            let mut parsed: Config = toml::from_str(&contents)?;
            if parsed.targets.is_empty() {
                parsed.targets = Config::default().targets;
            }
            Ok(parsed)
        } else {
            tracing::warn!("Config file {:?} not found, using defaults", path);
            Ok(Config::default())
        }
    }
}