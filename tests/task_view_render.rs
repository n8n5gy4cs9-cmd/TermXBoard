use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use chrono::{Local, TimeZone};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use termxboard::{
    AppState,
    news::{NewsSelection, NewsView},
    preferences::{Preferences, Theme, VisualMode},
    progress::{
        LoadProjectMenu, LoadedProject, Project, ProjectMonitor, REFRESH_INTERVAL, TaskControls,
        TaskSelection, load_progress_file,
    },
    telemetry::TelemetryView,
    ui,
    visuals::{ColorSupport, SemanticTone, theme_color},
    weather::WeatherView,
};

fn loaded_project() -> LoadedProject {
    let project: Project = serde_json::from_str(r#"{
      "project":"Nodus","currentTask":"T-WIP","verifyCommand":"pnpm verify",
      "tasks":[
        {"id":"T-DONE","title":"Foundation","status":"done","dependsOn":[],"verifiedAt":"2026-08-06T09:00:00Z"},
        {"id":"T-TODO","title":"Backlog","status":"todo","dependsOn":[]},
        {"id":"T-REVIEW","title":"Review","status":"awaiting-review","dependsOn":[]},
        {"id":"T-WIP","title":"Dashboard","status":"in-progress","milestone":"M1","phase":"P1","mode":"think-high","dependsOn":["T-DONE"],"notes":"Supplied note","verification":{"tests":"pass"}},
        {"id":"T-BLOCKED","title":"Blocked","status":"blocked","dependsOn":["T-TODO"]}
      ]
    }"#).expect("project fixture");
    LoadedProject {
        path: PathBuf::from("/tmp/progress.json"),
        project,
    }
}

fn rendered(app: &AppState, project: Option<&LoadedProject>, menu: &LoadProjectMenu) -> String {
    let selection = project
        .map(|loaded| TaskSelection::for_project(&loaded.project))
        .unwrap_or_default();
    rendered_with(app, project, menu, &selection, &TaskControls::default())
}

fn rendered_with(
    app: &AppState,
    project: Option<&LoadedProject>,
    menu: &LoadProjectMenu,
    task_selection: &TaskSelection,
    task_controls: &TaskControls,
) -> String {
    rendered_with_size(app, project, menu, task_selection, task_controls, 110, 32)
}

fn rendered_with_size(
    app: &AppState,
    project: Option<&LoadedProject>,
    menu: &LoadProjectMenu,
    task_selection: &TaskSelection,
    task_controls: &TaskControls,
    width: u16,
    height: u16,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| {
            ui::render_application(
                frame,
                ui::ApplicationData {
                    app,
                    telemetry: &TelemetryView::loading(),
                    weather: &WeatherView::loading(),
                    news: &NewsView::loading(),
                    news_selection: &NewsSelection::default(),
                    active_project: project,
                    load_menu: menu,
                    task_selection,
                    task_controls,
                    project_monitor: None,
                    cpu_history: &[],
                    memory_history: &[],
                },
            )
        })
        .expect("render");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn rendered_at(
    app: &AppState,
    project: &LoadedProject,
    selection: &TaskSelection,
    monitor: Option<&ProjectMonitor>,
    now: Instant,
) -> String {
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let wall_time = Local.with_ymd_and_hms(2026, 8, 7, 12, 34, 56).unwrap();
    terminal
        .draw(|frame| {
            ui::render_application_at(
                frame,
                ui::ApplicationData {
                    app,
                    telemetry: &TelemetryView::loading(),
                    weather: &WeatherView::loading(),
                    news: &NewsView::loading(),
                    news_selection: &NewsSelection::default(),
                    active_project: Some(project),
                    load_menu: &LoadProjectMenu::default(),
                    task_selection: selection,
                    task_controls: &TaskControls::default(),
                    project_monitor: monitor,
                    cpu_history: &[],
                    memory_history: &[],
                },
                ColorSupport::TrueColor,
                now,
                wall_time,
            )
        })
        .expect("render");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn task_view_uses_supplied_seconds_and_independent_wip_current_selection_layers() {
    let project = loaded_project();
    let mut app = AppState::default();
    app.show_task_view();
    let selection = TaskSelection::for_project(&project.project);
    let output = rendered_at(&app, &project, &selection, None, Instant::now());

    assert!(output.contains("12:34:56"));
    assert!(output.contains("┃▌> ▶ T-WIP"));
    assert!(output.contains("[CURRENT] [WIP]"));
}

#[test]
fn every_explicit_wip_keeps_its_badge_under_each_grouping() {
    let project: Project = serde_json::from_str(
        r#"{
      "project":"WIP layers",
      "tasks":[
        {"id":"T-1","title":"One","status":"in-progress","milestone":"M1","phase":"P1"},
        {"id":"T-2","title":"Two","status":"in-progress","milestone":"M1","phase":"P1"}
      ]
    }"#,
    )
    .unwrap();
    let loaded = LoadedProject {
        path: PathBuf::from("/tmp/progress.json"),
        project,
    };
    let mut app = AppState::default();
    app.show_task_view();
    let selection = TaskSelection::for_project(&loaded.project);

    for down_presses in 0..=2 {
        let mut controls = TaskControls::default();
        if down_presses > 0 {
            controls.handle_key(termxboard::KeyCommand::Character('g'), &loaded.project);
            for _ in 0..down_presses {
                controls.handle_key(termxboard::KeyCommand::Down, &loaded.project);
            }
            controls.handle_key(termxboard::KeyCommand::Enter, &loaded.project);
        }
        let output = rendered_with(
            &app,
            Some(&loaded),
            &LoadProjectMenu::default(),
            &selection,
            &controls,
        );
        assert_eq!(output.matches("[WIP]").count(), 2);
    }
}

