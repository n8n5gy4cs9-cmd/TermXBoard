use std::{
    io,
    time::{Duration, Instant},
};

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

use crate::{
    AppAction, AppState, KeyCommand, MIN_HEIGHT, MIN_WIDTH, ScreenMode, SettingsField,
    preferences::{PreferencesStore, SaveOutcome, Theme},
    screen_mode,
    telemetry::{LedState, MacTelemetrySource, TelemetryMonitor, TelemetryView},
};

#[derive(Clone, Copy)]
struct Palette {
    primary: Color,
    secondary: Color,
    accent: Color,
    dim: Color,
}

impl Palette {
    fn new(primary: u32, secondary: u32, accent: u32, dim: u32) -> Self {
        let color = |value| {
            Color::Rgb(
                ((value >> 16) & 0xff_u32) as u8,
                ((value >> 8) & 0xff_u32) as u8,
                (value & 0xff_u32) as u8,
            )
        };
        Self {
            primary: color(primary),
            secondary: color(secondary),
            accent: color(accent),
            dim: color(dim),
        }
    }
}

fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::SignatureNeon => Palette::new(0x00f0ff, 0x9a4dff, 0xff2d95, 0x56687a),
        Theme::Cyberpunk => Palette::new(0xffea00, 0x00ffd1, 0xff006e, 0x6e6080),
        Theme::Matrix => Palette::new(0x00ff41, 0x00b42d, 0xaaffbe, 0x376441),
        Theme::Nord => Palette::new(0x88c0d0, 0x81a1c1, 0xb48ead, 0x4c566a),
        Theme::Dracula => Palette::new(0x8be9fd, 0xbd93f9, 0xff79c6, 0x6272a4),
        Theme::SolarizedDark => Palette::new(0x2aa198, 0x268bd2, 0xd33682, 0x586e75),
        Theme::AmberCrt => Palette::new(0xffb000, 0xff8000, 0xffd666, 0x805714),
    }
}

pub fn run() -> io::Result<()> {
    let store = PreferencesStore::beside_executable()?;
    let loaded = store.load();
    let app = AppState::new(loaded.preferences, loaded.is_first_run, loaded.warning);
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let result = run_loop(&mut terminal, &store, app);
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

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    store: &PreferencesStore,
    mut app: AppState,
) -> io::Result<()> {
    let mut telemetry = TelemetryMonitor::new(
        MacTelemetrySource::new(),
        Duration::from_secs(3),
        Instant::now(),
    );
    loop {
        telemetry.tick(Instant::now());
        terminal.draw(|frame| render(frame, &app, telemetry.view()))?;
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
            KeyCode::Up => KeyCommand::Up,
            KeyCode::Down => KeyCommand::Down,
            KeyCode::Left => KeyCommand::Left,
            KeyCode::Right => KeyCommand::Right,
            KeyCode::Enter => KeyCommand::Enter,
            KeyCode::Esc => KeyCommand::Escape,
            KeyCode::Backspace => KeyCommand::Backspace,
            _ => continue,
        };
        if app.handle_key(command) == AppAction::Quit {
            return Ok(());
        }
        if let Some(preferences) = app.take_preferences_changed() {
            match store.save(&preferences) {
                SaveOutcome::Saved => app.set_warning(None),
                SaveOutcome::SessionOnly(warning) => app.set_warning(Some(warning)),
            }
        }
    }
}

pub fn render(frame: &mut Frame, app: &AppState, telemetry: &TelemetryView) {
    let area = frame.area();
    let colors = palette(app.preferences().theme);
    if screen_mode(area.width, area.height) == ScreenMode::Resize {
        render_resize(frame, area, colors);
        return;
    }
    render_dashboard(frame, area, app, telemetry, colors);
    if app.is_help_visible() {
        render_help(frame, centered_rect(58, 14, area), colors);
    } else if app.is_first_run() {
        render_first_run(frame, centered_rect(68, 12, area), app, colors);
    } else if app.is_settings_visible() {
        render_settings(frame, centered_rect(72, 16, area), app, colors);
    }
}

fn render_resize(frame: &mut Frame, area: Rect, colors: Palette) {
    let message = Paragraph::new(vec![
        Line::from(Span::styled(
            "◈  TERMINAL GEOMETRY INSUFFICIENT",
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
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
            .border_style(Style::default().fg(colors.accent))
            .title(" TermXBoard // RESIZE "),
    );
    frame.render_widget(message, area);
}

fn render_dashboard(
    frame: &mut Frame,
    area: Rect,
    app: &AppState,
    telemetry: &TelemetryView,
    colors: Palette,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(18),
            Constraint::Length(3),
        ])
        .split(area);
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" ◈ TERM", bold(colors.primary)),
        Span::styled("X", bold(colors.accent)),
        Span::styled("BOARD ", bold(colors.primary)),
        Span::styled("// COMMAND DECK", Style::default().fg(colors.dim)),
    ]))
    .block(bordered(colors.secondary));
    frame.render_widget(header, rows[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(42),
            Constraint::Percentage(36),
            Constraint::Percentage(22),
        ])
        .split(rows[1]);
    render_telemetry(frame, columns[0], telemetry, colors);
    clock(frame, columns[1], colors);
    standby(frame, columns[2], " PROJECT ", "NO PROJECT\nLOADED", colors);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ● ", Style::default().fg(Color::Green)),
        Span::styled("CORE ONLINE", Style::default().fg(colors.primary)),
        Span::raw("   "),
        Span::styled("?", bold(colors.accent)),
        Span::raw(" HELP   "),
        Span::styled("s", bold(colors.accent)),
        Span::raw(" SETTINGS   "),
        Span::styled("q", bold(colors.accent)),
        Span::raw(" QUIT"),
    ]))
    .alignment(Alignment::Center)
    .block(bordered(colors.secondary));
    frame.render_widget(footer, rows[2]);

    if let Some(warning) = app.warning() {
        let width = area.width.saturating_sub(4);
        frame.render_widget(
            Paragraph::new(format!("⚠ {warning}")).style(Style::default().fg(Color::Yellow)),
            Rect::new(area.x + 2, area.y + area.height - 5, width, 1),
        );
    }
}

