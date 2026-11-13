use crate::{RuntimeStatus, config::Config, db::Database};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, Tabs, Wrap},
};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

pub async fn run(
    db: Arc<Database>,
    cfg: Arc<RwLock<Config>>,
    runtime: Arc<RwLock<RuntimeStatus>>,
    shutdown: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, db, cfg, runtime, shutdown.clone()).await;

    shutdown.store(true, Ordering::Relaxed);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    res
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    System,
    Targets,
    Alerts,
    Events,
}

struct AppData {
    system: Option<crate::db::SystemMetric>,
    history: Vec<crate::db::SystemMetric>,
    targets: Vec<crate::db::TargetMetric>,
    alerts: Vec<crate::db::AlertRow>,
    events: Vec<crate::db::EventRow>,
    runtime: RuntimeStatus,
    last_error: Option<String>,
}

struct UiState {
    view: View,
    target_idx: usize,
    range_hours: i64,
}

async fn load_data(
    db: &Database,
    cfg: &Config,
    runtime: &Arc<RwLock<RuntimeStatus>>,
    state: &UiState,
) -> AppData {
    let mut last_error = None;
    let system = match db.latest_system().await {
        Ok(v) => v,
        Err(e) => {
            last_error = Some(format!("db: {e}"));
            None
        }
    };
    let history = match db.system_history(state.range_hours).await {
        Ok(v) => v,
        Err(e) => {
            last_error = Some(format!("db: {e}"));
            Vec::new()
        }
    };
    let mut targets = Vec::with_capacity(cfg.targets.len());
    for t in &cfg.targets {
        match db.latest_target(&t.name).await {
            Ok(Some(m)) => targets.push(m),
            Ok(None) => targets.push(crate::db::TargetMetric {
                id: 0,
                timestamp: "-".to_string(),
                target: t.name.clone(),
                kind: t.kind.clone().unwrap_or_else(|| "generic".to_string()),
                url: t.url.clone(),
                status: "unknown".to_string(),
                latency_ms: None,
                http_requests: None,
                detail: Some("no data yet".to_string()),
                cert_days_left: None,
                check_type: Some(t.check_type.clone()),
            }),
            Err(e) => last_error = Some(format!("db: {e}")),
        }
    }
    let alerts = db.list_alerts(None, 100).await.unwrap_or_default();
    let events = db.list_events(100).await.unwrap_or_default();
    let runtime = runtime.read().await.clone();
    AppData {
        system,
        history,
        targets,
        alerts,
        events,
        runtime,
        last_error,
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    db: Arc<Database>,
    cfg: Arc<RwLock<Config>>,
    runtime: Arc<RwLock<RuntimeStatus>>,
    shutdown: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut cfg_snap = cfg.read().await.clone();
    let mut ui = UiState {
        view: View::System,
        target_idx: 0,
        range_hours: 24,
    };
    let mut data = load_data(&db, &cfg_snap, &runtime, &ui).await;
    let tick_rate = Duration::from_secs(1);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &cfg_snap, &data, &ui))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    shutdown.store(true, Ordering::Relaxed);
                    break;
                }
                KeyCode::Char('c')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    shutdown.store(true, Ordering::Relaxed);
                    break;
                }
                KeyCode::Char('r') => {
                    let cs = cfg.read().await.clone();
                    data = load_data(&db, &cs, &runtime, &ui).await;
                    cfg_snap = cs;
                }
                KeyCode::Tab | KeyCode::Right => {
                    ui.view = match ui.view {
                        View::System => View::Targets,
                        View::Targets => View::Alerts,
                        View::Alerts => View::Events,
                        View::Events => View::System,
                    };
                }
                KeyCode::BackTab | KeyCode::Left => {
                    ui.view = match ui.view {
                        View::System => View::Events,
                        View::Targets => View::System,
                        View::Alerts => View::Targets,
                        View::Events => View::Alerts,
                    };
                }
                KeyCode::Char('1') => ui.view = View::System,
                KeyCode::Char('2') => ui.view = View::Targets,
                KeyCode::Char('3') => ui.view = View::Alerts,
                KeyCode::Char('4') => ui.view = View::Events,
                KeyCode::Down | KeyCode::Char('j') => {
                    if ui.view == View::Targets && !data.targets.is_empty() {
                        ui.target_idx = (ui.target_idx + 1).min(data.targets.len() - 1);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if ui.view == View::Targets && ui.target_idx > 0 {
                        ui.target_idx -= 1;
                    }
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    ui.range_hours = match ui.range_hours {
                        1 => 6,
                        6 => 24,
                        24 => 72,
                        72 => 168,
                        _ => 168,
                    };
                    let cs = cfg.read().await.clone();
                    data = load_data(&db, &cs, &runtime, &ui).await;
                    cfg_snap = cs;
                }
                KeyCode::Char('-') => {
                    ui.range_hours = match ui.range_hours {
                        168 => 72,
                        72 => 24,
                        24 => 6,
                        6 => 1,
                        _ => 1,
                    };
                    let cs = cfg.read().await.clone();
                    data = load_data(&db, &cs, &runtime, &ui).await;
                    cfg_snap = cs;
                }
                _ => {}
            }
        }

        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        if last_tick.elapsed() >= tick_rate {
            let cs = cfg.read().await.clone();
            if ui.target_idx >= data.targets.len() && !data.targets.is_empty() {
                ui.target_idx = data.targets.len() - 1;
            }
            data = load_data(&db, &cs, &runtime, &ui).await;
            cfg_snap = cs;
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn draw(f: &mut Frame, cfg: &Config, data: &AppData, ui: &UiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(f, chunks[0], data);
    render_tabs(f, chunks[1], ui, data);
    match ui.view {
        View::System => render_system(f, chunks[2], data, ui),
        View::Targets => render_targets(f, chunks[2], data, ui),
        View::Alerts => render_alerts(f, chunks[2], data),
        View::Events => render_events(f, chunks[2], data),
    }
    render_footer(f, chunks[3], cfg, data, ui);
}

fn render_header(f: &mut Frame, area: Rect, data: &AppData) {
    let status = if data.runtime.last_collection_ok == Some(true) {
        Span::styled("● OK", Style::default().fg(Color::Green))
    } else if data.runtime.collections_total == 0 {
        Span::styled("● starting", Style::default().fg(Color::Yellow))
    } else {
        Span::styled("● degraded", Style::default().fg(Color::Red))
    };
    let firing = data.alerts.iter().filter(|a| a.status == "firing").count();
    let line = Line::from(vec![
        Span::styled(
            format!(" TinyTop v{} ", data.runtime.version),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("| "),
        status,
        Span::raw(" | col: "),
        Span::raw(data.runtime.collections_total.to_string()),
        Span::raw(" err: "),
        Span::raw(data.runtime.collection_errors_total.to_string()),
        Span::raw(" | alerts: "),
        Span::styled(
            firing.to_string(),
            if firing > 0 {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            },
        ),
    ]);
    let p = Paragraph::new(line)
        .block(Block::default().borders(Borders::ALL).title("TinyTop TUI"))
        .alignment(Alignment::Left);
    f.render_widget(p, area);
}

fn render_tabs(f: &mut Frame, area: Rect, ui: &UiState, data: &AppData) {
    let titles = vec![
        Line::from("1 System"),
        Line::from(format!("2 Targets({})", data.targets.len())),
        Line::from(format!("3 Alerts({})", data.alerts.len())),
        Line::from(format!("4 Events({})", data.events.len())),
    ];
    let idx = match ui.view {
        View::System => 0,
        View::Targets => 1,
        View::Alerts => 2,
        View::Events => 3,
    };
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .select(idx)
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

fn render_system(f: &mut Frame, area: Rect, data: &AppData, ui: &UiState) {
    let block = Block::default().borders(Borders::ALL).title("System");
    let Some(s) = &data.system else {
        let p = Paragraph::new("No system metrics yet…")
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(p, area);
        return;
    };
    let inner = block.inner(area);
    f.render_widget(block, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .margin(1)
        .split(inner);

    render_gauge_row(
        f,
        rows[0],
        "CPU",
        s.cpu_percent,
        format!("{:.1}%", s.cpu_percent),
    );
    render_gauge_row(
        f,
        rows[1],
        "Mem",
        s.mem_used_percent,
        format!(
            "{:.0}/{:.0} MB ({:.1}%)",
            s.mem_used_mb, s.mem_total_mb, s.mem_used_percent
        ),
    );
    render_gauge_row(
        f,
        rows[2],
        "Disk",
        s.disk_used_percent,
        format!(
            "{:.1}/{:.1} GB ({:.1}%)",
            s.disk_used_mb / 1024.0,
            s.disk_total_mb / 1024.0,
            s.disk_used_percent
        ),
    );
    let swap = if s.swap_total_mb.unwrap_or(0.0) > 0.0 {
        format!(
            "{:.0}/{:.0} MB ({:.1}%)",
            s.swap_used_mb.unwrap_or(0.0),
            s.swap_total_mb.unwrap_or(0.0),
            s.swap_used_percent.unwrap_or(0.0)
        )
    } else {
        "n/d".to_string()
    };
    let load = format!(
        "{:.2}/{:.2}/{:.2}",
        s.load1.unwrap_or(0.0),
        s.load5.unwrap_or(0.0),
        s.load15.unwrap_or(0.0)
    );
    let l1 = Line::from(vec![
        Span::styled("Load ", Style::default().fg(Color::Cyan)),
        Span::raw(load),
        Span::raw("   "),
        Span::styled("Swap ", Style::default().fg(Color::Cyan)),
        Span::raw(swap),
        Span::raw("   "),
        Span::styled("Temp ", Style::default().fg(Color::Cyan)),
        Span::raw(
            s.temp_celsius
                .map(|t| format!("{t:.1}°C"))
                .unwrap_or_else(|| "n/d".into()),
        ),
    ]);
    f.render_widget(Paragraph::new(l1), rows[3]);
    let l2 = Line::from(vec![
        Span::styled("Net  ", Style::default().fg(Color::Cyan)),
        Span::raw(format!(
            "↓ {}/s ↑ {}/s (rx {} tx {})",
            fmt_bytes(s.net_rx_bytes_per_sec),
            fmt_bytes(s.net_tx_bytes_per_sec),
            fmt_bytes_total(s.net_rx_bytes),
            fmt_bytes_total(s.net_tx_bytes)
        )),
        Span::raw("  "),
        Span::styled("DiskIO ", Style::default().fg(Color::Cyan)),
        Span::raw(format!(
            "R {}/s W {}/s",
            fmt_bytes(s.disk_read_bytes_per_sec.unwrap_or(0.0)),
            fmt_bytes(s.disk_write_bytes_per_sec.unwrap_or(0.0))
        )),
    ]);
    f.render_widget(Paragraph::new(l2), rows[4]);
    let l3 = Line::from(vec![
        Span::styled("Up   ", Style::default().fg(Color::Cyan)),
        Span::raw(
            s.uptime_seconds
                .map(fmt_uptime)
                .unwrap_or_else(|| "n/d".into()),
        ),
        Span::raw("   "),
        Span::styled("Last ", Style::default().fg(Color::DarkGray)),
        Span::raw(&s.timestamp),
    ]);
    f.render_widget(Paragraph::new(l3), rows[5]);
    let spark = format!(
        "CPU spark ({}h, {} pts): {}",
        ui.range_hours,
        data.history.len(),
        sparkline(&data.history, |m| m.cpu_percent)
    );
    let spark2 = format!(
        "MEM spark ({}h, {} pts): {}",
        ui.range_hours,
        data.history.len(),
        sparkline(&data.history, |m| m.mem_used_percent)
    );
    f.render_widget(
        Paragraph::new(vec![Line::from(spark), Line::from(spark2)]).wrap(Wrap { trim: true }),
        rows[6],
    );
    let procs = top_procs_str(s.top_processes.as_deref());
    f.render_widget(
        Paragraph::new(format!("Top procesos: {procs}")).wrap(Wrap { trim: true }),
        rows[7],
    );
}

fn render_gauge_row(f: &mut Frame, area: Rect, label: &str, pct: f64, detail: String) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(10),
            Constraint::Length(34),
        ])
        .split(area);
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{label:<5}"),
            Style::default().fg(Color::Cyan),
        )),
        cols[0],
    );
    let ratio = (pct / 100.0).clamp(0.0, 1.0);
    let color = if pct >= 90.0 {
        Color::Red
    } else if pct >= 75.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
        .ratio(ratio)
        .label(format!("{pct:.1}%"));
    f.render_widget(gauge, cols[1]);
    f.render_widget(Paragraph::new(detail).alignment(Alignment::Right), cols[2]);
}

