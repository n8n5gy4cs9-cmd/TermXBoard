use termxboard::{AppAction, AppState, KeyCommand};

#[test]
fn q_exits_immediately() {
    let mut app = AppState::default();

    let action = app.handle_key(KeyCommand::Character('q'));

    assert_eq!(action, AppAction::Quit);
}

#[test]
fn question_mark_toggles_help() {
    let mut app = AppState::default();

    assert!(!app.is_help_visible());
    assert_eq!(
        app.handle_key(KeyCommand::Character('?')),
        AppAction::Continue
    );
    assert!(app.is_help_visible());

    app.handle_key(KeyCommand::Character('?'));
    assert!(!app.is_help_visible());
}
