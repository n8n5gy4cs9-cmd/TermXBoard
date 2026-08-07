use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use termxboard::{
    KeyCommand,
    progress::{LoadMenuStage, LoadProjectMenu, ProjectSession},
};

#[test]
fn load_menu_selects_remembered_project_or_accepts_a_new_path() {
    let remembered = PathBuf::from("/tmp/nodus/progress.json");
    let mut menu = LoadProjectMenu::default();
    menu.open(Some(remembered.clone()));

    assert_eq!(menu.stage(), LoadMenuStage::Choose);
    menu.handle_key(KeyCommand::Down);
    let request = menu
        .handle_key(KeyCommand::Enter)
        .expect("remembered request");
    assert_eq!(request.as_path(), remembered);

    menu.open(Some(PathBuf::from("/tmp/other.json")));
    menu.handle_key(KeyCommand::Enter);
    for character in "../project/progress.json".chars() {
        menu.handle_key(KeyCommand::Character(character));
    }
    let request = menu
        .handle_key(KeyCommand::Enter)
        .expect("new path request");
    assert_eq!(request.as_path(), Path::new("../project/progress.json"));
}

#[test]
fn menu_requests_load_relative_and_absolute_paths_and_preserve_state_on_error() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("termxboard-menu-{nonce}"));
    fs::create_dir_all(&directory).expect("temp directory");
    let valid = r#"{"project":"Menu","tasks":[{"id":"T-1","status":"todo"}]}"#;
    fs::write(directory.join("relative.json"), valid).expect("relative fixture");
    fs::write(directory.join("absolute.json"), valid).expect("absolute fixture");
    let mut session = ProjectSession::new(None);

    let relative = menu_path("relative.json");
    session
        .load(relative, &directory)
        .expect("relative menu load");
    assert_eq!(
        session
            .active()
            .expect("relative active")
            .project
            .project
            .as_deref(),
        Some("Menu")
    );

    let absolute_path = directory.join("absolute.json");
    let absolute = menu_path(absolute_path.to_string_lossy().as_ref());
    session
        .load(absolute, &directory)
        .expect("absolute menu load");
    let active_path = session.active().expect("absolute active").path.clone();

    let missing = menu_path("missing.json");
    assert!(session.load(missing, &directory).is_err());
    assert_eq!(session.active().expect("preserved").path, active_path);
    let _ = fs::remove_dir_all(directory);
}

fn menu_path(value: &str) -> PathBuf {
    let mut menu = LoadProjectMenu::default();
    menu.open(None);
    menu.handle_key(KeyCommand::Enter);
    for character in value.chars() {
        menu.handle_key(KeyCommand::Character(character));
    }
    menu.handle_key(KeyCommand::Enter).expect("menu request")
}

#[test]
fn load_menu_escape_is_recoverable_and_q_is_valid_path_input() {
    let mut menu = LoadProjectMenu::default();
    menu.open(None);
    menu.handle_key(KeyCommand::Enter);
    menu.handle_key(KeyCommand::Character('q'));
    assert_eq!(menu.path_draft(), "q");

    menu.handle_key(KeyCommand::Escape);
    assert_eq!(menu.stage(), LoadMenuStage::Closed);
}
