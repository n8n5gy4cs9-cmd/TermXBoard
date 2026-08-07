use std::{
    io,
    path::{Path, PathBuf},
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
    AppAction, AppState, AppView, KeyCommand, MIN_HEIGHT, MIN_WIDTH, ScreenMode, SettingsField,
    news::{
        FeedNewsClient, FeedView, MacUrlOpener, NewsFeed, NewsMonitor, NewsSelection, NewsView,
        activate_selected,
    },
    preferences::{PreferencesStore, SaveOutcome, Theme},
    progress::{
        LoadMenuStage, LoadProjectMenu, LoadedProject, ProjectHealth, ProjectMonitor,
        ProjectSession, ProjectTask, REFRESH_INTERVAL, TaskBoard, TaskControls, TaskMenu,
        TaskProjection, TaskSelection,
    },
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
    reduced_motion: bool,
}

struct TaskViewData<'a> {
    app: &'a AppState,
    loaded: &'a LoadedProject,
    weather: &'a WeatherView,
    selection: &'a TaskSelection,
    controls: &'a TaskControls,
    monitor: Option<&'a ProjectMonitor>,
    reduced_motion: bool,
}

pub struct ApplicationData<'a> {
    pub app: &'a AppState,
    pub telemetry: &'a TelemetryView,
    pub weather: &'a WeatherView,
    pub news: &'a NewsView,
    pub news_selection: &'a NewsSelection,
    pub active_project: Option<&'a LoadedProject>,
    pub load_menu: &'a LoadProjectMenu,
    pub task_selection: &'a TaskSelection,
    pub task_controls: &'a TaskControls,
    pub project_monitor: Option<&'a ProjectMonitor>,
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
    let remembered_project = loaded.preferences.remembered_project.clone();
    let mut app = AppState::new(loaded.preferences, loaded.is_first_run, loaded.warning);
    let mut project_session = ProjectSession::new(remembered_project);
    let cwd = std::env::current_dir()?;
    if let Some(argument) = std::env::args_os().nth(1) {
        load_requested_project(
            &mut project_session,
            &mut app,
            PathBuf::from(argument),
            &cwd,
            &store,
        );
    }
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let result = run_loop(&mut terminal, &store, app, project_session, cwd);
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
    mut project_session: ProjectSession,
    cwd: PathBuf,
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
    let mut load_menu = LoadProjectMenu::default();
    let mut project_monitor: Option<ProjectMonitor> = project_session
        .active()
        .map(|loaded| ProjectMonitor::new(loaded, REFRESH_INTERVAL, Instant::now()));
    let mut task_selection = project_session
        .active()
        .map(|loaded| TaskSelection::for_project(&loaded.project))
        .unwrap_or_default();
    let mut task_controls = TaskControls::default();
    loop {
        let now = Instant::now();
        telemetry.tick(now);
        weather.tick(now, &app.preferences().city);
        news.tick(now);
        if let Some(ref mut monitor) = project_monitor {
            monitor.tick(now, app.view() == AppView::Task);
            revalidate_selection_after_refresh(
                &mut task_selection,
                monitor.active().map(|loaded| &loaded.project),
                task_controls.filters(),
                task_controls.group_by(),
            );
        }
        terminal.draw(|frame| {
            render_application(
                frame,
                ApplicationData {
                    app: &app,
                    telemetry: telemetry.view(),
                    weather: weather.view(),
                    news: news.view(),
                    news_selection: &news_selection,
                    active_project: project_monitor.as_ref().and_then(|m| m.active()),
                    load_menu: &load_menu,
                    task_selection: &task_selection,
                    task_controls: &task_controls,
                    project_monitor: project_monitor.as_ref(),
                },
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
        if load_menu.stage() != LoadMenuStage::Closed {
            if let Some(path) = load_menu.handle_key(command)
                && load_requested_project(&mut project_session, &mut app, path, &cwd, store)
            {
                if let Some(loaded) = project_session.active() {
                    project_monitor = Some(ProjectMonitor::new(
                        loaded,
                        REFRESH_INTERVAL,
                        Instant::now(),
                    ));
                    task_selection = TaskSelection::for_project(&loaded.project);
                }
                task_controls = TaskControls::default();
            }
            continue;
        }
        let view_controls_active = !app.is_first_run()
            && !app.is_help_visible()
            && !app.is_settings_visible()
            && !app.is_city_editing();
        if command == KeyCommand::Character('t')
            && project_monitor.as_ref().and_then(|m| m.active()).is_some()
            && view_controls_active
        {
            app.show_task_view();
            continue;
        }
        if app.view() == AppView::Task
            && view_controls_active
            && let Some(loaded) = project_monitor.as_ref().and_then(|m| m.active())
        {
            if task_controls.menu() != TaskMenu::Closed
                || matches!(
                    command,
                    KeyCommand::Character('f') | KeyCommand::Character('g') | KeyCommand::Escape
                )
            {
                task_controls.handle_key(command, &loaded.project);
                let projection = TaskProjection::new(
                    &loaded.project,
                    task_controls.filters(),
                    task_controls.group_by(),
                );
                task_selection.ensure_visible(&projection.ordered_tasks());
                continue;
            }
            let projection = TaskProjection::new(
                &loaded.project,
                task_controls.filters(),
                task_controls.group_by(),
            );
            let ordered_tasks = projection.ordered_tasks();
            match command {
                KeyCommand::Up | KeyCommand::Left => {
                    task_selection.previous_in(&ordered_tasks);
                    continue;
                }
                KeyCommand::Down | KeyCommand::Right => {
                    task_selection.next_in(&ordered_tasks);
                    continue;
                }
                KeyCommand::Character('r') => {
                    if let Some(ref mut monitor) = project_monitor {
                        monitor.refresh_now(Instant::now());
                    }
                    revalidate_selection_after_refresh(
                        &mut task_selection,
                        project_monitor
                            .as_ref()
                            .and_then(|m| m.active())
                            .map(|loaded| &loaded.project),
                        task_controls.filters(),
                        task_controls.group_by(),
                    );
                    continue;
                }
                _ => {}
            }
        }
        if command == KeyCommand::Character('l') && view_controls_active {
            load_menu.open(project_session.remembered_project().map(Path::to_path_buf));
            continue;
        }
        let news_controls_active = view_controls_active && app.view() == AppView::Dashboard;
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

fn load_requested_project(
    session: &mut ProjectSession,
    app: &mut AppState,
    path: PathBuf,
    cwd: &Path,
    store: &PreferencesStore,
) -> bool {
    match session.load(path, cwd) {
        Ok(loaded) => {
            let absolute_path = loaded.path.clone();
            app.show_task_view();
            app.remember_project(absolute_path);
            app.set_warning(None);
            if !app.is_first_run()
                && let Some(preferences) = app.take_preferences_changed()
            {
                match store.save(&preferences) {
                    SaveOutcome::Saved => app.set_warning(None),
                    SaveOutcome::SessionOnly(warning) => app.set_warning(Some(warning)),
                }
            }
            true
        }
        Err(error) => {
            app.set_warning(Some(error));
            false
        }
    }
}

fn revalidate_selection_after_refresh(
    selection: &mut TaskSelection,
    project: Option<&crate::progress::Project>,
    filters: &crate::progress::TaskFilters,
    group_by: crate::progress::GroupBy,
) {
    let Some(project) = project else {
        return;
    };
    let projection = TaskProjection::new(project, filters, group_by);
    let switched = project.current_task.as_ref().is_some_and(|id| {
        selection
            .selected(project)
            .map(|selected| selected.id != *id)
            .unwrap_or(true)
    });
    if switched {
        *selection = TaskSelection::for_project(project);
    }
    selection.ensure_visible(&projection.ordered_tasks());
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
    render_application(
        frame,
        ApplicationData {
            app,
            telemetry,
            weather,
            news,
            news_selection,
            active_project: None,
            load_menu: &LoadProjectMenu::default(),
            task_selection: &TaskSelection::default(),
            task_controls: &TaskControls::default(),
            project_monitor: None,
        },
    );
}

pub fn render_application(frame: &mut Frame, data: ApplicationData<'_>) {
    let ApplicationData {
        app,
        telemetry,
        weather,
        news,
        news_selection,
        active_project,
        load_menu,
        task_selection,
        task_controls,
        project_monitor,
    } = data;
    let area = frame.area();
    let colors = palette(app.preferences().theme);
    let reduced_motion = app.preferences().reduced_motion;
    if screen_mode(area.width, area.height) == ScreenMode::Resize {
        render_resize(frame, area, colors);
        return;
    }
    if app.view() == AppView::Task {
        if let Some(loaded) = active_project {
            render_task_view(
                frame,
                area,
                TaskViewData {
                    app,
                    loaded,
                    weather,
                    selection: task_selection,
                    controls: task_controls,
                    monitor: project_monitor,
                    reduced_motion,
                },
                colors,
            );
        } else {
            render_dashboard(
                frame,
                area,
                DashboardData {
                    app,
                    telemetry,
                    weather,
                    news,
                    news_selection,
                    reduced_motion,
                },
                colors,
            );
        }
    } else {
        render_dashboard(
            frame,
            area,
            DashboardData {
                app,
                telemetry,
                weather,
                news,
                news_selection,
                reduced_motion,
            },
            colors,
        );
    }
    if app.is_help_visible() {
        render_help(frame, centered_rect(58, 14, area), colors);
    } else if app.is_first_run() {
        render_first_run(frame, centered_rect(68, 12, area), app, colors);
    } else if app.is_settings_visible() {
        render_settings(frame, centered_rect(72, 16, area), app, colors);
    } else if load_menu.stage() != LoadMenuStage::Closed {
        render_load_menu(frame, centered_rect(76, 14, area), load_menu, colors);
    } else if let (Some(loaded), menu) = (active_project, task_controls.menu())
        && menu != TaskMenu::Closed
    {
        render_task_controls_menu(
            frame,
            centered_rect(72, 18, area),
            task_controls,
            &loaded.project,
            colors,
        );
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
    let project_prompt = if data.app.preferences().remembered_project.is_some() {
        "L LOAD PROJECT\nREMEMBERED READY"
    } else {
        "L LOAD PROJECT"
    };
    standby(frame, right[1], " PROJECT ", project_prompt, colors);
    render_news(frame, body[1], data.news, data.news_selection, colors);

    let reduced_motion = data.reduced_motion;
    let led_char = pulse_led_char(reduced_motion);
    let led_style = pulse_led_style(Color::Green, reduced_motion);
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(format!(" {led_char} "), led_style),
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
        Span::styled("l", bold(colors.accent)),
        Span::raw(" LOAD   "),
        Span::styled("q", bold(colors.accent)),
        Span::raw(" QUIT"),
    ]))
    .alignment(Alignment::Center)
    .block(bordered(colors.secondary));
    frame.render_widget(footer, rows[2]);

    if let Some(warning) = data.app.warning() {
        let glyph = safe_glyph("\u{26A0}", "!", data.reduced_motion);
        let width = area.width.saturating_sub(4);
        frame.render_widget(
            Paragraph::new(format!("{glyph} {warning}")).style(Style::default().fg(Color::Yellow)),
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

fn render_task_view(frame: &mut Frame, area: Rect, data: TaskViewData<'_>, colors: Palette) {
    let rows = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(20),
        Constraint::Length(3),
    ])
    .split(area);
    let project_name = data.loaded.project.project.as_deref().unwrap_or("PROJECT");
    let weather_summary = compact_weather(data.weather);
    let filter_summary = data.controls.filter_summary();
    let health_line = if let Some(monitor) = data.monitor {
        let (led_color, label) = match monitor.health() {
            ProjectHealth::Healthy => (Color::Green, "HEALTHY"),
            ProjectHealth::Loading => (Color::Rgb(255, 165, 0), "LOADING"),
            ProjectHealth::Error { message } => (Color::Red, message.as_str()),
        };
        let time_str = monitor
            .last_update()
            .map(|t| {
                let elapsed = Instant::now().duration_since(t);
                let elapsed = chrono::Duration::from_std(elapsed).unwrap_or_default();
                let dt = Local::now() - elapsed;
                dt.format("%H:%M").to_string()
            })
            .unwrap_or_else(|| "--:--".to_string());
        let prefix = if matches!(monitor.health(), ProjectHealth::Error { .. }) {
            "Error"
        } else {
            "Updated"
        };
        Line::from(vec![
            Span::styled("● ", Style::default().fg(led_color)),
            Span::styled(label, Style::default().fg(led_color)),
            Span::raw(format!("  {prefix} {time_str}")),
        ])
    } else {
        Line::from("")
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" ◈ TASK VIEW ", bold(colors.primary)),
                Span::styled(
                    format!("// {project_name}"),
                    Style::default().fg(colors.secondary),
                ),
                Span::raw("   "),
                Span::styled(
                    Local::now().format("%H:%M  %a %d %b").to_string(),
                    bold(colors.accent),
                ),
                Span::raw("   "),
                Span::styled(weather_summary, Style::default().fg(colors.dim)),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(" GROUP: {} ", data.controls.group_by().label()),
                    Style::default().fg(colors.secondary),
                ),
                Span::styled(
                    format!(
                        "// FILTERS: {}",
                        if filter_summary.is_empty() {
                            "None"
                        } else {
                            &filter_summary
                        }
                    ),
                    Style::default().fg(colors.accent),
                ),
            ]),
            health_line,
        ])
        .block(bordered(colors.secondary)),
        rows[0],
    );

    let body = Layout::vertical([Constraint::Length(14), Constraint::Min(6)]).split(rows[1]);
    let board = TaskBoard::new(&data.loaded.project);
    let projection = TaskProjection::new(
        &data.loaded.project,
        data.controls.filters(),
        data.controls.group_by(),
    );
    let selected_group = projection
        .groups()
        .iter()
        .position(|group| {
            group
                .tasks
                .iter()
                .any(|task| data.selection.is_selected(task))
        })
        .unwrap_or(0);
    let first_group = selected_group.saturating_sub(4);
    let displayed_groups = projection
        .groups()
        .iter()
        .skip(first_group)
        .take(5)
        .collect::<Vec<_>>();
    if displayed_groups.is_empty() {
        frame.render_widget(
            Paragraph::new("No Tasks match active filters")
                .alignment(Alignment::Center)
                .block(bordered(colors.secondary).title(" TASKS ")),
            body[0],
        );
    }
    let columns = Layout::horizontal(vec![
        Constraint::Ratio(
            1,
            displayed_groups.len().max(1) as u32
        );
        displayed_groups.len()
    ])
    .split(body[0]);
    let changed_ids: Vec<&str> = data
        .monitor
        .map(|m| m.changed_task_ids().iter().map(String::as_str).collect())
        .unwrap_or_default();
    for (column, group) in columns.iter().zip(displayed_groups) {
        let capacity = column.height.saturating_sub(2) as usize;
        let selected_position = group
            .tasks
            .iter()
            .position(|task| data.selection.is_selected(task));
        let offset = selected_position
            .map(|position| position.saturating_sub(capacity.saturating_sub(1)))
            .unwrap_or(0);
        let mut lines = Vec::new();
        for task in group.tasks.iter().skip(offset).take(capacity) {
            let current = board.is_current(task);
            let selected = data.selection.is_selected(task);
            let changed = changed_ids.contains(&task.id.as_str());
            let marker = if current {
                safe_glyph("\u{25C6}", "+", data.reduced_motion)
            } else if selected {
                safe_glyph("\u{25B6}", ">", data.reduced_motion)
            } else {
                safe_glyph("\u{00B7}", ".", data.reduced_motion)
            };
            let mut text = format!("{marker} {}", task.id);
            if let Some(title) = &task.title {
                text.push_str(&format!(" {title}"));
            }
            lines.push(Line::from(Span::styled(
                text,
                if current || selected {
                    bold(colors.accent)
                } else if changed {
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.dim)
                },
            )));
        }
        frame.render_widget(
            Paragraph::new(lines).block(bordered(colors.secondary).title(format!(
                " {} ({}) ",
                group.label,
                group.tasks.len()
            ))),
            *column,
        );
    }

    let visible_ids = projection.visible_task_ids();
    let notice = projection.current_task_notice();
    let selected = data
        .selection
        .selected(&data.loaded.project)
        .filter(|task| visible_ids.contains(&task.id.as_str()))
        .or_else(|| projection.ordered_tasks().first().copied());
    render_task_details(
        frame,
        body[1],
        &data.loaded.project,
        &board,
        selected,
        notice,
        colors,
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("d", bold(colors.accent)),
            Span::raw(" DASHBOARD   "),
            Span::styled("t", bold(colors.accent)),
            Span::raw(" TASKS   "),
            Span::styled("r", bold(colors.accent)),
            Span::raw(" REFRESH   "),
            Span::styled("l", bold(colors.accent)),
            Span::raw(" LOAD   "),
            Span::styled(
                safe_glyph("\u{2191}/\u{2193}", "^/v", data.reduced_motion),
                bold(colors.accent),
            ),
            Span::raw(" SELECT   "),
            Span::styled("f/g", bold(colors.accent)),
            Span::raw(" FILTER/GROUP   "),
            Span::styled("?", bold(colors.accent)),
            Span::raw(" HELP   "),
            Span::styled("q", bold(colors.accent)),
            Span::raw(" QUIT"),
        ]))
        .alignment(Alignment::Center)
        .block(bordered(colors.secondary)),
        rows[2],
    );

    if let Some(warning) = data.app.warning() {
        let glyph = safe_glyph("\u{26A0}", "!", data.reduced_motion);
        frame.render_widget(
            Paragraph::new(format!("{glyph} {warning}")).style(Style::default().fg(Color::Yellow)),
            Rect::new(
                area.x + 2,
                area.y + area.height - 5,
                area.width.saturating_sub(4),
                1,
            ),
        );
    }
}

