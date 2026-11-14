use crate::config::{Config, Target};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
#[cfg(not(target_os = "openbsd"))]
use sysinfo::{Components, Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemMetrics {
    pub cpu_percent: f64,
    pub mem_used_mb: f64,
    pub mem_total_mb: f64,
    pub mem_used_percent: f64,
    pub disk_used_mb: f64,
    pub disk_total_mb: f64,
    pub disk_available_mb: f64,
    pub disk_used_percent: f64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
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
    pub disk_read_bytes: Option<u64>,
    pub disk_write_bytes: Option<u64>,
    pub disk_read_bytes_per_sec: Option<f64>,
    pub disk_write_bytes_per_sec: Option<f64>,
    pub temp_celsius: Option<f64>,
    pub top_processes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetInfo {
    pub name: String,
    pub kind: String,
    pub url: String,
    pub status: String,
    pub latency_ms: Option<f64>,
    pub http_requests: Option<i64>,
    pub detail: Option<String>,
    pub cert_days_left: Option<i64>,
    pub check_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub timestamp: String,
    pub system: SystemMetrics,
    pub targets: Vec<TargetInfo>,
}

#[cfg(not(target_os = "openbsd"))]
pub struct CollectorState {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    last_net: Option<(u64, u64, Instant)>,
    last_disk_io: Option<(u64, u64, Instant)>,
}

#[cfg(target_os = "openbsd")]
#[derive(Default)]
pub struct CollectorState {
    last_net: Option<(u64, u64, Instant)>,
    last_cpu: Option<(u64, u64, Instant)>, // busy, total, at
    last_disk_io: Option<(u64, u64, Instant)>,
}

#[cfg(not(target_os = "openbsd"))]
impl Default for CollectorState {
    fn default() -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_all();
        Self {
            sys,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            last_net: None,
            last_disk_io: None,
        }
    }
}

impl std::fmt::Debug for CollectorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectorState").finish_non_exhaustive()
    }
}

pub async fn collect(config: &Config, state: &mut CollectorState) -> anyhow::Result<Snapshot> {
    let system = collect_system(state);
    let targets = collect_targets(config).await;
    Ok(Snapshot {
        timestamp: chrono::Utc::now().to_rfc3339(),
        system,
        targets,
    })
}

fn http_client(timeout_secs: u64) -> reqwest::Client {
    HTTP_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(timeout_secs.max(1)))
                .pool_max_idle_per_host(4)
                .tcp_keepalive(Duration::from_secs(30))
                .user_agent(format!("tinytop/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("failed to build http client")
        })
        .clone()
}

fn collect_system(state: &mut CollectorState) -> SystemMetrics {
    #[cfg(target_os = "openbsd")]
    {
        collect_system_openbsd(state)
    }
    #[cfg(not(target_os = "openbsd"))]
    {
        collect_system_sysinfo(state)
    }
}