fn render_targets(f: &mut Frame, area: Rect, data: &AppData, ui: &UiState) {
    let header = Row::new(vec![
        "Name", "Check", "Status", "Latency", "Req/TLS", "Detail", "Updated",
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));
    let rows = data.targets.iter().enumerate().map(|(i, t)| {
        let status_style = match t.status.as_str() {
            "up" | "ok" => Style::default().fg(Color::Green),
            "degraded" => Style::default().fg(Color::Yellow),
            "down" | "error" => Style::default().fg(Color::Red),
            _ => Style::default().fg(Color::Gray),
        };
        let latency = t
            .latency_ms
            .map(|v| format!("{v:.1} ms"))
            .unwrap_or_else(|| "-".into());
        let req = t
            .cert_days_left
            .map(|d| format!("cert {d}d"))
            .or_else(|| t.http_requests.map(|v| v.to_string()))
            .unwrap_or_else(|| "-".into());
        let detail = t.detail.clone().unwrap_or_else(|| "-".into());
        let mut row = Row::new(vec![
            Cell::from(t.target.clone()),
            Cell::from(t.check_type.clone().unwrap_or_else(|| t.kind.clone())),
            Cell::from(Span::styled(t.status.clone(), status_style)),
            Cell::from(latency),
            Cell::from(req),
            Cell::from(detail),
            Cell::from(t.timestamp.clone()),
        ]);
        if i == ui.target_idx {
            row = row.style(Style::default().bg(Color::DarkGray));
        }
        row
    });
    let widths = [
        Constraint::Length(14),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Min(16),
        Constraint::Length(20),
    ];
    let title = format!(
        "Targets ({})  [↑/↓ select, +/- range {}h]",
        data.targets.len(),
        ui.range_hours
    );
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .column_spacing(1);
    f.render_widget(table, area);
}