fn render_task_controls_menu(
    frame: &mut Frame,
    area: Rect,
    controls: &TaskControls,
    project: &crate::progress::Project,
    colors: Palette,
) {
    frame.render_widget(Clear, area);
    let lines = match controls.menu() {
        TaskMenu::Closed => return,
        TaskMenu::Filters => {
            let tabs = [
                crate::progress::FilterCategory::Status,
                crate::progress::FilterCategory::Milestone,
                crate::progress::FilterCategory::Phase,
            ]
            .into_iter()
            .map(|category| {
                if category == controls.filter_category() {
                    format!("[{}]", category.label())
                } else {
                    category.label().to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("  ");
            let mut lines = vec![
                Line::from(Span::styled("FILTER TASKS", bold(colors.primary))),
                Line::from(tabs),
                Line::from(""),
            ];
            lines.extend(
                controls
                    .filter_options(project)
                    .into_iter()
                    .enumerate()
                    .map(|(index, option)| {
                        let cursor = if index == controls.filter_option_index() {
                            ">"
                        } else {
                            " "
                        };
                        let check = if option.selected { "x" } else { " " };
                        Line::from(format!("{cursor} [{check}] {}", option.label))
                    }),
            );
            lines.push(Line::from(""));
            lines.push(Line::from(
                "←/→ category  ↑/↓ option  Enter toggle  Esc clear",
            ));
            lines
        }
        TaskMenu::Grouping => {
            let mut lines = vec![
                Line::from(Span::styled("GROUP TASKS", bold(colors.primary))),
                Line::from(""),
            ];
            lines.extend(crate::progress::GroupBy::ALL.into_iter().enumerate().map(
                |(index, group)| {
                    let cursor = if index == controls.grouping_index() {
                        ">"
                    } else {
                        " "
                    };
                    Line::from(format!("{cursor} {}", group.label()))
                },
            ));
            lines.push(Line::from(""));
            lines.push(Line::from("↑/↓ select  Enter apply  Esc clear filters"));
            lines
        }
    };
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(bordered(colors.accent).title(" TASK VIEW CONTROLS ")),
        area,
    );
}

fn compact_weather(weather: &WeatherView) -> String {
    match weather {
        WeatherView::Loading => "● Weather loading".into(),
        WeatherView::Ready { city, report } => {
            format!("● {city}  {}°C  {}", report.temperature_c, report.condition)
        }
        WeatherView::Error {
            last_good: Some((city, report)),
            ..
        } => {
            format!("● STALE {city}  {}°C", report.temperature_c)
        }
        WeatherView::Error { .. } => "● Weather error".into(),
    }
}

fn render_task_details<'a>(
    frame: &mut Frame,
    area: Rect,
    project: &'a crate::progress::Project,
    board: &TaskBoard<'a>,
    task: Option<&'a ProjectTask>,
    notice: Option<&str>,
    colors: Palette,
) {
    let Some(task) = task else {
        frame.render_widget(
            Paragraph::new(notice.unwrap_or("No Tasks supplied by Progress File"))
                .alignment(Alignment::Center)
                .block(bordered(colors.secondary).title(" TASK DETAILS ")),
            area,
        );
        return;
    };
    let mut lines = Vec::new();
    if let Some(notice) = notice {
        lines.push(Line::from(Span::styled(
            notice,
            Style::default().fg(Color::Yellow),
        )));
    }
    lines.push(task_detail_heading(task, board.is_current(task), colors));
    push_optional_detail(&mut lines, "Milestone", task.milestone.as_deref());
    push_optional_detail(&mut lines, "Phase", task.phase.as_deref());
    push_optional_detail(&mut lines, "Mode", task.mode.as_deref());
    if !task.depends_on.is_empty() {
        let dependencies = board
            .dependencies(task)
            .into_iter()
            .map(|dependency| match dependency.status {
                Some(status) => format!("{} [{}]", dependency.id, status.label()),
                None => dependency.id.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(format!("Depends: {dependencies}")));
    }
    push_optional_detail(&mut lines, "Notes", task.notes.as_deref());
    push_optional_detail(&mut lines, "Verified", task.verified_at.as_deref());
    if let Some(verification) = &task.verification {
        lines.push(Line::from(format!("Verification: {verification}")));
    }
    push_optional_detail(&mut lines, "Verify", project.verify_command.as_deref());
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(bordered(colors.accent).title(" CURRENT TASK DETAILS ")),
        area,
    );
}

fn task_detail_heading<'a>(task: &'a ProjectTask, current: bool, colors: Palette) -> Line<'a> {
    let title = task.title.as_deref().unwrap_or("");
    Line::from(Span::styled(
        format!(
            "{}  {}  {title}",
            if current { "CURRENT" } else { "SELECTED" },
            task.id
        ),
        bold(colors.accent),
    ))
}