#[cfg(not(target_os = "openbsd"))]
fn collect_system_sysinfo(state: &mut CollectorState) -> SystemMetrics {
    state.sys.refresh_cpu_all();
    state.sys.refresh_memory();
    state.sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything()
            .without_disk_usage()
            .without_user(),
    );
    state.disks.refresh(true);
    state.networks.refresh(true);
    state.components.refresh(true);

    let cpu_percent = f64::from(state.sys.global_cpu_usage());
    let mem_total = state.sys.total_memory() as f64 / 1024.0 / 1024.0;
    let mem_used = state.sys.used_memory() as f64 / 1024.0 / 1024.0;
    let mem_used_percent = if mem_total > 0.0 {
        (mem_used / mem_total) * 100.0
    } else {
        0.0
    };

    let swap_total = state.sys.total_swap() as f64 / 1024.0 / 1024.0;
    let swap_used = state.sys.used_swap() as f64 / 1024.0 / 1024.0;
    let swap_used_percent = if swap_total > 0.0 {
        Some((swap_used / swap_total) * 100.0)
    } else {
        None
    };

    let load = System::load_average();
    let uptime_seconds = Some(System::uptime() as i64);

    let mut disk_total = 0.0f64;
    let mut disk_available = 0.0f64;
    let mut seen_devices = HashSet::new();
    for d in &state.disks {
        let fs = d.file_system().to_string_lossy().to_ascii_lowercase();
        if matches!(
            fs.as_str(),
            "tmpfs"
                | "devtmpfs"
                | "overlay"
                | "squashfs"
                | "proc"
                | "procfs"
                | "sysfs"
                | "cgroup"
                | "cgroup2"
                | "nsfs"
                | "mqueue"
                | "debugfs"
                | "tracefs"
                | "securityfs"
                | "pstore"
                | "efivarfs"
                | "bpf"
                | "autofs"
                | "ramfs"
                | "fusectl"
                | "configfs"
                | "hugetlbfs"
                | "rpc_pipefs"
                | "binfmt_misc"
                | "aufs"
                | "devpts"
                | "devfs"
                | "fdescfs"
                | "linprocfs"
                | "linsysfs"
                | "nullfs"
        ) {
            continue;
        }
        let name = d.name().to_string_lossy();
        let key = if name.is_empty() {
            d.mount_point().to_string_lossy().into_owned()
        } else {
            name.into_owned()
        };
        if !seen_devices.insert(key) {
            continue;
        }
        disk_total += d.total_space() as f64 / 1024.0 / 1024.0;
        disk_available += d.available_space() as f64 / 1024.0 / 1024.0;
    }
    let disk_used = (disk_total - disk_available).max(0.0);
    let disk_used_percent = if disk_total > 0.0 {
        (disk_used / disk_total) * 100.0
    } else {
        0.0
    };

    let mut net_rx_bytes = 0u64;
    let mut net_tx_bytes = 0u64;
    for (_name, data) in &state.networks {
        net_rx_bytes = net_rx_bytes.saturating_add(data.total_received());
        net_tx_bytes = net_tx_bytes.saturating_add(data.total_transmitted());
    }
    let now = Instant::now();
    let (net_rx_bytes_per_sec, net_tx_bytes_per_sec) = if let Some((prx, ptx, at)) = state.last_net
    {
        let dt = now.duration_since(at).as_secs_f64().max(0.001);
        (
            net_rx_bytes.saturating_sub(prx) as f64 / dt,
            net_tx_bytes.saturating_sub(ptx) as f64 / dt,
        )
    } else {
        (0.0, 0.0)
    };
    state.last_net = Some((net_rx_bytes, net_tx_bytes, now));

    // disk IO from processes aggregate as best-effort
    let mut read_bytes = 0u64;
    let mut write_bytes = 0u64;
    for p in state.sys.processes().values() {
        let usage = p.disk_usage();
        read_bytes = read_bytes.saturating_add(usage.total_read_bytes);
        write_bytes = write_bytes.saturating_add(usage.total_written_bytes);
    }
    let (disk_read_bytes_per_sec, disk_write_bytes_per_sec) =
        if let Some((pr, pw, at)) = state.last_disk_io {
            let dt = now.duration_since(at).as_secs_f64().max(0.001);
            (
                Some(read_bytes.saturating_sub(pr) as f64 / dt),
                Some(write_bytes.saturating_sub(pw) as f64 / dt),
            )
        } else {
            (Some(0.0), Some(0.0))
        };
    state.last_disk_io = Some((read_bytes, write_bytes, now));

    // temperature: average of available components as best-effort
    let temps: Vec<f32> = state
        .components
        .iter()
        .filter_map(|c| c.temperature())
        .collect();
    let temp_celsius = if temps.is_empty() {
        None
    } else {
        Some((temps.iter().copied().sum::<f32>() / temps.len() as f32) as f64)
    };

    // top processes by cpu/mem (best-effort, compact json string)
    let mut procs: Vec<_> = state
        .sys
        .processes()
        .values()
        .map(|p| {
            (
                p.name().to_string_lossy().to_string(),
                f64::from(p.cpu_usage()),
                p.memory() as f64 / 1024.0 / 1024.0,
            )
        })
        .collect();
    procs.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
    });
    procs.truncate(5);
    let top = serde_json::to_string(
        &procs
            .iter()
            .map(|(n, c, m)| serde_json::json!({"name":n,"cpu":c,"mem_mb":m}))
            .collect::<Vec<_>>(),
    )
    .ok();

    SystemMetrics {
        cpu_percent,
        mem_used_mb: mem_used,
        mem_total_mb: mem_total,
        mem_used_percent,
        disk_used_mb: disk_used,
        disk_total_mb: disk_total,
        disk_available_mb: disk_available,
        disk_used_percent,
        net_rx_bytes,
        net_tx_bytes,
        net_rx_bytes_per_sec,
        net_tx_bytes_per_sec,
        load1: Some(load.one),
        load5: Some(load.five),
        load15: Some(load.fifteen),
        swap_used_mb: Some(swap_used),
        swap_total_mb: Some(swap_total),
        swap_used_percent,
        uptime_seconds,
        disk_read_bytes: Some(read_bytes),
        disk_write_bytes: Some(write_bytes),
        disk_read_bytes_per_sec,
        disk_write_bytes_per_sec,
        temp_celsius,
        top_processes: top,
    }
}

