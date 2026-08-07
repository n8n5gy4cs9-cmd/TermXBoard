use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use termxboard::progress::{ProjectSession, TaskStatus, load_progress_file};

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("termxboard-progress-{name}-{nonce}"))
}

fn valid_json(project: &str) -> String {
    format!(
        r#"{{
      "project": "{project}",
      "currentTask": "T-2",
      "tasks": [
        {{"id":"T-1","title":"Foundation","status":"done","dependsOn":[],"unknownTaskField":true}},
        {{"id":"T-2","title":"Dashboard","status":"in-progress","dependsOn":["T-1"],"verifiedAt":"2026-08-06T09:00:00Z"}}
      ],
      "unknownRootField": {{"safe":true}}
    }}"#
    )
}

#[test]
fn absolute_and_relative_progress_paths_load_and_tolerate_unknown_fields() {
    let directory = temp_dir("paths");
    fs::create_dir_all(&directory).expect("temp directory");
    let path = directory.join("progress.json");
    fs::write(&path, valid_json("Nodus")).expect("fixture");
    fs::write(directory.join("TASKS.md"), "this file must never be parsed")
        .expect("unrelated file");

    let relative = load_progress_file("progress.json", &directory).expect("relative path");
    let absolute = load_progress_file(&path, PathBuf::from("/")).expect("absolute path");

    assert!(relative.path.is_absolute());
    assert_eq!(relative.path, absolute.path);
    assert_eq!(relative.project.project.as_deref(), Some("Nodus"));
    assert_eq!(relative.project.tasks[1].status, TaskStatus::InProgress);
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn missing_file_malformed_json_and_missing_required_task_data_are_clear_errors() {
    let directory = temp_dir("errors");
    fs::create_dir_all(&directory).expect("temp directory");

    let missing = load_progress_file("missing.json", &directory).expect_err("missing file");
    assert!(missing.contains("could not read"));

    fs::write(directory.join("progress.json"), "not json").expect("fixture");
    let malformed = load_progress_file("progress.json", &directory).expect_err("malformed");
    assert!(malformed.contains("malformed JSON"));

    fs::write(
        directory.join("progress.json"),
        r#"{"tasks":[{"id":"T-1"}]}"#,
    )
    .expect("fixture");
    let invalid = load_progress_file("progress.json", &directory).expect_err("missing status");
    assert!(invalid.contains("missing required data"));
    assert!(invalid.contains("missing field `status`"));

    fs::write(directory.join("progress.json"), r#"{"project":"No tasks"}"#).expect("fixture");
    let missing_tasks = load_progress_file("progress.json", &directory).expect_err("missing tasks");
    assert!(missing_tasks.contains("missing required data"));
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn session_remembers_only_successful_absolute_path_and_preserves_valid_project_on_error() {
    let directory = temp_dir("session");
    fs::create_dir_all(&directory).expect("temp directory");
    fs::write(directory.join("one.json"), valid_json("One")).expect("fixture");
    fs::write(directory.join("broken.json"), "{").expect("fixture");
    let mut session = ProjectSession::new(None);

    session.load("one.json", &directory).expect("first project");
    let remembered = session
        .remembered_project()
        .expect("remembered")
        .to_path_buf();
    let error = session
        .load("broken.json", &directory)
        .expect_err("invalid replacement");

    assert!(remembered.is_absolute());
    assert_eq!(session.remembered_project(), Some(remembered.as_path()));
    assert_eq!(
        session
            .active()
            .expect("preserved")
            .project
            .project
            .as_deref(),
        Some("One")
    );
    assert!(error.contains("malformed JSON"));
    let _ = fs::remove_dir_all(directory);
}
