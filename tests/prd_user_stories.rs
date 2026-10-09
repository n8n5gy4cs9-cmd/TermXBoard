use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use termxboard::progress::{TaskStatus, load_progress_file};

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("termxboard-user-stories-{name}-{nonce}"))
}

fn user_story_prd() -> String {
    r#"{
      "$schema": "docs/prd.schema.json",
      "schemaVersion": 1,
      "project": "buddy bits",
      "branchName": "",
      "description": "Offline party game implementation status.",
      "schemaSource": "https://example.com/prd.json.example",
      "updatedAt": "2026-09-26T08:24:51.368Z",
      "currentTask": "T04",
      "userStories": [
        {
          "id": "S00",
          "title": "Reproducible developer setup",
          "description": "Set up a fresh project.",
          "acceptanceCriteria": ["Fresh starter copy regenerates paths."],
          "priority": 1,
          "passes": true,
          "notes": "Fresh-folder setup verified.",
          "status": "done",
          "dependsOn": [],
          "updatedAt": "2026-09-26T06:22:20.489Z",
          "evidence": [
            {"criterion": 0, "result": "pass", "detail": "verified", "artifact": "docs/SETUP_VERIFICATION.md"}
          ]
        },
        {
          "id": "T04",
          "title": "Player and session flow",
          "description": "Complete human setup.",
          "acceptanceCriteria": ["Player counts 1 through 8 obey rules."],
          "priority": 5,
          "passes": false,
          "notes": "Implementing next milestone.",
          "status": "in_progress",
          "dependsOn": ["S00"],
          "updatedAt": "2026-09-26T08:24:51.368Z",
          "evidence": []
        }
      ]
    }"#
    .to_string()
}

#[test]
fn user_story_prd_loads_and_maps_stories_to_tasks() {
    let directory = temp_dir("load");
    fs::create_dir_all(&directory).expect("temp directory");
    fs::write(directory.join("prd.json"), user_story_prd()).expect("fixture");

    let loaded = load_progress_file("prd.json", &directory).expect("user-story PRD");

    assert_eq!(loaded.project.project.as_deref(), Some("buddy bits"));
    assert_eq!(loaded.project.current_task.as_deref(), Some("T04"));
    assert_eq!(loaded.project.updated_at.as_deref(), Some("2026-09-26T08:24:51.368Z"));

    let tasks = &loaded.project.tasks;
    assert_eq!(tasks.len(), 2);

    let setup = &tasks[0];
    assert_eq!(setup.id, "S00");
    assert_eq!(setup.status, TaskStatus::Done);
    assert_eq!(setup.depends_on, Vec::<String>::new());
    assert_eq!(setup.files_changed, vec!["docs/SETUP_VERIFICATION.md"]);
    assert_eq!(
        setup.verified_at.as_deref(),
        Some("2026-09-26T06:22:20.489Z")
    );
    let notes = setup.notes.as_deref().expect("setup notes");
    assert!(notes.contains("Fresh-folder setup verified."));
    assert!(notes.contains("Acceptance: Fresh starter copy regenerates paths."));
    assert!(notes.contains("Priority: 1"));
    assert!(setup.verification.is_some());

    let flow = &tasks[1];
    assert_eq!(flow.id, "T04");
    assert_eq!(flow.status, TaskStatus::InProgress);
    assert_eq!(flow.depends_on, vec!["S00".to_string()]);
    assert_eq!(flow.verified_at, None);
    assert!(flow.files_changed.is_empty());
    assert!(flow.verification.is_none());

    let _ = fs::remove_dir_all(directory);
}