fn render_alerts(f: &mut Frame, area: Rect, data: &AppData) {
    let header = Row::new(vec!["Status", "Sev", "Source", "Message", "Created"])
        .style(Style::default().add_modifier(Modifier::BOLD));
    let rows = data.alerts.iter().take(200).map(|a| {
        let st = match a.status.as_str() {
            "firing" => Style::default().fg(Color::Red),
            "resolved" => Style::default().fg(Color::Green),
            _ => Style::default().fg(Color::Gray),
        };
        Row::new(vec![
            Cell::from(Span::styled(a.status.clone(), st)),
            Cell::from(a.severity.clone()),
            Cell::from(a.source.clone()),
            Cell::from(a.message.clone()),
            Cell::from(a.created_at.clone()),
        ])
    });
    let widths = [
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Length(14),
        Constraint::Min(20),
        Constraint::Length(20),
    ];
    let firing = data.alerts.iter().filter(|a| a.status == "firing").count();
    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Alerts (firing {firing} / total {})", data.alerts.len())),
    );
    f.render_widget(table, area);
}

fn render_events(f: &mut Frame, area: Rect, data: &AppData) {
    let header = Row::new(vec!["Level", "Kind", "Source", "Message", "Time"])
        .style(Style::default().add_modifier(Modifier::BOLD));
    let rows = data.events.iter().take(200).map(|e| {
        let st = match e.level.as_str() {
            "error" => Style::default().fg(Color::Red),
            "warn" => Style::default().fg(Color::Yellow),
            _ => Style::default().fg(Color::Green),
        };
        Row::new(vec![
            Cell::from(Span::styled(e.level.clone(), st)),
            Cell::from(e.kind.clone()),
            Cell::from(e.source.clone().unwrap_or_else(|| "-".into())),
            Cell::from(e.message.clone()),
            Cell::from(e.timestamp.clone()),
        ])
    });
    let widths = [
        Constraint::Length(7),
        Constraint::Length(14),
        Constraint::Length(12),
        Constraint::Min(20),
        Constraint::Length(20),
    ];
    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Events ({})", data.events.len())),
    );
    f.render_widget(table, area);
}

