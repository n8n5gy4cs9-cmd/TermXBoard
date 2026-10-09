use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use termxboard::progress::{
    GroupBy, ProjectActivity, ProjectHealth, ProjectMonitor, REFRESH_INTERVAL, TaskFilters,
    TaskProjection, TaskSelection, load_progress_file,
};

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("termxboard-refresh-{name}-{nonce}"))
}

#[test]
fn refresh_presentation_counts_down_only_during_final_ten_seconds() {
    let directory = temp_dir("countdown");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));
    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    assert_eq!(
        monitor.activity_at(start + Duration::from_secs(49)),
        ProjectActivity::Healthy
    );
    assert_eq!(
        monitor.activity_at(start + Duration::from_millis(50_001)),
        ProjectActivity::Countdown(10)
    );
    assert_eq!(
        monitor.activity_at(start + Duration::from_millis(59_999)),
        ProjectActivity::Countdown(1)
    );

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn successful_refresh_shows_updating_for_600ms_without_drifting_schedule() {
    let directory = temp_dir("updating-feedback");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));
    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);
    let refresh_started = start + Duration::from_secs(5);

    monitor.refresh_now(refresh_started);

    assert_eq!(
        monitor.activity_at(refresh_started),
        ProjectActivity::Updating
    );
    assert_eq!(
        monitor.activity_at(refresh_started + Duration::from_millis(599)),
        ProjectActivity::Updating
    );
    assert_eq!(
        monitor.activity_at(refresh_started + Duration::from_millis(600)),
        ProjectActivity::Healthy
    );
    assert_eq!(
        monitor.activity_at(refresh_started + Duration::from_secs(50)),
        ProjectActivity::Countdown(10)
    );

    let _ = fs::remove_dir_all(directory);
}

fn project_json(project: &str, tasks: &str) -> String {
    format!(
        r#"{{
      "project": "{project}",
      "currentTask": "T-2",
      "tasks": [{tasks}]
    }}"#
    )
}

fn simple_tasks() -> String {
    r#"{"id":"T-1","title":"Foundation","status":"done"},
       {"id":"T-2","title":"Dashboard","status":"in-progress"}"#
        .to_string()
}

fn write_fixture(path: &PathBuf, content: &str) {
    fs::write(path, content).expect("fixture write");
}

// ── scheduling ───────────────────────────────────────────────────────────

