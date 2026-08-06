use std::{
    io,
    time::{Duration, Instant},
};

use chrono::{Local, Utc};
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
    news::{
        FeedNewsClient, FeedView, MacUrlOpener, NewsFeed, NewsMonitor, NewsSelection, NewsView,
        activate_selected,
    },
    preferences::{PreferencesStore, SaveOutcome, Theme},
    screen_mode,
    telemetry::{LedState, MacTelemetrySource, TelemetryMonitor, TelemetryView},
    weather::{WeatherLed, WeatherMonitor, WeatherReport, WeatherView, WttrWeatherClient},
};

#[derive(Clone, Copy)]
struct Palette {
    primary: Color,
    secondary: Color,
    accent: Color,
    dim: Color,
}

struct DashboardData<'a> {
    app: &'a AppState,
    telemetry: &'a TelemetryView,
    weather: &'a WeatherView,
    news: &'a NewsView,
    news_selection: &'a NewsSelection,
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
    let mut weather = WeatherMonitor::new(
        WttrWeatherClient::new(),
        Duration::from_secs(15 * 60),
        Instant::now(),
    );
    let mut news = NewsMonitor::new(
        FeedNewsClient::new(),
        Duration::from_secs(15 * 60),
        Instant::now(),
    );
    let mut news_selection = NewsSelection::default();
    let mut url_opener = MacUrlOpener;
    loop {
        let now = Instant::now();
        telemetry.tick(now);
        weather.tick(now, &app.preferences().city);
        news.tick(now);
        terminal.draw(|frame| {
            render_with_news(
                frame,
                &app,
                telemetry.view(),
                weather.view(),
                news.view(),
                &news_selection,
            )
        })?;
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
        let news_controls_active = !app.is_first_run()
            && !app.is_help_visible()
            && !app.is_settings_visible()
            && !app.is_city_editing();
        if news_controls_active {
            match command {
                KeyCommand::Character('n') => news.refresh_now(Instant::now()),
                KeyCommand::Left => news_selection.left(news.view()),
                KeyCommand::Right => news_selection.right(news.view()),
                KeyCommand::Up => news_selection.up(news.view()),
                KeyCommand::Down => news_selection.down(news.view()),
                KeyCommand::Enter => {
                    if let Err(message) =
                        activate_selected(&news_selection, news.view(), &mut url_opener)
                    {
                        app.set_warning(Some(message));
                    }
                }
                _ => {}
            }
        }
        if app.handle_key(command) == AppAction::Quit {
            return Ok(());
        }
        if app.take_weather_refresh_requested() {
            weather.refresh_now(Instant::now());
        }
        if let Some(preferences) = app.take_preferences_changed() {
            match store.save(&preferences) {
                SaveOutcome::Saved => app.set_warning(None),
                SaveOutcome::SessionOnly(warning) => app.set_warning(Some(warning)),
            }
        }
    }
}

pub fn render(frame: &mut Frame, app: &AppState, telemetry: &TelemetryView, weather: &WeatherView) {
    render_with_news(
        frame,
        app,
        telemetry,
        weather,
        &NewsView::loading(),
        &NewsSelection::default(),
    );
}