#[cfg(target_os = "openbsd")]
fn collect_system_openbsd(state: &mut CollectorState) -> SystemMetrics {
    use std::process::Command;

    fn sysctl_n(name: &str) -> Option<String> {
        let out = Command::new("sysctl").arg("-n").arg(name).output().ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn run(cmd: &str, args: &[&str]) -> Option<String> {
        let out = Command::new(cmd).args(args).output().ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).to_string())
    }

    let now = Instant::now();

    // Load average
    let loadavg = sysctl_n("vm.loadavg").unwrap_or_default();
    // format: "{ 1.23 0.45 0.67 }" or "1.23 0.45 0.67"
    let load_parts: Vec<f64> = loadavg
        .replace(['{', '}'], " ")
        .split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    let load1 = load_parts.first().copied();
    let load5 = load_parts.get(1).copied();
    let load15 = load_parts.get(2).copied();

    // Uptime from boottime
    let uptime_seconds = sysctl_n("kern.boottime").and_then(|s| {
        // "sec usec" or "sec" depending; extract first int
        s.split(|c: char| !c.is_ascii_digit() && c != '-')
            .find(|p| !p.is_empty())
            .and_then(|p| p.parse::<i64>().ok())
            .map(|boot| (chrono::Utc::now().timestamp() - boot).max(0))
    });

    // CPU from kern.cp_time: user nice sys spin intr idle
    let cpu_percent = if let Some(ct) = sysctl_n("kern.cp_time") {
        let vals: Vec<u64> = ct
            .split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .filter_map(|p| p.parse::<u64>().ok())
            .collect();
        if vals.len() >= 5 {
            let user = vals[0];
            let nice = vals[1];
            let sysc = vals[2];
            let spin = *vals.get(3).unwrap_or(&0);
            let intr = *vals.get(4).unwrap_or(&0);
            let idle = *vals.get(5).unwrap_or(&0);
            let total = user + nice + sysc + spin + intr + idle;
            let busy = total.saturating_sub(idle);
            let pct = if let Some((pb, pt, at)) = state.last_cpu {
                let dt_busy = busy.saturating_sub(pb);
                let dt_total = total.saturating_sub(pt);
                let _ = at; // time not needed, ticks are cumulative
                if dt_total > 0 {
                    (dt_busy as f64 / dt_total as f64) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            };
            state.last_cpu = Some((busy, total, now));
            pct
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Memory (best-effort via sysctl)
    let physmem = sysctl_n("hw.physmem64")
        .or_else(|| sysctl_n("hw.physmem"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let pagesize = sysctl_n("hw.pagesize")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(4096);
    let uvm = sysctl_n("vm.uvmexp").unwrap_or_default();
    // uvmexp: key=value pairs (pagesize npages free active inactive wired ...)
    fn uvm_val(s: &str, key: &str) -> Option<u64> {
        for part in s.split_whitespace() {
            if let Some(v) = part.strip_prefix(&format!("{key}=")) {
                return v.parse::<u64>().ok();
            }
            // sometimes "key=val,"
        }
        // fallback: "key val"
        let mut it = s.split_whitespace();
        while let Some(k) = it.next() {
            if k.trim_end_matches('=') == key || k == key {
                if let Some(v) = it.next() {
                    return v.trim_matches(|c: char| c == '=' || c == ',').parse().ok();
                }
            }
        }
        None
    }
    let mem_total_mb = physmem as f64 / 1024.0 / 1024.0;
    let (mem_used_mb, mem_used_percent) = if !uvm.is_empty() {
        let active = uvm_val(&uvm, "active").unwrap_or(0);
        let wired = uvm_val(&uvm, "wired").unwrap_or(0);
        let used_pages = active.saturating_add(wired);
        let used = (used_pages * pagesize) as f64 / 1024.0 / 1024.0;
        let pct = if mem_total_mb > 0.0 {
            (used / mem_total_mb) * 100.0
        } else {
            0.0
        };
        (used.min(mem_total_mb), pct.min(100.0))
    } else {
        (0.0, 0.0)
    };

    // Swap via swapctl -sk (1K-blocks)
    let (swap_total_mb, swap_used_mb, swap_used_percent) =
        if let Some(out) = run("swapctl", &["-sk"]) {
            // "total: 1048576 1K-blocks allocated, 12345 used, 1036231 available"
            let nums: Vec<f64> = out
                .split(|c: char| !(c.is_ascii_digit() || c == '.'))
                .filter(|p| !p.is_empty())
                .filter_map(|p| p.parse::<f64>().ok())
                .collect();
            if nums.len() >= 2 {
                let total_kb = nums[0];
                let used_kb = nums[1];
                let total = total_kb / 1024.0;
                let used = used_kb / 1024.0;
                let pct = if total > 0.0 {
                    Some((used / total) * 100.0)
                } else {
                    None
                };
                (Some(total), Some(used), pct)
            } else {
                (Some(0.0), Some(0.0), None)
            }
        } else {
            (None, None, None)
        };

    // Disk usage via df -k (local filesystems)
    let (disk_total_mb, disk_available_mb, disk_used_mb, disk_used_percent) =
        if let Some(out) = run("df", &["-k"]) {
            let mut total: f64 = 0.0;
            let mut avail: f64 = 0.0;
            let mut seen: HashSet<String> = HashSet::new();
            for line in out.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 6 {
                    continue;
                }
                let fs = parts[0].to_string();
                let mnt = parts.last().copied().unwrap_or("");
                // skip pseudo
                if fs.starts_with("procfs")
                    || fs.starts_with("mfs")
                    || mnt.starts_with("/dev")
                    || mnt == "/kern"
                {
                    continue;
                }
                if !seen.insert(fs) {
                    continue;
                }
                // 1K-blocks total, used, avail, cap, mounted
                let t = parts
                    .get(1)
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let a = parts
                    .get(3)
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                total += t;
                avail += a;
            }
            let used = (total - avail).max(0.0);
            let total_mb = total / 1024.0;
            let avail_mb = avail / 1024.0;
            let used_mb = used / 1024.0;
            let pct = if total_mb > 0.0 {
                (used_mb / total_mb) * 100.0
            } else {
                0.0
            };
            (total_mb, avail_mb, used_mb, pct)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

    // Network totals via netstat -ibn
    let (net_rx_bytes, net_tx_bytes) = if let Some(out) = run("netstat", &["-ibn"]) {
        let mut rx: u64 = 0;
        let mut tx: u64 = 0;
        let mut seen_if: HashSet<String> = HashSet::new();
        for line in out.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }
            let iface = parts[0].trim_end_matches('*').to_string();
            // header per-if sometimes appears; take Link/# lines
            if parts.get(2).copied() != Some("<Link>")
                && !parts.get(2).map(|s| s.contains(':')).unwrap_or(false)
            {
                // heuristic: some formats vary; try to detect numeric Ibytes/Obytes near end
            }
            // OpenBSD netstat -ibn: Name Mtu Network Address Ipkts Ierrs Ibytes Opkts Oerrs Obytes Colls
            if parts.len() >= 10 && parts.get(2).copied() == Some("<Link>") {
                if !seen_if.insert(iface.clone()) {
                    continue;
                }
                let ibytes = parts
                    .get(6)
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                let obytes = parts
                    .get(9)
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                rx = rx.saturating_add(ibytes);
                tx = tx.saturating_add(obytes);
            }
        }
        (rx, tx)
    } else {
        (0, 0)
    };
    let (net_rx_bytes_per_sec, net_tx_bytes_per_sec) = if let Some((prx, ptx, at)) = state.last_net
    {
        let dt = now.duration_since(at).as_secs_f64().max(0.001);
        (
            net_rx_bytes.saturating_sub(prx) as f64 / dt,
            net_tx_bytes.saturating_sub(ptx) as f64 / dt,
        )
    } else {
        (0.0, 0.0)
    };
    state.last_net = Some((net_rx_bytes, net_tx_bytes, now));

    // Top processes via ps (best-effort)
    let top_processes = run("ps", &["-axo", "comm,%cpu,rss", "-r"]).and_then(|out| {
        let mut procs = Vec::new();
        for line in out.lines().skip(1).take(5) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[0].to_string();
                let cpu = parts[1].parse::<f64>().unwrap_or(0.0);
                let mem_kb = parts[2].parse::<f64>().unwrap_or(0.0);
                procs.push(serde_json::json!({"name":name,"cpu":cpu,"mem_mb":mem_kb / 1024.0}));
            }
        }
        if procs.is_empty() {
            None
        } else {
            serde_json::to_string(&procs).ok()
        }
    });

    // Temperature best-effort via sysctl hw.sensors
    let temp_celsius = run("sysctl", &["hw.sensors"]).and_then(|out| {
        let mut temps: Vec<f64> = Vec::new();
        for line in out.lines() {
            // ...temp0=45.00 degC
            if line.contains("degC") || line.contains("temp") {
                for tok in line.split_whitespace() {
                    if let Some(v) = tok.strip_suffix("degC") {
                        if let Ok(t) = v
                            .trim_end_matches(|c: char| c == '=' || c == ',')
                            .parse::<f64>()
                        {
                            if t > -50.0 && t < 150.0 {
                                temps.push(t);
                            }
                        }
                    }
                }
                // also "temp0=45.00"
                if let Some(idx) = line.find("degC") {
                    let before = &line[..idx];
                    if let Some(eq) = before.rfind('=') {
                        if let Ok(t) = before[eq + 1..].trim().parse::<f64>() {
                            if t > -50.0 && t < 150.0 {
                                temps.push(t);
                            }
                        }
                    }
                }
            }
        }
        if temps.is_empty() {
            None
        } else {
            Some(temps.iter().sum::<f64>() / temps.len() as f64)
        }
    });

    SystemMetrics {
        cpu_percent,
        mem_used_mb,
        mem_total_mb,
        mem_used_percent,
        disk_used_mb,
        disk_total_mb,
        disk_available_mb,
        disk_used_percent,
        net_rx_bytes,
        net_tx_bytes,
        net_rx_bytes_per_sec,
        net_tx_bytes_per_sec,
        load1,
        load5,
        load15,
        swap_used_mb,
        swap_total_mb,
        swap_used_percent,
        uptime_seconds,
        disk_read_bytes: None,
        disk_write_bytes: None,
        disk_read_bytes_per_sec: None,
        disk_write_bytes_per_sec: None,
        temp_celsius,
        top_processes,
    }
}