#[test]
fn task_view_error_keeps_last_good_task_and_shows_clean_and_full_errors() {
    let directory = std::env::temp_dir().join(format!(
        "termxboard-activity-error-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("progress.json");
    fs::write(&path, r#"{"project":"Visual","currentTask":"T-1","tasks":[{"id":"T-1","title":"Still visible","status":"in-progress"}]}"#).unwrap();
    let loaded = load_progress_file(&path, &directory).unwrap();
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);
    fs::write(&path, "{\n broken").unwrap();
    monitor.refresh_now(start + Duration::from_secs(1));
    let active = monitor.active().unwrap();
    let selection = TaskSelection::for_project(&active.project);
    let mut app = AppState::default();
    app.show_task_view();

    let output = rendered_at(
        &app,
        active,
        &selection,
        Some(&monitor),
        start + Duration::from_secs(1),
    );

    assert!(output.contains("ERROR"));
    assert!(output.contains("Still visible"));
    assert!(output.contains("ERROR: malformed JSON"));
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn task_view_selects_non_current_tasks_and_shows_their_dependencies() {
    let project = loaded_project();
    let mut app = AppState::default();
    app.show_task_view();
    let mut selection = TaskSelection::for_project(&project.project);
    selection.previous(&project.project);

    let output = rendered_with(
        &app,
        Some(&project),
        &LoadProjectMenu::default(),
        &selection,
        &TaskControls::default(),
    );

    assert!(output.contains("SELECTED  T-BLOCKED"));
    assert!(output.contains("T-TODO [Undone]"));
}

#[test]
fn task_view_reports_when_visible_projection_hides_current_task() {
    let project = loaded_project();
    let mut app = AppState::default();
    app.show_task_view();
    let selection = TaskSelection::for_project(&project.project);
    let mut controls = TaskControls::default();
    controls.handle_key(termxboard::KeyCommand::Character('f'), &project.project);
    controls.handle_key(termxboard::KeyCommand::Enter, &project.project);
    controls.handle_key(termxboard::KeyCommand::Character('f'), &project.project);

    let output = rendered_with(
        &app,
        Some(&project),
        &LoadProjectMenu::default(),
        &selection,
        &controls,
    );

    assert!(
        output.contains("Current Task T-WIP hidden by active filters"),
        "{output}"
    );
}

#[test]
fn task_view_shows_active_filters_and_grouping() {
    let project = loaded_project();
    let mut app = AppState::default();
    app.show_task_view();
    let selection = TaskSelection::for_project(&project.project);
    let mut controls = TaskControls::default();
    controls.handle_key(termxboard::KeyCommand::Character('f'), &project.project);
    controls.handle_key(termxboard::KeyCommand::Enter, &project.project);
    controls.handle_key(termxboard::KeyCommand::Character('f'), &project.project);
    controls.handle_key(termxboard::KeyCommand::Character('g'), &project.project);
    controls.handle_key(termxboard::KeyCommand::Down, &project.project);
    controls.handle_key(termxboard::KeyCommand::Enter, &project.project);

    let output = rendered_with(
        &app,
        Some(&project),
        &LoadProjectMenu::default(),
        &selection,
        &controls,
    );

    assert!(output.contains("GROUP: Milestone"));
    assert!(output.contains("FILTERS: Status=Blocked"));
    assert!(output.contains("Unspecified (1)"));
}

#[test]
fn task_view_is_status_first_with_compact_header_details_and_no_news() {
    let project = loaded_project();
    let mut app = AppState::default();
    app.show_task_view();

    let output = rendered(&app, Some(&project), &LoadProjectMenu::default());

    for expected in [
        "TASK VIEW",
        "Blocked",
        "WIP",
        "User Review",
        "Undone",
        "Done",
        "T-WIP",
        "Dashboard",
        "CURRENT",
        "T-DONE [Done]",
        "pnpm verify",
        "Supplied note",
    ] {
        assert!(output.contains(expected), "missing {expected:?}");
    }
    assert!(!output.contains("HACKER NEWS"));
    assert!(!output.contains("SYSTEM TELEMETRY"));
}

#[test]
fn dashboard_load_menu_offers_new_and_remembered_paths() {
    let app = AppState::default();
    let mut menu = LoadProjectMenu::default();
    menu.open(Some(PathBuf::from("/tmp/nodus/progress.json")));

    let output = rendered(&app, None, &menu);

    assert!(output.contains("LOAD PROGRESS FILE"));
    assert!(output.contains("Load new path"));
    assert!(output.contains("/tmp/nodus/progress.json"));
}

#[test]
fn help_explains_filter_clear_and_close_controls() {
    let mut app = AppState::default();
    app.handle_key(termxboard::KeyCommand::Character('h'));

    let output = rendered(&app, None, &LoadProjectMenu::default());

    assert!(output.contains("c  Clear filters (filter panel)"));
    assert!(output.contains("Esc  Close Task menu"));
}

#[test]
fn ascii_visual_mode_renders_fallback_glyphs_independently_of_motion() {
    let prefs = Preferences {
        reduced_motion: false,
        visual_mode: VisualMode::Ascii,
        ..Preferences::default()
    };
    let mut app = AppState::new(prefs, false, None);
    app.show_task_view();
    let project = loaded_project();
    let selection = TaskSelection::for_project(&project.project);

    let output = rendered_with(
        &app,
        Some(&project),
        &LoadProjectMenu::default(),
        &selection,
        &TaskControls::default(),
    );

    assert!(
        !output.contains("\u{25C6}"),
        "should not contain ◆ in reduced motion mode"
    );
    assert!(
        !output.contains("\u{25B6}"),
        "should not contain ▶ in reduced motion mode"
    );
    assert!(
        !output.contains("\u{00B7}"),
        "should not contain · in reduced motion mode"
    );
    assert!(
        !output.contains("\u{2191}"),
        "should not contain ↑ in footer"
    );
    assert!(!output.contains("\u{26A0}"), "should not contain ⚠");
    assert!(
        output.contains("^/v SELECT"),
        "should have ASCII arrows in footer"
    );
    assert!(output.is_ascii(), "ASCII mode leaked a non-ASCII glyph");
}

#[test]
fn reduced_motion_keeps_unicode_status_language_when_unicode_is_selected() {
    let prefs = Preferences {
        reduced_motion: true,
        visual_mode: VisualMode::Unicode,
        ..Preferences::default()
    };
    let mut app = AppState::new(prefs, false, None);
    app.show_task_view();
    let project = loaded_project();

    let output = rendered(&app, Some(&project), &LoadProjectMenu::default());

    assert!(output.contains("┃▌> ▶ T-WIP"));
    assert!(output.contains("[CURRENT] [WIP]"));
    assert!(output.contains("✓ T-DONE"));
}

#[test]
fn task_rows_use_semantic_icons_for_every_status() {
    let project = loaded_project();
    let mut app = AppState::default();
    app.show_task_view();
    let output = rendered(&app, Some(&project), &LoadProjectMenu::default());

    for expected in [
        "◆ T-BLOCKED",
        "┃▌> ▶ T-WIP",
        "◉ T-REVIEW",
        "○ T-TODO",
        "✓ T-DONE",
    ] {
        assert!(output.contains(expected), "missing {expected:?}");
    }
    assert!(output.contains("[CURRENT]"));
}

#[test]
fn every_theme_preserves_dashboard_identity_and_controls() {
    for theme in Theme::ALL {
        let app = AppState::new(
            Preferences {
                theme,
                ..Preferences::default()
            },
            false,
            None,
        );
        let output = rendered(&app, None, &LoadProjectMenu::default());
        for expected in [
            "TERMXBOARD",
            "RETRO COMMAND DECK",
            "SYSTEM TELEMETRY",
            "WEATHER",
            "LOAD",
            "QUIT",
        ] {
            assert!(
                output.contains(expected),
                "{} missing {expected:?}",
                theme.label()
            );
        }

        let mut task_app = AppState::new(
            Preferences {
                theme,
                ..Preferences::default()
            },
            false,
            None,
        );
        task_app.show_task_view();
        let project = loaded_project();
        let task_output = rendered(&task_app, Some(&project), &LoadProjectMenu::default());
        for expected in ["Blocked", "WIP", "User Review", "Undone", "Done", "CURRENT"] {
            assert!(
                task_output.contains(expected),
                "{} Task View missing {expected:?}",
                theme.label()
            );
        }
    }
}

#[test]
fn nerd_font_mode_uses_enhanced_status_icons() {
    let mut app = AppState::new(
        Preferences {
            visual_mode: VisualMode::NerdFont,
            ..Preferences::default()
        },
        false,
        None,
    );
    app.show_task_view();
    let project = loaded_project();
    let output = rendered(&app, Some(&project), &LoadProjectMenu::default());
    assert!(output.contains("󰅖 T-BLOCKED"));
    assert!(output.contains("┃▌> 󰐊 T-WIP"));
}

#[test]
fn larger_terminal_keeps_the_shared_layout_and_reveals_task_titles() {
    let mut app = AppState::default();
    app.show_task_view();
    let project = loaded_project();
    let selection = TaskSelection::for_project(&project.project);
    let output = rendered_with_size(
        &app,
        Some(&project),
        &LoadProjectMenu::default(),
        &selection,
        &TaskControls::default(),
        150,
        42,
    );
    assert!(output.contains("Dashboard"));
    assert!(!output.contains("TERMINAL GEOMETRY INSUFFICIENT"));
}

#[test]
fn ansi_ascii_render_keeps_dashboard_readable_without_rgb_or_unicode() {
    let app = AppState::new(
        Preferences {
            visual_mode: VisualMode::Ascii,
            ..Preferences::default()
        },
        false,
        None,
    );
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| {
            ui::render_application_with_color_support(
                frame,
                ui::ApplicationData {
                    app: &app,
                    telemetry: &TelemetryView::loading(),
                    weather: &WeatherView::loading(),
                    news: &NewsView::loading(),
                    news_selection: &NewsSelection::default(),
                    active_project: None,
                    load_menu: &LoadProjectMenu::default(),
                    task_selection: &TaskSelection::default(),
                    task_controls: &TaskControls::default(),
                    project_monitor: None,
                    cpu_history: &[],
                    memory_history: &[],
                },
                ColorSupport::Ansi,
            )
        })
        .expect("render");
    let buffer = terminal.backend().buffer();
    let output = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.is_ascii());
    for expected in ["TERMXBOARD", "SYSTEM TELEMETRY", "WEATHER", "LOAD", "QUIT"] {
        assert!(output.contains(expected));
    }
    assert!(
        buffer
            .content()
            .iter()
            .all(|cell| !matches!(cell.fg, Color::Rgb(..)))
    );
}

#[test]
fn recovered_changed_current_task_keeps_recovery_selection_and_glow_layers() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("termxboard-visual-{nonce}"));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("progress.json");
    fs::write(&path, r#"{"project":"Visual","currentTask":"T-1","tasks":[{"id":"T-1","title":"Before","status":"in-progress"}]}"#).unwrap();
    let loaded = load_progress_file(&path, &directory).unwrap();
    let start = Instant::now() - Duration::from_secs(3);
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);
    fs::write(&path, "{").unwrap();
    monitor.refresh_now(start + Duration::from_secs(1));
    fs::write(&path, r#"{"project":"Visual","currentTask":"T-1","tasks":[{"id":"T-1","title":"After","status":"in-progress"}]}"#).unwrap();
    monitor.refresh_now(start + Duration::from_secs(2));

    let active = monitor.active().unwrap();
    let selection = TaskSelection::for_project(&active.project);
    let prefs = Preferences {
        reduced_motion: true,
        ..Preferences::default()
    };
    let mut app = AppState::new(prefs, false, None);
    app.show_task_view();
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            ui::render_application_with_color_support(
                frame,
                ui::ApplicationData {
                    app: &app,
                    telemetry: &TelemetryView::loading(),
                    weather: &WeatherView::loading(),
                    news: &NewsView::loading(),
                    news_selection: &NewsSelection::default(),
                    active_project: Some(active),
                    load_menu: &LoadProjectMenu::default(),
                    task_selection: &selection,
                    task_controls: &TaskControls::default(),
                    project_monitor: Some(&monitor),
                    cpu_history: &[],
                    memory_history: &[],
                },
                ColorSupport::TrueColor,
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let output = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("RECOVERED"));
    let changed_background = theme_color(
        Theme::SignatureNeon,
        SemanticTone::Accent,
        ColorSupport::TrueColor,
    );
    assert!(
        buffer
            .content()
            .iter()
            .any(|cell| cell.symbol() == "▶" && cell.bg == changed_background)
    );
    let _ = fs::remove_dir_all(directory);
}