pub fn render_with_news(
    frame: &mut Frame,
    app: &AppState,
    telemetry: &TelemetryView,
    weather: &WeatherView,
    news: &NewsView,
    news_selection: &NewsSelection,
) {
    let area = frame.area();
    let colors = palette(app.preferences().theme);
    if screen_mode(area.width, area.height) == ScreenMode::Resize {
        render_resize(frame, area, colors);
        return;
    }
    render_dashboard(
        frame,
        area,
        DashboardData {
            app,
            telemetry,
            weather,
            news,
            news_selection,
        },
        colors,
    );
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

fn render_dashboard(frame: &mut Frame, area: Rect, data: DashboardData<'_>, colors: Palette) {
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

    let body = Layout::vertical([Constraint::Length(16), Constraint::Min(9)]).split(rows[1]);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(42),
            Constraint::Percentage(36),
            Constraint::Percentage(22),
        ])
        .split(body[0]);
    render_telemetry(frame, columns[0], data.telemetry, colors);
    clock(frame, columns[1], colors);
    let right = Layout::vertical([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(columns[2]);
    render_weather(frame, right[0], data.weather, colors);
    standby(frame, right[1], " PROJECT ", "NO PROJECT LOADED", colors);
    render_news(frame, body[1], data.news, data.news_selection, colors);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ● ", Style::default().fg(Color::Green)),
        Span::styled("CORE ONLINE", Style::default().fg(colors.primary)),
        Span::raw("   "),
        Span::styled("?", bold(colors.accent)),
        Span::raw(" HELP   "),
        Span::styled("s", bold(colors.accent)),
        Span::raw(" SETTINGS   "),
        Span::styled("w", bold(colors.accent)),
        Span::raw(" WEATHER   "),
        Span::styled("n", bold(colors.accent)),
        Span::raw(" NEWS   "),
        Span::styled("q", bold(colors.accent)),
        Span::raw(" QUIT"),
    ]))
    .alignment(Alignment::Center)
    .block(bordered(colors.secondary));
    frame.render_widget(footer, rows[2]);

    if let Some(warning) = data.app.warning() {
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

fn render_weather(frame: &mut Frame, area: Rect, weather: &WeatherView, colors: Palette) {
    let led = match weather.led() {
        WeatherLed::Green => Color::Green,
        WeatherLed::Orange => Color::Rgb(255, 165, 0),
        WeatherLed::Red => Color::Red,
    };
    let mut lines = vec![Line::from(vec![
        Span::styled("● ", Style::default().fg(led)),
        Span::styled(
            match weather {
                WeatherView::Loading => "LOADING",
                WeatherView::Ready { .. } => "ONLINE",
                WeatherView::Error {
                    last_good: Some(_), ..
                } => "STALE",
                WeatherView::Error {
                    last_good: None, ..
                } => "ERROR",
            },
            bold(led),
        ),
    ])];
    match weather {
        WeatherView::Loading => lines.push(Line::from("Loading weather…")),
        WeatherView::Ready { city, report } => weather_lines(&mut lines, city, report, colors),
        WeatherView::Error { message, last_good } => {
            if let Some((city, report)) = last_good {
                weather_lines(&mut lines, city, report, colors);
            }
            lines.push(Line::from(Span::styled(
                message.as_str(),
                Style::default().fg(Color::Red),
            )));
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(bordered(colors.secondary).title(" WEATHER // 15m ")),
        area,
    );
}

fn render_news(
    frame: &mut Frame,
    area: Rect,
    news: &NewsView,
    selection: &NewsSelection,
    colors: Palette,
) {
    let columns = Layout::horizontal([
        Constraint::Percentage(34),
        Constraint::Percentage(33),
        Constraint::Percentage(33),
    ])
    .split(area);
    let now = Utc::now();
    for (column, feed) in columns.iter().zip(NewsFeed::ALL) {
        let (status, led) = match news.feed(feed) {
            FeedView::Loading { last_good: Some(_) } => ("REFRESH", Color::Rgb(255, 165, 0)),
            FeedView::Loading { last_good: None } => ("LOADING", Color::Rgb(255, 165, 0)),
            FeedView::Ready(_) => ("ONLINE", Color::Green),
            FeedView::Error {
                last_good: Some(_), ..
            } => ("STALE", Color::Red),
            FeedView::Error {
                last_good: None, ..
            } => ("ERROR", Color::Red),
        };
        let mut lines = vec![Line::from(vec![
            Span::styled("● ", Style::default().fg(led)),
            Span::styled(status, bold(led)),
        ])];
        for (index, headline) in news.headlines(feed).iter().enumerate() {
            let marker = if selection.is_selected(feed, index) {
                "▶"
            } else {
                " "
            };
            let style = if selection.is_selected(feed, index) {
                bold(colors.accent)
            } else {
                Style::default().fg(colors.dim)
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "{marker} {}  {}",
                    relative_time(headline.published_at, now),
                    headline.title
                ),
                style,
            )));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .block(bordered(colors.secondary).title(format!(" {} ", feed.label()))),
            *column,
        );
    }
}

fn relative_time(published_at: chrono::DateTime<Utc>, now: chrono::DateTime<Utc>) -> String {
    let seconds = now.signed_duration_since(published_at).num_seconds().max(0);
    if seconds < 60 {
        "now".into()
    } else if seconds < 60 * 60 {
        format!("{}m", seconds / 60)
    } else if seconds < 24 * 60 * 60 {
        format!("{}h", seconds / (60 * 60))
    } else {
        format!("{}d", seconds / (24 * 60 * 60))
    }
}

fn weather_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    city: &'a str,
    report: &WeatherReport,
    colors: Palette,
) {
    let updated: chrono::DateTime<Local> = report.updated_at.into();
    lines.extend([
        Line::from(Span::styled(city, bold(colors.primary))),
        Line::from(report.condition.clone()),
        Line::from(format!(
            "{}°C  //  Feels {}°C",
            report.temperature_c, report.feels_like_c
        )),
        Line::from(format!("Humidity {}%", report.humidity_percent)),
        Line::from(format!("Wind {} km/h", report.wind_kmh)),
        Line::from(format!("Updated {}", updated.format("%H:%M"))),
    ]);
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
        key_line("w", "Refresh weather", colors),
        key_line("n", "Refresh news", colors),
        key_line("Arrows / Enter", "Select / open headline", colors),
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