#[test]
fn no_refresh_before_interval_and_refresh_at_interval() {
    let directory = temp_dir("schedule");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    let just_before = start + REFRESH_INTERVAL - Duration::from_millis(1);
    monitor.tick(just_before, true);
    assert_eq!(monitor.health(), &ProjectHealth::Healthy);
    assert_eq!(monitor.last_update(), Some(start));
    assert!(monitor.changed_task_ids().is_empty());

    let at_interval = start + REFRESH_INTERVAL;
    monitor.tick(at_interval, true);

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn no_scheduled_refresh_on_dashboard_then_resumes_in_task_view() {
    let directory = temp_dir("dashboard-pause");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    let past_interval = start + REFRESH_INTERVAL + Duration::from_secs(10);
    monitor.tick(past_interval, false);
    assert_eq!(monitor.health(), &ProjectHealth::Healthy);
    assert_eq!(monitor.last_update(), Some(start));

    monitor.tick(past_interval + Duration::from_millis(1), true);
    assert_eq!(
        monitor.last_update(),
        Some(past_interval + Duration::from_millis(1))
    );
    assert_eq!(
        monitor.activity_at(past_interval + Duration::from_millis(1)),
        ProjectActivity::Updating
    );

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn r_key_triggers_immediate_refresh() {
    let directory = temp_dir("immediate");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    write_fixture(
        &path,
        &project_json(
            "Updated",
            r#"{"id":"T-1","title":"Foundation","status":"done"},
               {"id":"T-2","title":"Dashboard v2","status":"in-progress"}"#,
        ),
    );

    let before_interval = start + Duration::from_secs(5);
    monitor.refresh_now(before_interval);

    assert_eq!(monitor.health(), &ProjectHealth::Healthy);
    assert!(monitor.last_update().is_some());
    assert_eq!(
        monitor.active().unwrap().project.project.as_deref(),
        Some("Updated")
    );
    assert!(monitor.changed_task_ids().contains(&"T-2".to_string()));
    assert!(monitor.highlight_until().is_some());

    let _ = fs::remove_dir_all(directory);
}

// ── change detection ─────────────────────────────────────────────────────

#[test]
fn valid_changed_file_updates_project_and_reports_exact_changed_ids() {
    let directory = temp_dir("change-detect");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    write_fixture(
        &path,
        &project_json(
            "Updated",
            r#"{"id":"T-1","title":"Foundation","status":"done"},
               {"id":"T-2","title":"Dashboard v2","status":"in-progress","notes":"changed"},
               {"id":"T-3","title":"New task","status":"todo"}"#,
        ),
    );

    let after = start + Duration::from_secs(10);
    monitor.refresh_now(after);

    let project = &monitor.active().unwrap().project;
    assert_eq!(project.project.as_deref(), Some("Updated"));
    assert_eq!(project.tasks.len(), 3);

    let mut changed = monitor.changed_task_ids().to_vec();
    changed.sort();
    assert_eq!(changed, vec!["T-2".to_string(), "T-3".to_string()]);
}

#[test]
fn added_removed_and_modified_tasks_are_each_detected() {
    let directory = temp_dir("detect-all");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");

    let old_json = r#"{"tasks":[
        {"id":"T-1","title":"One","status":"todo"},
        {"id":"T-2","title":"Two","status":"done"}
    ]}"#;
    write_fixture(&path, old_json);

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    let new_json = r#"{"tasks":[
        {"id":"T-2","title":"Two changed","status":"todo"},
        {"id":"T-3","title":"Three","status":"blocked"}
    ]}"#;
    write_fixture(&path, new_json);

    monitor.refresh_now(start + Duration::from_secs(1));

    let mut changed = monitor.changed_task_ids().to_vec();
    changed.sort();
    assert_eq!(changed, vec!["T-1", "T-2", "T-3"]);

    let _ = fs::remove_dir_all(directory);
}

// ── error handling ────────────────────────────────────────────────────────

