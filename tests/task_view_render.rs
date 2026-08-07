use std::path::PathBuf;

use ratatui::{Terminal, backend::TestBackend};
use termxboard::{
    AppState,
    news::{NewsSelection, NewsView},
    preferences::Preferences,
    progress::{LoadProjectMenu, LoadedProject, Project, TaskControls, TaskSelection},
    telemetry::TelemetryView,
    ui,
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
    let backend = TestBackend::new(110, 32);
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
fn reduced_motion_renders_ascii_fallback_glyphs() {
    let prefs = Preferences {
        reduced_motion: true,
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
}