async fn collect_targets(config: &Config) -> Vec<TargetInfo> {
    let client = http_client(config.request_timeout_secs);
    let mut set = tokio::task::JoinSet::new();
    let timeout = Duration::from_secs(config.request_timeout_secs.max(1));
    for target in config.targets.clone() {
        let client = client.clone();
        set.spawn(async move { probe_target(&client, &target, timeout).await });
    }
    let mut results = Vec::with_capacity(config.targets.len());
    while let Some(res) = set.join_next().await {
        match res {
            Ok(info) => results.push(info),
            Err(e) => tracing::warn!(error = %e, "target probe task failed"),
        }
    }
    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

async fn probe_target(client: &reqwest::Client, target: &Target, timeout: Duration) -> TargetInfo {
    let kind = target.kind.clone().unwrap_or_else(|| "generic".to_string());
    match target.check_type.as_str() {
        "tcp" => probe_tcp(target, timeout).await,
        "ping" => probe_ping(target, timeout).await,
        "tls" => probe_tls(target, timeout).await,
        _ => probe_http(client, target, &kind).await,
    }
}

async fn probe_http(client: &reqwest::Client, target: &Target, kind: &str) -> TargetInfo {
    let start = Instant::now();
    match client.get(&target.url).send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            let status_code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let (status, http_requests, detail) = interpret_response(kind, status_code, &body);
            // best-effort TLS cert days for https urls
            let cert_days_left = if target.url.starts_with("https://") {
                extract_host_port(&target.url).and_then(|(h, p)| tls_days_left(&h, p).ok())
            } else {
                None
            };
            TargetInfo {
                name: target.name.clone(),
                kind: kind.to_string(),
                url: target.url.clone(),
                status,
                latency_ms: Some(latency_ms),
                http_requests,
                detail,
                cert_days_left,
                check_type: "http".to_string(),
            }
        }
        Err(e) => TargetInfo {
            name: target.name.clone(),
            kind: kind.to_string(),
            url: target.url.clone(),
            status: "error".to_string(),
            latency_ms: None,
            http_requests: None,
            detail: Some(shorten(&e.to_string(), 180)),
            cert_days_left: None,
            check_type: "http".to_string(),
        },
    }
}