#[test]
fn missing_or_malformed_file_retains_last_good_state_and_exposes_error() {
    let directory = temp_dir("errors");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Good", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    let malformed_json = "{not valid";
    write_fixture(&path, malformed_json);
    monitor.refresh_now(start + Duration::from_secs(10));

    assert!(matches!(monitor.health(), ProjectHealth::Error { .. }));
    assert!(monitor.error_message().unwrap().contains("malformed JSON"));
    assert_eq!(
        monitor.active().unwrap().project.project.as_deref(),
        Some("Good")
    );
    assert_eq!(monitor.changed_task_ids().len(), 0);

    drop(monitor);
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn deleted_file_is_handled_as_error_and_last_good_is_preserved() {
    let directory = temp_dir("deleted");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Good", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    fs::remove_file(&path).expect("delete");
    monitor.refresh_now(start + Duration::from_secs(1));

    assert!(matches!(monitor.health(), ProjectHealth::Error { .. }));
    assert!(monitor.error_message().is_some());
    assert_eq!(
        monitor.active().unwrap().project.project.as_deref(),
        Some("Good")
    );

    drop(monitor);
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn later_valid_file_clears_error_and_recovers() {
    let directory = temp_dir("recover");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Initial", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    write_fixture(&path, "corrupted json {{{");
    monitor.refresh_now(start + Duration::from_secs(1));
    assert!(matches!(monitor.health(), ProjectHealth::Error { .. }));

    write_fixture(
        &path,
        &project_json(
            "Recovered",
            r#"{"id":"T-1","title":"Foundation","status":"done"},
               {"id":"T-2","title":"Dashboard","status":"done"}"#,
        ),
    );
    monitor.refresh_now(start + Duration::from_secs(2));

    assert_eq!(monitor.health(), &ProjectHealth::Recovered);
    assert_eq!(
        monitor.active().unwrap().project.project.as_deref(),
        Some("Recovered")
    );
    assert!(monitor.error_message().is_none());

    let _ = fs::remove_dir_all(directory);
}

// ── highlight expiry ──────────────────────────────────────────────────────

#[test]
fn highlight_expires_after_duration() {
    let directory = temp_dir("highlight-expiry");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    write_fixture(
        &path,
        &project_json(
            "Changed",
            r#"{"id":"T-1","title":"Foundation v2","status":"done"},
               {"id":"T-2","title":"Dashboard","status":"in-progress"}"#,
        ),
    );

    let after = start + Duration::from_secs(1);
    monitor.refresh_now(after);
    assert!(!monitor.changed_task_ids().is_empty());
    assert!(monitor.highlight_until().is_some());

    let highlight_until = monitor.highlight_until().unwrap();
    monitor.tick(highlight_until + Duration::from_secs(1), true);
    assert!(monitor.changed_task_ids().is_empty());
    assert!(monitor.highlight_until().is_none());

    let _ = fs::remove_dir_all(directory);
}

// ── selection / filter / grouping survival ────────────────────────────────

#[test]
fn selection_preserved_when_task_still_exists_after_refresh() {
    let directory = temp_dir("selection-survive");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    let mut selection = TaskSelection::for_project(&loaded.project);
    assert_eq!(selection.selected(&loaded.project).unwrap().id, "T-2");

    write_fixture(
        &path,
        &project_json(
            "Changed",
            r#"{"id":"T-1","title":"Foundation v2","status":"done"},
               {"id":"T-2","title":"Dashboard v2","status":"in-progress","notes":"added"}"#,
        ),
    );
    monitor.refresh_now(start + Duration::from_secs(1));

    let project = &monitor.active().unwrap().project;
    let filters = TaskFilters::default();
    let projection = TaskProjection::new(project, &filters, GroupBy::Status);
    selection.ensure_visible(&projection.ordered_tasks());

    assert_eq!(selection.selected(project).unwrap().id, "T-2");

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn selection_falls_back_to_first_visible_when_previous_selection_removed() {
    let directory = temp_dir("selection-fallback");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    let mut selection = TaskSelection::for_project(&loaded.project);
    assert_eq!(selection.selected(&loaded.project).unwrap().id, "T-2");

    write_fixture(
        &path,
        &project_json(
            "T2 removed",
            r#"{"id":"T-1","title":"Foundation","status":"done"}"#,
        ),
    );
    monitor.refresh_now(start + Duration::from_secs(1));

    let project = &monitor.active().unwrap().project;
    let filters = TaskFilters::default();
    let projection = TaskProjection::new(project, &filters, GroupBy::Status);
    selection.ensure_visible(&projection.ordered_tasks());

    assert_eq!(selection.selected(project).unwrap().id, "T-1");

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn filters_and_grouping_are_not_cleared_after_refresh() {
    let directory = temp_dir("filter-survive");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    let mut filters = TaskFilters::default();
    filters.toggle_status(termxboard::progress::TaskStatus::InProgress);
    let group_by = GroupBy::Milestone;

    write_fixture(
        &path,
        &project_json(
            "Changed",
            r#"{"id":"T-1","title":"Foundation","status":"done"},
               {"id":"T-2","title":"Dashboard v2","status":"in-progress","milestone":"M1"}"#,
        ),
    );
    monitor.refresh_now(start + Duration::from_secs(1));

    let project = &monitor.active().unwrap().project;
    let projection = termxboard::progress::TaskProjection::new(project, &filters, group_by);
    assert_eq!(projection.visible_task_ids().len(), 1);
    assert_eq!(projection.visible_task_ids()[0], "T-2");

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn current_task_change_honored_after_refresh() {
    let directory = temp_dir("current-task-change");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    let mut selection = TaskSelection::for_project(&loaded.project);
    assert_eq!(selection.selected(&loaded.project).unwrap().id, "T-2");

    let new_json = r#"{"project":"Changed","currentTask":"T-1","tasks":[{"id":"T-1","title":"Foundation","status":"done"},{"id":"T-2","title":"Dashboard","status":"in-progress"}]}"#.to_string();
    write_fixture(&path, &new_json);

    monitor.refresh_now(start + Duration::from_secs(1));
    let project = &monitor.active().unwrap().project;

    let filters = TaskFilters::default();
    let projection = TaskProjection::new(project, &filters, GroupBy::Status);
    selection = TaskSelection::for_project(project);
    selection.ensure_visible(&projection.ordered_tasks());

    assert_eq!(selection.selected(project).unwrap().id, "T-1");

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn load_is_total_and_accepts_added_and_removed_tasks() {
    let directory = temp_dir("total-load");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    write_fixture(
        &path,
        &project_json(
            "Rebuilt",
            r#"{"id":"T-A","title":"Alpha","status":"blocked"},
               {"id":"T-B","title":"Beta","status":"todo"}"#,
        ),
    );

    monitor.refresh_now(start + Duration::from_secs(1));
    let tasks = &monitor.active().unwrap().project.tasks;
    assert_eq!(tasks.len(), 2);
    let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["T-A", "T-B"]);

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn monitor_health_transitions_through_loading_on_refresh() {
    let directory = temp_dir("health-loading");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Test", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    assert_eq!(monitor.health(), &ProjectHealth::Healthy);

    write_fixture(
        &path,
        &project_json(
            "Changed",
            r#"{"id":"T-1","title":"Updated","status":"done"},
               {"id":"T-2","title":"Dashboard","status":"in-progress"}"#,
        ),
    );
    monitor.refresh_now(start + Duration::from_secs(1));
    assert_eq!(monitor.health(), &ProjectHealth::Healthy);
    assert!(monitor.last_update().is_some());

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn reset_replaces_project_and_clears_state() {
    let directory = temp_dir("reset");
    fs::create_dir_all(&directory).expect("temp dir");
    let path = directory.join("progress.json");
    write_fixture(&path, &project_json("Original", &simple_tasks()));

    let loaded = load_progress_file(&path, &directory).expect("load");
    let start = Instant::now();
    let mut monitor = ProjectMonitor::new(&loaded, REFRESH_INTERVAL, start);

    write_fixture(
        &path,
        &project_json(
            "Changed",
            r#"{"id":"T-1","title":"Original","status":"done"},
               {"id":"T-2","title":"Changed","status":"done"},
               {"id":"T-3","title":"Extra","status":"todo"}"#,
        ),
    );
    monitor.refresh_now(start + Duration::from_secs(1));
    assert!(!monitor.changed_task_ids().is_empty());

    write_fixture(
        &path,
        &project_json("Reset", r#"{"id":"T-99","title":"Fresh","status":"todo"}"#),
    );
    let reloaded = load_progress_file(&path, &directory).expect("reload");
    monitor.reset(&reloaded, start + Duration::from_secs(5));

    assert_eq!(monitor.health(), &ProjectHealth::Healthy);
    assert!(monitor.changed_task_ids().is_empty());
    assert!(monitor.highlight_until().is_none());
    assert_eq!(
        monitor.active().unwrap().project.project.as_deref(),
        Some("Reset")
    );
    assert_eq!(monitor.active().unwrap().project.tasks.len(), 1);

    let _ = fs::remove_dir_all(directory);
}
