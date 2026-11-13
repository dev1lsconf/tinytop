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
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, Wrap},
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

struct AppData {
    system: Option<crate::db::SystemMetric>,
    targets: Vec<crate::db::TargetMetric>,
    runtime: RuntimeStatus,
    last_error: Option<String>,
}

async fn load_data(db: &Database, cfg: &Config, runtime: &Arc<RwLock<RuntimeStatus>>) -> AppData {
    let mut last_error = None;
    let system = match db.latest_system().await {
        Ok(v) => v,
        Err(e) => {
            last_error = Some(format!("db: {e}"));
            None
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
            Err(e) => {
                last_error = Some(format!("db: {e}"));
            }
        }
    }
    let runtime = runtime.read().await.clone();
    AppData {
        system,
        targets,
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
    let mut data = load_data(&db, &cfg_snap, &runtime).await;
    let tick_rate = Duration::from_secs(1);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &cfg_snap, &data))?;

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
                    data = load_data(&db, &cs, &runtime).await;
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
            data = load_data(&db, &cs, &runtime).await;
            cfg_snap = cs;
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, cfg: &Config, data: &AppData) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(f, chunks[0], data);
    render_system(f, chunks[1], &data.system);
    render_targets(f, chunks[2], &data.targets);
    render_footer(f, chunks[3], cfg, data);
}

fn render_header(f: &mut Frame, area: Rect, data: &AppData) {
    let status = if data.runtime.last_collection_ok == Some(true) {
        Span::styled("● OK", Style::default().fg(Color::Green))
    } else if data.runtime.collections_total == 0 {
        Span::styled("● starting", Style::default().fg(Color::Yellow))
    } else {
        Span::styled("● degraded", Style::default().fg(Color::Red))
    };
    let line = Line::from(vec![
        Span::styled(
            format!(" TinyTop v{} ", data.runtime.version),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        status,
        Span::raw(" | collections: "),
        Span::raw(data.runtime.collections_total.to_string()),
        Span::raw(" (errors: "),
        Span::raw(data.runtime.collection_errors_total.to_string()),
        Span::raw(")"),
    ]);
    let block = Block::default().borders(Borders::ALL).title("TUI");
    let p = Paragraph::new(line).block(block).alignment(Alignment::Left);
    f.render_widget(p, area);
}

fn render_system(f: &mut Frame, area: Rect, system: &Option<crate::db::SystemMetric>) {
    let block = Block::default().borders(Borders::ALL).title("System");
    if let Some(s) = system {
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
        let net = Line::from(vec![
            Span::styled("Net  ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "↓ {}/s  ↑ {}/s  (rx {} tx {})",
                fmt_bytes(s.net_rx_bytes_per_sec),
                fmt_bytes(s.net_tx_bytes_per_sec),
                fmt_bytes_total(s.net_rx_bytes),
                fmt_bytes_total(s.net_tx_bytes),
            )),
        ]);
        f.render_widget(Paragraph::new(net), rows[4]);
        let ts = Line::from(vec![
            Span::styled("Last ", Style::default().fg(Color::DarkGray)),
            Span::raw(&s.timestamp),
        ]);
        f.render_widget(Paragraph::new(ts), rows[5]);
    } else {
        let p = Paragraph::new("No system metrics yet (waiting for first collection)…")
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(p, area);
    }
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
    let lbl = Paragraph::new(Span::styled(
        format!("{label:<5}"),
        Style::default().fg(Color::Cyan),
    ));
    f.render_widget(lbl, cols[0]);
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
    let d = Paragraph::new(detail).alignment(Alignment::Right);
    f.render_widget(d, cols[2]);
}

fn render_targets(f: &mut Frame, area: Rect, targets: &[crate::db::TargetMetric]) {
    let header = Row::new(vec!["Name", "Kind", "Status", "Latency", "Requests", "URL"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);
    let rows = targets.iter().map(|t| {
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
        let reqs = t
            .http_requests
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into());
        Row::new(vec![
            Cell::from(t.target.clone()),
            Cell::from(t.kind.clone()),
            Cell::from(Span::styled(t.status.clone(), status_style)),
            Cell::from(latency),
            Cell::from(reqs),
            Cell::from(t.url.clone()),
        ])
    });
    let widths = [
        Constraint::Length(16),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Min(20),
    ];
    let title = format!("Targets ({})", targets.len());
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .column_spacing(1);
    f.render_widget(table, area);
}

fn render_footer(f: &mut Frame, area: Rect, cfg: &Config, data: &AppData) {
    let mut spans = vec![
        Span::styled(" q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": quit  "),
        Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": refresh  "),
        Span::raw(format!(
            "| interval {}s | retention {}h | targets {}",
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