async fn probe_tcp(target: &Target, timeout: Duration) -> TargetInfo {
    let host = target.host.clone().unwrap_or_default();
    let port = target.port.unwrap_or(0);
    let addr = format!("{host}:{port}");
    let start = Instant::now();
    let res = tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&addr)).await;
    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    match res {
        Ok(Ok(_)) => TargetInfo {
            name: target.name.clone(),
            kind: "tcp".into(),
            url: addr,
            status: "up".into(),
            latency_ms: Some(latency_ms),
            http_requests: None,
            detail: None,
            cert_days_left: None,
            check_type: "tcp".into(),
        },
        Ok(Err(e)) => TargetInfo {
            name: target.name.clone(),
            kind: "tcp".into(),
            url: addr,
            status: "down".into(),
            latency_ms: Some(latency_ms),
            http_requests: None,
            detail: Some(shorten(&e.to_string(), 180)),
            cert_days_left: None,
            check_type: "tcp".into(),
        },
        Err(_) => TargetInfo {
            name: target.name.clone(),
            kind: "tcp".into(),
            url: format!("{host}:{port}"),
            status: "down".into(),
            latency_ms: Some(latency_ms),
            http_requests: None,
            detail: Some("timeout".into()),
            cert_days_left: None,
            check_type: "tcp".into(),
        },
    }
}

async fn probe_ping(target: &Target, timeout: Duration) -> TargetInfo {
    let host = target
        .host
        .clone()
        .or_else(|| extract_host_port(&target.url).map(|(h, _)| h))
        .unwrap_or_else(|| target.url.clone());
    let start = Instant::now();
    // best-effort: try system ping (may not be available / may need perms). Fallback to TCP 80? no, report unsupported if fails.
    let out = tokio::time::timeout(
        timeout + Duration::from_millis(200),
        tokio::process::Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg("-W")
            .arg(format!("{}", timeout.as_secs().max(1)))
            .arg(&host)
            .output(),
    )
    .await;
    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    match out {
        Ok(Ok(o)) if o.status.success() => TargetInfo {
            name: target.name.clone(),
            kind: "ping".into(),
            url: host,
            status: "up".into(),
            latency_ms: Some(latency_ms),
            http_requests: None,
            detail: None,
            cert_days_left: None,
            check_type: "ping".into(),
        },
        Ok(Ok(o)) => TargetInfo {
            name: target.name.clone(),
            kind: "ping".into(),
            url: host,
            status: "down".into(),
            latency_ms: Some(latency_ms),
            http_requests: None,
            detail: Some(shorten(&String::from_utf8_lossy(&o.stderr), 180)),
            cert_days_left: None,
            check_type: "ping".into(),
        },
        Ok(Err(e)) => TargetInfo {
            name: target.name.clone(),
            kind: "ping".into(),
            url: host,
            status: "error".into(),
            latency_ms: None,
            http_requests: None,
            detail: Some(shorten(&format!("ping unavailable: {e}"), 180)),
            cert_days_left: None,
            check_type: "ping".into(),
        },
        Err(_) => TargetInfo {
            name: target.name.clone(),
            kind: "ping".into(),
            url: host,
            status: "down".into(),
            latency_ms: Some(latency_ms),
            http_requests: None,
            detail: Some("timeout".into()),
            cert_days_left: None,
            check_type: "ping".into(),
        },
    }
}

