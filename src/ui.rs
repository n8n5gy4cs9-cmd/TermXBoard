use std::{io, time::Duration};

use chrono::Local;
use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{AppAction, AppState, KeyCommand, MIN_HEIGHT, MIN_WIDTH, ScreenMode, screen_mode};

const CYAN: Color = Color::Rgb(0, 240, 255);
const VIOLET: Color = Color::Rgb(154, 77, 255);
const MAGENTA: Color = Color::Rgb(255, 45, 149);
const DIM: Color = Color::Rgb(86, 104, 122);

pub fn run() -> io::Result<()> {
    let _session = TerminalSession::enter()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let result = run_loop(&mut terminal);
    terminal.show_cursor()?;
    result
}

struct TerminalSession {
    alternate_screen: bool,
}

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut session = Self {
            alternate_screen: false,
        };

        execute!(io::stdout(), EnterAlternateScreen)?;
        session.alternate_screen = true;
        Ok(session)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.alternate_screen {
            let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
        }
        let _ = disable_raw_mode();
    }
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = AppState::default();

    loop {
        terminal.draw(|frame| render(frame, &app))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let command = match key.code {
            KeyCode::Char(character) => KeyCommand::Character(character),
            _ => continue,
        };
        if app.handle_key(command) == AppAction::Quit {
            return Ok(());
        }
    }
}

fn render(frame: &mut Frame, app: &AppState) {
    let area = frame.area();
    if screen_mode(area.width, area.height) == ScreenMode::Resize {
        render_resize(frame, area);
        return;
    }

    render_dashboard(frame, area);
    if app.is_help_visible() {
        render_help(frame, centered_rect(58, 14, area));
    }
}

fn render_resize(frame: &mut Frame, area: Rect) {
    let message = Paragraph::new(vec![
        Line::from(Span::styled(
            "◈  TERMINAL GEOMETRY INSUFFICIENT",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "Current: {}×{}  //  Required: {MIN_WIDTH}×{MIN_HEIGHT}",
            area.width, area.height
        )),
        Line::from("Resize the terminal to restore Dashboard."),
    ])
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(MAGENTA))
            .title(" TermXBoard // RESIZE "),
    );
    frame.render_widget(message, area);
}

fn render_dashboard(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(18),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " ◈ TERM",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "X",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "BOARD ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("// COMMAND DECK", Style::default().fg(DIM)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(VIOLET)),
    );
    frame.render_widget(header, rows[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22),
            Constraint::Percentage(56),
            Constraint::Percentage(22),
        ])
        .split(rows[1]);
    render_standby_panel(frame, columns[0], " SYSTEM ", "TELEMETRY\nLINK STANDBY");
    render_clock(frame, columns[1]);
    render_standby_panel(frame, columns[2], " PROJECT ", "NO PROJECT\nLOADED");

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ● ", Style::default().fg(Color::Green)),
        Span::styled("CORE ONLINE", Style::default().fg(CYAN)),
        Span::raw("   "),
        Span::styled(
            "?",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" HELP   "),
        Span::styled(
            "q",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" QUIT"),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(VIOLET)),
    );
    frame.render_widget(footer, rows[2]);
}

fn render_clock(frame: &mut Frame, area: Rect) {
    let now = Local::now();
    let clock = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            now.format("%H:%M:%S").to_string(),
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            now.format("%A  //  %d %B %Y").to_string().to_uppercase(),
            Style::default().fg(VIOLET),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "DASHBOARD READY",
            Style::default().fg(Color::Green),
        )),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(CYAN))
            .title(" LOCAL CHRONOMETER "),
    );
    frame.render_widget(clock, area);
}

fn render_standby_panel(frame: &mut Frame, area: Rect, title: &str, message: &str) {
    let panel = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("◇", Style::default().fg(VIOLET))),
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(DIM))),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(VIOLET))
            .title(title),
    );
    frame.render_widget(panel, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    let help = Paragraph::new(vec![
        Line::from(Span::styled(
            "KEYBOARD CONTROL",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("?", Style::default().fg(MAGENTA)),
            Span::raw("  Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("q", Style::default().fg(MAGENTA)),
            Span::raw("  Quit immediately"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "More controls activate with later modules.",
            Style::default().fg(DIM),
        )),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(MAGENTA))
            .title(" HELP // ? TO CLOSE "),
    );
    frame.render_widget(help, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