fn push_optional_detail<'a>(lines: &mut Vec<Line<'a>>, label: &str, value: Option<&'a str>) {
    if let Some(value) = value {
        lines.push(Line::from(format!("{label}: {value}")));
    }
}

fn render_load_menu(frame: &mut Frame, area: Rect, menu: &LoadProjectMenu, colors: Palette) {
    frame.render_widget(Clear, area);
    let lines = match menu.stage() {
        LoadMenuStage::Closed => return,
        LoadMenuStage::Choose => {
            let new_marker = if menu.remembered_selected() { " " } else { ">" };
            let mut lines = vec![
                Line::from(Span::styled("LOAD PROGRESS FILE", bold(colors.primary))),
                Line::from(""),
                Line::from(format!("{new_marker} Load new path")),
            ];
            if let Some(path) = menu.remembered_project() {
                let marker = if menu.remembered_selected() { ">" } else { " " };
                lines.push(Line::from(format!(
                    "{marker} Remembered: {}",
                    path.display()
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("↑/↓ select  Enter confirm  Esc cancel"));
            lines
        }
        LoadMenuStage::PathInput => vec![
            Line::from(Span::styled(
                "ENTER PROGRESS FILE PATH",
                bold(colors.primary),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{}█", menu.path_draft()),
                Style::default().fg(colors.accent),
            )),
            Line::from(""),
            Line::from("Relative or absolute path  //  Enter load  Esc cancel"),
        ],
    };
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(bordered(colors.accent).title(" LOAD PROJECT ")),
        area,
    );
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
        key_line("r", "Refresh Progress File (Task View)", colors),
        key_line("l", "Load Progress File", colors),
        key_line("d / t", "Dashboard / Task View", colors),
        key_line("Task arrows", "Select Task details", colors),
        key_line("f / g", "Filter / group Tasks", colors),
        key_line("Esc", "Clear Task filters", colors),
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

fn pulse_led_style(base_color: Color, reduced_motion: bool) -> Style {
    if reduced_motion {
        return Style::default().fg(base_color);
    }
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64;
    let phase = (millis / 2000.0 * std::f64::consts::TAU).sin();
    let intensity = 0.35 + 0.65 * phase.abs();
    match base_color {
        Color::Rgb(r, g, b) => Style::default().fg(Color::Rgb(
            ((r as f64) * intensity) as u8,
            ((g as f64) * intensity) as u8,
            ((b as f64) * intensity) as u8,
        )),
        _ => {
            let multiplier = (intensity * 255.0) as u8;
            Style::default().fg(Color::Rgb(multiplier, multiplier, multiplier))
        }
    }
}

fn pulse_led_char(reduced_motion: bool) -> &'static str {
    if reduced_motion {
        return "\u{25CF}";
    }
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        % 1000;
    if millis < 800 { "\u{25CF}" } else { "\u{25CB}" }
}

fn safe_glyph<'a>(unicode: &'a str, ascii: &'a str, reduced_motion: bool) -> &'a str {
    if reduced_motion { ascii } else { unicode }
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