fn render_telemetry(frame: &mut Frame, area: Rect, telemetry: &TelemetryView, colors: Palette) {
    let block = bordered(colors.secondary).title(" SYSTEM TELEMETRY // 3s ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([
        Constraint::Percentage(34),
        Constraint::Percentage(33),
        Constraint::Percentage(33),
    ])
    .split(inner);
    let cards = telemetry.cards();
    for (row_index, row) in rows.iter().enumerate() {
        let columns = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(*row);
        for (column_index, column) in columns.iter().enumerate() {
            let card = &cards[row_index * 2 + column_index];
            let led = match card.led {
                LedState::Green => Color::Green,
                LedState::Orange => Color::Rgb(255, 165, 0),
                LedState::Red => Color::Red,
            };
            let widget = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("● ", Style::default().fg(led)),
                    Span::styled(card.label, bold(colors.primary)),
                ]),
                Line::from(Span::styled(
                    card.value.as_str(),
                    Style::default().fg(colors.dim),
                )),
            ])
            .alignment(Alignment::Center)
            .block(bordered(colors.secondary));
            frame.render_widget(widget, *column);
        }
    }
}

fn clock(frame: &mut Frame, area: Rect, colors: Palette) {
    let now = Local::now();
    let widget = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            now.format("%H:%M:%S").to_string(),
            bold(colors.primary),
        )),
        Line::from(""),
        Line::from(Span::styled(
            now.format("%A  //  %d %B %Y").to_string().to_uppercase(),
            Style::default().fg(colors.secondary),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "DASHBOARD READY",
            Style::default().fg(Color::Green),
        )),
    ])
    .alignment(Alignment::Center)
    .block(bordered(colors.primary).title(" LOCAL CHRONOMETER "));
    frame.render_widget(widget, area);
}

fn standby(frame: &mut Frame, area: Rect, title: &str, message: &str, colors: Palette) {
    let widget = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("◇", Style::default().fg(colors.secondary))),
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(colors.dim))),
    ])
    .alignment(Alignment::Center)
    .block(bordered(colors.secondary).title(title));
    frame.render_widget(widget, area);
}

fn render_help(frame: &mut Frame, area: Rect, colors: Palette) {
    frame.render_widget(Clear, area);
    let help = Paragraph::new(vec![
        Line::from(Span::styled("KEYBOARD CONTROL", bold(colors.primary))),
        Line::from(""),
        key_line("?", "Toggle this help", colors),
        key_line("s", "Toggle Settings", colors),
        key_line("q", "Quit immediately", colors),
    ])
    .alignment(Alignment::Center)
    .block(bordered(colors.accent).title(" HELP // ? TO CLOSE "));
    frame.render_widget(help, area);
}

fn render_first_run(frame: &mut Frame, area: Rect, app: &AppState, colors: Palette) {
    frame.render_widget(Clear, area);
    let content = Paragraph::new(vec![
        Line::from(Span::styled("WELCOME TO TERMXBOARD", bold(colors.primary))),
        Line::from(""),
        Line::from("Weather city (type to replace default):"),
        Line::from(Span::styled(
            format!("  {}_", app.city_draft()),
            Style::default().fg(colors.accent),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter  Save and open Dashboard",
            Style::default().fg(colors.dim),
        )),
    ])
    .alignment(Alignment::Center)
    .block(bordered(colors.accent).title(" FIRST RUN "));
    frame.render_widget(content, area);
}

fn render_settings(frame: &mut Frame, area: Rect, app: &AppState, colors: Palette) {
    frame.render_widget(Clear, area);
    let selected = |field| {
        if app.settings_field() == field {
            ">"
        } else {
            " "
        }
    };
    let city = if app.is_city_editing() {
        format!("{}█", app.city_draft())
    } else {
        app.preferences().city.clone()
    };
    let content = Paragraph::new(vec![
        Line::from(Span::styled("PORTABLE PREFERENCES", bold(colors.primary))),
        Line::from(""),
        Line::from(format!(
            "{} City           {city}",
            selected(SettingsField::City)
        )),
        Line::from(format!(
            "{} Theme          {}",
            selected(SettingsField::Theme),
            app.preferences().theme.label()
        )),
        Line::from(format!(
            "{} Reduced motion {}",
            selected(SettingsField::ReducedMotion),
            if app.preferences().reduced_motion {
                "ON"
            } else {
                "OFF"
            }
        )),
        Line::from(""),
        Line::from(Span::styled(
            "↑/↓ select  ←/→ change  Enter edit/toggle  Esc close",
            Style::default().fg(colors.dim),
        )),
    ])
    .block(bordered(colors.accent).title(" SETTINGS "));
    frame.render_widget(content, area);
}

fn bold(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn bordered(color: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
}

fn key_line<'a>(key: &'a str, description: &'a str, colors: Palette) -> Line<'a> {
    Line::from(vec![
        Span::styled(key, Style::default().fg(colors.accent)),
        Span::raw(format!("  {description}")),
    ])
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
