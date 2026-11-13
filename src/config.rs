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
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    #[serde(default)]
    pub alerts: AlertConfig,
    #[serde(default)]
    pub notifications: NotificationConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AuthConfig {
    /// Optional API/UI token. If set, requests must provide Authorization: Bearer <token>
    /// or X-API-Key: <token> (or ?token= for SSE-less UI basics). Health can be exempt via allow_health_unauth.
    pub token: Option<String>,
    #[serde(default)]
    pub basic_user: Option<String>,
    #[serde(default)]
    pub basic_password: Option<String>,
    #[serde(default = "default_true")]
    pub allow_health_unauth: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Target {
    pub name: String,
    pub url: String,
    /// http kind/integration: generic, spring-boot/spring/actuator, quarkus, micrometer, micrometer/prometheus text etc.
    #[serde(default)]
    pub kind: Option<String>,
    /// check type: http (default), tcp, ping, tls
    #[serde(default = "default_check_type")]
    pub check_type: String,
    /// for tcp: host:port or url host/port parsed; for tls: host:port/sni
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    /// optional thresholds/overrides per target
    #[serde(default)]
    pub latency_warn_ms: Option<f64>,
    #[serde(default)]
    pub cert_expiry_warn_days: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlertConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_cpu_warn")]
    pub cpu_percent_warn: f64,
    #[serde(default = "default_mem_warn")]
    pub mem_percent_warn: f64,
    #[serde(default = "default_disk_warn")]
    pub disk_percent_warn: f64,
    #[serde(default = "default_temp_warn")]
    pub temp_celsius_warn: f64,
    #[serde(default = "default_latency_warn")]
    pub target_latency_ms_warn: f64,
    #[serde(default = "default_cert_days_warn")]
    pub cert_expiry_days_warn: i64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cpu_percent_warn: default_cpu_warn(),
            mem_percent_warn: default_mem_warn(),
            disk_percent_warn: default_disk_warn(),
            temp_celsius_warn: default_temp_warn(),
            target_latency_ms_warn: default_latency_warn(),
            cert_expiry_days_warn: default_cert_days_warn(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct NotificationConfig {
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub telegram_bot_token: Option<String>,
    #[serde(default)]
    pub telegram_chat_id: Option<String>,
    #[serde(default)]
    pub ntfy_url: Option<String>,
    #[serde(default)]
    pub ntfy_topic: Option<String>,
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
fn default_check_type() -> String {
    "http".to_string()
}
fn default_true() -> bool {
    true
}
fn default_cpu_warn() -> f64 {
    90.0
}
fn default_mem_warn() -> f64 {
    90.0
}
fn default_disk_warn() -> f64 {
    90.0
}
fn default_temp_warn() -> f64 {
    85.0
}
fn default_latency_warn() -> f64 {
    1000.0
}
fn default_cert_days_warn() -> i64 {
    14
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
                check_type: "http".to_string(),
                host: None,
                port: None,
                latency_warn_ms: None,
                cert_expiry_warn_days: None,
            }],
            auth: None,
            alerts: AlertConfig::default(),
            notifications: NotificationConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RunMode {
    Server,
    Tui,
    CheckConfig,
    Help,
    Version,
}

impl Config {
    pub fn load_with_path(path: Option<PathBuf>) -> anyhow::Result<Self> {
        let path = path
            .or_else(|| std::env::var("TINYTOP_CONFIG").ok().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("config.toml"));

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
        cfg.normalize();
        cfg.validate()?;
        Ok(cfg)
    }

    #[allow(dead_code)]
    pub fn load() -> anyhow::Result<Self> {
        Self::load_with_path(None)
    }

    fn normalize(&mut self) {
        for t in &mut self.targets {
            t.check_type = t.check_type.trim().to_ascii_lowercase();
            if t.check_type.is_empty() {
                t.check_type = "http".to_string();
            }
            if let Some(k) = &mut t.kind {
                *k = k.trim().to_ascii_lowercase();
            }
        }
        if let Some(auth) = &mut self.auth {
            if auth
                .token
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(false)
            {
                auth.token = None;
            }
            if auth
                .basic_user
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(false)
            {
                auth.basic_user = None;
            }
            if auth
                .basic_password
                .as_ref()
                .map(|s| s.is_empty())
                .unwrap_or(false)
            {
                auth.basic_password = None;
            }
        }
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
        if let Ok(v) = std::env::var("TINYTOP_AUTH_TOKEN") {
            let a = self.auth.get_or_insert_with(AuthConfig::default);
            a.token = if v.trim().is_empty() { None } else { Some(v) };
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
            let ct = t.check_type.trim().to_ascii_lowercase();
            match ct.as_str() {
                "http" | "https" => {
                    let url = t.url.trim();
                    if !(url.starts_with("http://") || url.starts_with("https://")) {
                        bail!("target '{}' has invalid url: {}", t.name, t.url);
                    }
                }
                "tcp" => {
                    if t.host.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
                        || t.port.is_none()
                    {
                        // allow url-like host:port in host field? require host+port
                        bail!("tcp target '{}' requires host and port", t.name);
                    }
                }
                "ping" => {
                    if t.host.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
                        && t.url.trim().is_empty()
                    {
                        bail!("ping target '{}' requires host or url/host", t.name);
                    }
                }
                "tls" => {
                    if t.host.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
                        || t.port.is_none()
                    {
                        bail!("tls target '{}' requires host and port", t.name);
                    }
                }
                other => bail!("target '{}' has unsupported check_type '{}'", t.name, other),
            }
            if let Some(kind) = &t.kind
                && kind.trim().is_empty()
            {
                bail!("target '{}' has empty kind", t.name);
            }
        }
        Ok(())
    }

    /// Redacted view safe for /api/config when auth is sensitive (tokens hidden)
    pub fn redacted(&self) -> Self {
        let mut c = self.clone();
        if let Some(a) = &mut c.auth {
            if a.token.is_some() {
                a.token = Some("***".into());
            }
            if a.basic_password.is_some() {
                a.basic_password = Some("***".into());
            }
        }
        if c.notifications.webhook_url.is_some() {
            c.notifications.webhook_url = Some("***".into());
        }
        if c.notifications.telegram_bot_token.is_some() {
            c.notifications.telegram_bot_token = Some("***".into());
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        let cfg = Config::default();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn rejects_duplicate_names() {
        let cfg = Config {
            targets: vec![
                Target {
                    name: "a".into(),
                    url: "http://localhost".into(),
                    kind: None,
                    check_type: "http".into(),
                    host: None,
                    port: None,
                    latency_warn_ms: None,
                    cert_expiry_warn_days: None,
                },
                Target {
                    name: "a".into(),
                    url: "http://localhost".into(),
                    kind: None,
                    check_type: "http".into(),
                    host: None,
                    port: None,
                    latency_warn_ms: None,
                    cert_expiry_warn_days: None,
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
                check_type: "http".into(),
                host: None,
                port: None,
                latency_warn_ms: None,
                cert_expiry_warn_days: None,
            }],
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn accepts_tcp_and_tls_and_ping_targets() {
        let cfg = Config {
            targets: vec![
                Target {
                    name: "tcp1".into(),
                    url: "".into(),
                    kind: None,
                    check_type: "tcp".into(),
                    host: Some("127.0.0.1".into()),
                    port: Some(22),
                    latency_warn_ms: None,
                    cert_expiry_warn_days: None,
                },
                Target {
                    name: "tls1".into(),
                    url: "".into(),
                    kind: None,
                    check_type: "tls".into(),
                    host: Some("example.com".into()),
                    port: Some(443),
                    latency_warn_ms: None,
                    cert_expiry_warn_days: None,
                },
                Target {
                    name: "ping1".into(),
                    url: "".into(),
                    kind: None,
                    check_type: "ping".into(),
                    host: Some("127.0.0.1".into()),
                    port: None,
                    latency_warn_ms: None,
                    cert_expiry_warn_days: None,
                },
            ],
            ..Config::default()
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn rejects_tcp_without_host_port() {
        let cfg = Config {
            targets: vec![Target {
                name: "tcp1".into(),
                url: "".into(),
                kind: None,
                check_type: "tcp".into(),
                host: Some("127.0.0.1".into()),
                port: None,
                latency_warn_ms: None,
                cert_expiry_warn_days: None,
            }],
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_tls_without_host_port_and_ping_without_host() {
        let tls_bad = Config {
            targets: vec![Target {
                name: "tls1".into(),
                url: "".into(),
                kind: None,
                check_type: "tls".into(),
                host: Some("example.com".into()),
                port: None,
                latency_warn_ms: None,
                cert_expiry_warn_days: None,
            }],
            ..Config::default()
        };
        assert!(tls_bad.validate().is_err());

        let ping_bad = Config {
            targets: vec![Target {
                name: "ping1".into(),
                url: "".into(),
                kind: None,
                check_type: "ping".into(),
                host: None,
                port: None,
                latency_warn_ms: None,
                cert_expiry_warn_days: None,
            }],
            ..Config::default()
        };
        assert!(ping_bad.validate().is_err());

        // ping accepted via url fallback
        let ping_ok = Config {
            targets: vec![Target {
                name: "ping2".into(),
                url: "127.0.0.1".into(),
                kind: None,
                check_type: "ping".into(),
                host: None,
                port: None,
                latency_warn_ms: None,
                cert_expiry_warn_days: None,
            }],
            ..Config::default()
        };
        assert!(ping_ok.validate().is_ok());
    }

    #[test]
    fn rejects_unsupported_check_type_and_empty_kind() {
        let bad_type = Config {
            targets: vec![Target {
                name: "x".into(),
                url: "http://localhost".into(),
                kind: None,
                check_type: "udp".into(),
                host: None,
                port: None,
                latency_warn_ms: None,
                cert_expiry_warn_days: None,
            }],
            ..Config::default()
        };
        assert!(bad_type.validate().is_err());

        let bad_kind = Config {
            targets: vec![Target {
                name: "x".into(),
                url: "http://localhost".into(),
                kind: Some("   ".into()),
                check_type: "http".into(),
                host: None,
                port: None,
                latency_warn_ms: None,
                cert_expiry_warn_days: None,
            }],
            ..Config::default()
        };
        assert!(bad_kind.validate().is_err());
    }

    #[test]
    fn redacts_sensitive_auth_and_notifications() {
        let cfg = Config {
            auth: Some(AuthConfig {
                token: Some("secret".into()),
                basic_user: Some("u".into()),
                basic_password: Some("p".into()),
                allow_health_unauth: true,
            }),
            notifications: NotificationConfig {
                webhook_url: Some("https://hook".into()),
                telegram_bot_token: Some("tok".into()),
                telegram_chat_id: Some("123".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let r = cfg.redacted();
        let a = r.auth.unwrap();
        assert_eq!(a.token.as_deref(), Some("***"));
        assert_eq!(a.basic_user.as_deref(), Some("u"));
        assert_eq!(a.basic_password.as_deref(), Some("***"));
        assert_eq!(r.notifications.webhook_url.as_deref(), Some("***"));
        assert_eq!(r.notifications.telegram_bot_token.as_deref(), Some("***"));
        // chat id kept (not secret in same way) but config still usable
        assert_eq!(r.notifications.telegram_chat_id.as_deref(), Some("123"));
    }
}