async fn probe_tls(target: &Target, timeout: Duration) -> TargetInfo {
    let host = target.host.clone().unwrap_or_default();
    let port = target.port.unwrap_or(443);
    let addr = format!("{host}:{port}");
    let start = Instant::now();
    // handshake/connect as liveness + cert days via blocking helper in spawn_blocking? keep simple: try tcp connect then parse cert days best-effort
    let res = tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&addr)).await;
    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let days = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking({
            let h = host.clone();
            move || tls_days_left(&h, port).ok()
        }),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
    .flatten();
    match res {
        Ok(Ok(_)) => TargetInfo {
            name: target.name.clone(),
            kind: "tls".into(),
            url: addr,
            status: "up".into(),
            latency_ms: Some(latency_ms),
            http_requests: None,
            detail: days.map(|d| format!("cert_days_left={d}")),
            cert_days_left: days,
            check_type: "tls".into(),
        },
        Ok(Err(e)) => TargetInfo {
            name: target.name.clone(),
            kind: "tls".into(),
            url: addr,
            status: "down".into(),
            latency_ms: Some(latency_ms),
            http_requests: None,
            detail: Some(shorten(&e.to_string(), 180)),
            cert_days_left: days,
            check_type: "tls".into(),
        },
        Err(_) => TargetInfo {
            name: target.name.clone(),
            kind: "tls".into(),
            url: format!("{host}:{port}"),
            status: "down".into(),
            latency_ms: Some(latency_ms),
            http_requests: None,
            detail: Some("timeout".into()),
            cert_days_left: days,
            check_type: "tls".into(),
        },
    }
}