fn render_footer(f: &mut Frame, area: Rect, cfg: &Config, data: &AppData, ui: &UiState) {
    let mut spans = vec![
        Span::styled(" q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": quit "),
        Span::styled("Tab/1-4", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": views "),
        Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": refresh "),
        Span::styled("+/-", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!(": range {}h ", ui.range_hours)),
        Span::raw(format!(
            "| int {}s ret {}h targets {}",
            cfg.interval_seconds,
            cfg.retention_hours,
            cfg.targets.len()
        )),
    ];
    if let Some(err) = &data.last_error {
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(err, Style::default().fg(Color::Red)));
    } else if let Some(err) = &data.runtime.last_collection_error {
        spans.push(Span::raw(" | last err: "));
        spans.push(Span::styled(err, Style::default().fg(Color::Red)));
    }
    let p = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn sparkline<F: Fn(&crate::db::SystemMetric) -> f64>(hist: &[crate::db::SystemMetric], f: F) -> String {
    if hist.is_empty() {
        return "-".to_string();
    }
    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let take = 40usize.min(hist.len());
    let step = (hist.len() as f64 / take as f64).ceil() as usize;
    let mut out = String::new();
    let mut i = 0;
    while i < hist.len() && out.chars().count() < take {
        let v = f(&hist[i]).clamp(0.0, 100.0);
        let idx = ((v / 100.0) * (chars.len() - 1) as f64).round() as usize;
        out.push(chars[idx.min(chars.len() - 1)]);
        i += step.max(1);
    }
    out
}

fn top_procs_str(raw: Option<&str>) -> String {
    let Some(raw) = raw else {
        return "-".to_string();
    };
    match serde_json::from_str::<Vec<serde_json::Value>>(raw) {
        Ok(v) if !v.is_empty() => v
            .iter()
            .take(5)
            .map(|p| {
                let n = p.get("name").and_then(|x| x.as_str()).unwrap_or("?");
                let c = p.get("cpu").and_then(|x| x.as_f64()).unwrap_or(0.0);
                format!("{n} {c:.0}%")
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => "-".to_string(),
    }
}

fn fmt_uptime(sec: i64) -> String {
    let d = sec / 86400;
    let h = (sec % 86400) / 3600;
    let m = (sec % 3600) / 60;
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

fn fmt_bytes(bytes_per_sec: f64) -> String {
    let b = bytes_per_sec.max(0.0);
    if b >= 1024.0 * 1024.0 {
        format!("{:.2} MB", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else {
        format!("{b:.0} B")
    }
}

fn fmt_bytes_total(bytes: i64) -> String {
    let b = bytes.max(0) as f64;
    if b >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} GB", b / (1024.0 * 1024.0 * 1024.0))
    } else if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else {
        format!("{b:.0} B")
    }
}
