use anyhow::Result;
use chrono::{DateTime, Utc};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};
use sqlx::{PgPool, Row as _};
use tokio::time::{self, Duration};
use tracing::warn;

/// Represents a signal for display in the TUI.
#[derive(Debug, Clone)]
pub struct UISignal {
    pub timestamp: DateTime<Utc>,
    pub market_id: String,
    pub question: String,
    pub side: String,      // "BUY" or "SELL"
    pub probability: f64,  // 0.0 to 1.0
    pub direction: String, // same as side for display
    pub score: f64,        // Kelly size (bets) or |edge| (rejected)
    pub status: String,    // ACTIVE / WON / LOST / REJECTED
    pub source: String,    // xgboost, llm_consensus, rejected, ...
}

/// State for the TUI application.
pub struct TuiState {
    pub signals: Vec<UISignal>,
    pub table_state: TableState,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
            table_state: TableState::default(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) if i >= self.signals.len().saturating_sub(1) => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(0) | None => self.signals.len().saturating_sub(1),
            Some(i) => i - 1,
        };
        self.table_state.select(Some(i));
    }

    /// Merge newly fetched signals, deduping by (market_id, question) within a
    /// short time window, newest first, capped at 1000.
    pub fn add_signals(&mut self, new_signals: Vec<UISignal>) {
        for signal in new_signals {
            let is_duplicate = self.signals.iter().any(|existing| {
                existing.market_id == signal.market_id
                    && existing.question == signal.question
                    && (existing.timestamp - signal.timestamp).num_seconds().abs() < 10
            });
            if !is_duplicate {
                self.signals.push(signal);
            }
        }
        self.signals.sort_by_key(|s| std::cmp::Reverse(s.timestamp));
        self.signals.truncate(1000);
        if self.table_state.selected().is_none() && !self.signals.is_empty() {
            self.table_state.select(Some(0));
        }
    }
}

fn truncate_string(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Query the database for recent signals (placed bets + rejected signals,
/// last 5 minutes). Uses the shared SQLx pool — same layer as the rest of the bot.
async fn fetch_recent_signals(pool: &PgPool) -> Result<Vec<UISignal>> {
    let mut signals = Vec::new();

    let bet_rows = sqlx::query(
        "SELECT market_id, question, side, estimated_prob, kelly_size, source, \
                placed_at, resolved, won \
         FROM bets \
         WHERE placed_at > NOW() - INTERVAL '5 minutes' \
         ORDER BY placed_at DESC",
    )
    .fetch_all(pool)
    .await?;

    for row in bet_rows {
        let side: String = row.get("side");
        let resolved: bool = row.get("resolved");
        let won: Option<bool> = row.get("won");
        let (side_str, status) = if resolved {
            let s = if won.unwrap_or(false) { "WON" } else { "LOST" };
            (if side == "Yes" { "BUY" } else { "SELL" }, s.to_string())
        } else {
            (
                if side == "Yes" { "BUY" } else { "SELL" },
                "ACTIVE".to_string(),
            )
        };
        signals.push(UISignal {
            timestamp: row.get("placed_at"),
            market_id: row.get("market_id"),
            question: row.get("question"),
            side: side_str.to_string(),
            probability: row.get("estimated_prob"),
            direction: side_str.to_string(),
            score: row.get("kelly_size"),
            status,
            source: row.get("source"),
        });
    }

    let rejected_rows = sqlx::query(
        "SELECT market_id, question, estimated_prob, edge, created_at \
         FROM rejected_signals \
         WHERE created_at > NOW() - INTERVAL '5 minutes' \
         ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    for row in rejected_rows {
        let probability: f64 = row.try_get("estimated_prob").unwrap_or(0.5);
        let edge: f64 = row.try_get("edge").unwrap_or(0.0);
        let side_str = if probability > 0.5 { "BUY" } else { "SELL" };
        signals.push(UISignal {
            timestamp: row.get("created_at"),
            market_id: row.get("market_id"),
            question: row.get("question"),
            side: side_str.to_string(),
            probability,
            direction: side_str.to_string(),
            score: edge.abs(),
            status: "REJECTED".to_string(),
            source: "rejected".to_string(),
        });
    }

    Ok(signals)
}

/// Run the TUI application against the shared SQLx pool.
pub async fn run_tui(pool: PgPool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let res = run_loop(&mut terminal, &pool).await;

    // Always restore the terminal, even on error.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    pool: &PgPool,
) -> Result<()> {
    let mut state = TuiState::new();
    let mut tick = time::interval(Duration::from_secs(2));

    if let Ok(initial) = fetch_recent_signals(pool).await {
        state.add_signals(initial);
    }

    loop {
        terminal.draw(|f| draw(f, &mut state))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Up => state.previous(),
                KeyCode::Down => state.next(),
                KeyCode::Home => state.table_state.select(Some(0)),
                KeyCode::End if !state.signals.is_empty() => {
                    state.table_state.select(Some(state.signals.len() - 1));
                }
                _ => {}
            }
        }

        tick.tick().await;
        match fetch_recent_signals(pool).await {
            Ok(new) if !new.is_empty() => state.add_signals(new),
            Ok(_) => {}
            Err(e) => warn!("Failed to fetch signals: {}", e),
        }
    }

    Ok(())
}

fn draw(f: &mut ratatui::Frame, state: &mut TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(80), Constraint::Length(3)])
        .split(f.size());

    let widths = [
        Constraint::Length(8),
        Constraint::Length(40),
        Constraint::Length(8),
        Constraint::Length(4),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(12),
    ];

    let rows = state.signals.iter().map(|s| {
        let time_str = s.timestamp.format("%H:%M:%S").to_string();
        let prob_percent = (s.probability * 100.0).round() as u64;
        let side_color = if s.side == "BUY" {
            Color::Green
        } else {
            Color::Red
        };
        let status_color = match s.status.as_str() {
            "ACTIVE" | "NEW" => Color::Yellow,
            "WON" => Color::Green,
            "LOST" | "REJECTED" => Color::Red,
            _ => Color::White,
        };
        Row::new(vec![
            Cell::from(Span::raw(time_str)),
            Cell::from(Span::raw(truncate_string(&s.question, 38))),
            Cell::from(Span::raw(format!("{prob_percent}%"))),
            Cell::from(Span::styled(
                s.direction.clone(),
                Style::default().fg(side_color),
            )),
            Cell::from(Span::raw(format!("{:.2}", s.score))),
            Cell::from(Span::styled(
                s.status.clone(),
                Style::default().fg(status_color),
            )),
            Cell::from(Span::raw(s.source.clone())),
        ])
        .height(1)
    });

    let header = Row::new(vec![
        "Time", "Market", "Prob %", "Dir", "Score", "Status", "Source",
    ])
    .style(Style::default().fg(Color::Yellow))
    .bottom_margin(1);

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title("Real-time Signals")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, chunks[0], &mut state.table_state);

    let last = state
        .signals
        .first()
        .map(|s| s.timestamp.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "--:--:--".to_string());
    let footer = Paragraph::new(Line::from(vec![Span::raw(format!(
        "Signals: {} | Last: {} | [q] Quit",
        state.signals.len(),
        last
    ))]))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(footer, chunks[1]);
}