fn tls_days_left(host: &str, port: u16) -> anyhow::Result<i64> {
    // best-effort using openssl s_client if available (avoids adding heavy deps). fallback: None
    let out = std::process::Command::new("openssl")
        .args([
            "s_client",
            "-servername",
            host,
            "-connect",
            &format!("{host}:{port}"),
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()?;
    if !out.status.success() {
        anyhow::bail!("openssl failed");
    }
    // pipe to openssl x509 -noout -enddate
    let mut child = std::process::Command::new("openssl")
        .args(["x509", "-noout", "-enddate"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(&out.stdout);
    }
    let out2 = child.wait_with_output()?;
    let s = String::from_utf8_lossy(&out2.stdout);
    // notAfter=Jun  7 12:00:00 2026 GMT
    if let Some(v) = s.trim().strip_prefix("notAfter=") {
        // parse with chrono
        if let Ok(dt) = chrono::DateTime::parse_from_str(v.trim(), "%b %e %H:%M:%S %Y %Z")
            .or_else(|_| chrono::DateTime::parse_from_str(v.trim(), "%b %d %H:%M:%S %Y %Z"))
        {
            let days = (dt.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_days();
            return Ok(days);
        }
    }
    anyhow::bail!("unparsed")
}

fn extract_host_port(url: &str) -> Option<(String, u16)> {
    let u = reqwest::Url::parse(url).ok()?;
    let host = u.host_str()?.to_string();
    let port = u.port_or_known_default()?;
    Some((host, port))
}

// --- HTTP interpretation helpers (kept compatible) ---
fn interpret_response(
    kind: &str,
    status_code: reqwest::StatusCode,
    body: &str,
) -> (String, Option<i64>, Option<String>) {
    let ok_http = status_code.is_success();
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
    let mut http_requests = extract_requests_count(parsed.as_ref());
    let mut detail = None;
    let status = match kind.to_ascii_lowercase().as_str() {
        "spring-boot" | "spring" | "actuator" => {
            interpret_spring(ok_http, parsed.as_ref(), &mut detail)
        }
        "quarkus" | "micronaut" | "micrometer" => {
            interpret_micrometer_like(ok_http, parsed.as_ref(), &mut detail)
        }
        _ => interpret_generic(ok_http, parsed.as_ref(), &mut detail),
    };
    if http_requests.is_none() {
        http_requests = extract_text_metrics_requests(body);
    }
    if detail.is_none() && !ok_http {
        detail = Some(format!("http {}", status_code.as_u16()));
    }
    (status, http_requests, detail)
}
fn interpret_spring(
    ok_http: bool,
    parsed: Option<&serde_json::Value>,
    detail: &mut Option<String>,
) -> String {
    if let Some(v) = parsed {
        if let Some(s) = v.get("status").and_then(|x| x.as_str()) {
            *detail = Some(format!("status={s}"));
            return match s.to_ascii_uppercase().as_str() {
                "UP" => "up".into(),
                "DOWN" | "OUT_OF_SERVICE" => "down".into(),
                _ if ok_http => "up".into(),
                _ => "degraded".into(),
            };
        }
        if v.get("name").is_some() && v.get("measurements").is_some() {
            *detail = v
                .get("name")
                .and_then(|x| x.as_str())
                .map(|n| format!("metric={n}"));
            return if ok_http { "up".into() } else { "down".into() };
        }
    }
    interpret_generic(ok_http, parsed, detail)
}
fn interpret_micrometer_like(
    ok_http: bool,
    parsed: Option<&serde_json::Value>,
    detail: &mut Option<String>,
) -> String {
    if let Some(v) = parsed {
        if let Some(status) = v.get("status").and_then(|x| x.as_str()) {
            *detail = Some(format!("status={status}"));
            return if status.eq_ignore_ascii_case("up") || status.eq_ignore_ascii_case("ok") {
                "up".into()
            } else if ok_http {
                "degraded".into()
            } else {
                "down".into()
            };
        }
        if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
            *detail = Some(format!("metric={name}"));
        }
    }
    if ok_http { "up".into() } else { "down".into() }
}
fn interpret_generic(
    ok_http: bool,
    parsed: Option<&serde_json::Value>,
    detail: &mut Option<String>,
) -> String {
    if let Some(v) = parsed
        && let Some(s) = v.get("status").and_then(|x| x.as_str())
    {
        *detail = Some(format!("status={s}"));
        let up = matches!(
            s.to_ascii_lowercase().as_str(),
            "up" | "ok" | "healthy" | "pass" | "available"
        );
        let down = matches!(
            s.to_ascii_lowercase().as_str(),
            "down" | "error" | "fail" | "unhealthy"
        );
        return if up {
            "up".into()
        } else if down {
            "down".into()
        } else if ok_http {
            "degraded".into()
        } else {
            "down".into()
        };
    }
    if ok_http { "up".into() } else { "down".into() }
}
fn extract_requests_count(v: Option<&serde_json::Value>) -> Option<i64> {
    let v = v?;
    if let Some(measurements) = v.get("measurements").and_then(|m| m.as_array()) {
        for m in measurements {
            let stat = m.get("statistic").and_then(|s| s.as_str()).unwrap_or("");
            if (stat.eq_ignore_ascii_case("count") || stat.eq_ignore_ascii_case("value"))
                && let Some(n) = m.get("value").and_then(as_i64)
            {
                return Some(n);
            }
        }
        if let Some(n) = measurements
            .first()
            .and_then(|m| m.get("value"))
            .and_then(as_i64)
        {
            return Some(n);
        }
    }
    for key in [
        "requests",
        "request_count",
        "requestCount",
        "http_requests",
        "httpRequests",
        "count",
        "total",
    ] {
        if let Some(n) = v.get(key).and_then(as_i64) {
            return Some(n);
        }
    }
    v.get("metrics")
        .and_then(|m| m.get("requests"))
        .and_then(as_i64)
}
fn extract_text_metrics_requests(body: &str) -> Option<i64> {
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if (line.contains("http_server_requests_seconds_count")
            || line.contains("http_requests_total")
            || line.contains("requests_count"))
            && let Some(val) = line.split_whitespace().last()
            && let Ok(f) = val.parse::<f64>()
            && f.is_finite()
            && f >= 0.0
        {
            return Some(f as i64);
        }
    }
    None
}
fn as_i64(v: &serde_json::Value) -> Option<i64> {
    match v {
        serde_json::Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().map(|u| u.min(i64::MAX as u64) as i64))
            .or_else(|| n.as_f64().map(|f| f as i64)),
        serde_json::Value::String(s) => s
            .parse::<i64>()
            .ok()
            .or_else(|| s.parse::<f64>().ok().map(|f| f as i64)),
        _ => None,
    }
}
fn shorten(s: &str, max: usize) -> String {
    let s = s.trim().replace('\n', " ");
    if s.chars().count() <= max {
        s
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_actuator_count() {
        let v: serde_json::Value = serde_json::json!({"name":"http.server.requests","measurements":[{"statistic":"COUNT","value":42.0}]});
        assert_eq!(extract_requests_count(Some(&v)), Some(42));
    }

    #[test]
    fn extracts_requests_from_common_keys() {
        let v: serde_json::Value = serde_json::json!({"http_requests": 7});
        assert_eq!(extract_requests_count(Some(&v)), Some(7));
        let v2: serde_json::Value = serde_json::json!({"metrics":{"requests": 9}});
        assert_eq!(extract_requests_count(Some(&v2)), Some(9));
    }

    #[test]
    fn extracts_text_metrics_count() {
        let body = "# HELP x\nhttp_server_requests_seconds_count{uri=\"/\"} 123\n";
        assert_eq!(extract_text_metrics_requests(body), Some(123));
    }

    #[test]
    fn as_i64_parses_number_and_string() {
        assert_eq!(as_i64(&serde_json::json!(5)), Some(5));
        assert_eq!(as_i64(&serde_json::json!("6")), Some(6));
        assert_eq!(as_i64(&serde_json::json!(7.9)), Some(7));
        assert_eq!(as_i64(&serde_json::json!("nope")), None);
    }

    #[test]
    fn interprets_generic_up_down_and_degraded() {
        let (st, _, _) =
            interpret_response("generic", reqwest::StatusCode::OK, "{\"status\":\"ok\"}");
        assert_eq!(st, "up");
        let (st, _, _) =
            interpret_response("generic", reqwest::StatusCode::OK, "{\"status\":\"error\"}");
        assert_eq!(st, "down");
        // unknown status string: degraded when HTTP is ok, down when HTTP is not ok
        let (st, _, _) =
            interpret_response("generic", reqwest::StatusCode::OK, "{\"status\":\"weird\"}");
        assert_eq!(st, "degraded");
        let (st, _, _) = interpret_response(
            "generic",
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "{\"status\":\"weird\"}",
        );
        assert_eq!(st, "down");
        let (st, _, _) = interpret_response(
            "generic",
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "oops",
        );
        assert_eq!(st, "down");
    }

    #[test]
    fn interprets_spring_status() {
        let (st, _, d) = interpret_response(
            "spring-boot",
            reqwest::StatusCode::OK,
            "{\"status\":\"UP\"}",
        );
        assert_eq!(st, "up");
        assert!(d.unwrap_or_default().contains("UP"));
        let (st, _, _) = interpret_response(
            "actuator",
            reqwest::StatusCode::SERVICE_UNAVAILABLE,
            "{\"status\":\"DOWN\"}",
        );
        assert_eq!(st, "down");
    }

    #[test]
    fn interprets_micrometer_like() {
        let (st, _, _) =
            interpret_response("quarkus", reqwest::StatusCode::OK, "{\"status\":\"UP\"}");
        assert_eq!(st, "up");
        let (st, _, d) = interpret_response(
            "micrometer",
            reqwest::StatusCode::OK,
            "{\"name\":\"http.server.requests\",\"measurements\":[]}",
        );
        assert_eq!(st, "up");
        assert!(d.unwrap_or_default().contains("metric="));
    }

    #[test]
    fn shorten_and_extract_host_port() {
        assert_eq!(shorten("  hello\nworld  ", 5), "hello…");
        assert_eq!(shorten("ok", 5), "ok");
        assert_eq!(
            extract_host_port("https://example.com:8443/path"),
            Some(("example.com".to_string(), 8443))
        );
        assert_eq!(
            extract_host_port("http://example.com/x"),
            Some(("example.com".to_string(), 80))
        );
    }

    #[tokio::test]
    async fn probe_tcp_reports_up_and_down() {
        use std::net::TcpListener as StdTcpListener;

        let listener = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let up_target = Target {
            name: "tcp-up".into(),
            url: String::new(),
            kind: None,
            check_type: "tcp".into(),
            host: Some("127.0.0.1".into()),
            port: Some(port),
            latency_warn_ms: None,
            cert_expiry_warn_days: None,
        };
        let up = probe_tcp(&up_target, Duration::from_secs(1)).await;
        assert_eq!(up.status, "up");
        assert_eq!(up.check_type, "tcp");
        assert!(up.latency_ms.is_some());

        drop(listener);
        let down_target = Target {
            name: "tcp-down".into(),
            url: String::new(),
            kind: None,
            check_type: "tcp".into(),
            host: Some("127.0.0.1".into()),
            port: Some(port),
            latency_warn_ms: None,
            cert_expiry_warn_days: None,
        };
        // ensure port is no longer accepting connections (avoids rare reuse races)
        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        loop {
            match tokio::time::timeout(
                Duration::from_millis(50),
                tokio::net::TcpStream::connect(("127.0.0.1", port)),
            )
            .await
            {
                Ok(Ok(_)) if tokio::time::Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                _ => break,
            }
        }
        let down = probe_tcp(&down_target, Duration::from_secs(1)).await;
        assert_eq!(down.status, "down");
        assert_eq!(down.check_type, "tcp");
    }

    #[tokio::test]
    async fn probe_tls_reports_connectivity_best_effort() {
        use std::net::TcpListener as StdTcpListener;

        let listener = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let up_target = Target {
            name: "tls-up".into(),
            url: String::new(),
            kind: None,
            check_type: "tls".into(),
            host: Some("127.0.0.1".into()),
            port: Some(port),
            latency_warn_ms: None,
            cert_expiry_warn_days: None,
        };
        let up = probe_tls(&up_target, Duration::from_secs(1)).await;
        assert_eq!(up.status, "up");
        assert_eq!(up.check_type, "tls");
        assert!(up.latency_ms.is_some());
        // plain TCP listener: cert parsing is best-effort/unavailable
        assert!(up.cert_days_left.is_none());

        drop(listener);
        let down_target = Target {
            name: "tls-down".into(),
            url: String::new(),
            kind: None,
            check_type: "tls".into(),
            host: Some("127.0.0.1".into()),
            port: Some(port),
            latency_warn_ms: None,
            cert_expiry_warn_days: None,
        };
        let down = probe_tls(&down_target, Duration::from_secs(1)).await;
        assert_eq!(down.status, "down");
        assert_eq!(down.check_type, "tls");
    }

    #[tokio::test]
    async fn probe_ping_is_best_effort() {
        let target = Target {
            name: "ping-local".into(),
            url: String::new(),
            kind: None,
            check_type: "ping".into(),
            host: Some("127.0.0.1".into()),
            port: None,
            latency_warn_ms: None,
            cert_expiry_warn_days: None,
        };
        let res = probe_ping(&target, Duration::from_secs(1)).await;
        assert_eq!(res.check_type, "ping");
        assert!(matches!(res.status.as_str(), "up" | "down" | "error"));
    }
}
