use termxboard::{AppAction, AppState, AppView, KeyCommand};

#[test]
fn q_exits_immediately() {
    let mut app = AppState::default();

    let action = app.handle_key(KeyCommand::Character('q'));

    assert_eq!(action, AppAction::Quit);
}

#[test]
fn successful_project_load_opens_task_view_and_d_t_switch_views() {
    let mut app = AppState::default();
    assert_eq!(app.view(), AppView::Dashboard);
    app.show_task_view();
    assert_eq!(app.view(), AppView::Task);

    app.handle_key(KeyCommand::Character('d'));
    assert_eq!(app.view(), AppView::Dashboard);
    app.show_task_view();
    assert_eq!(app.view(), AppView::Task);
}

#[test]
fn task_view_cannot_open_without_a_loaded_project() {
    let mut app = AppState::default();
    app.handle_key(KeyCommand::Character('t'));
    assert_eq!(app.view(), AppView::Dashboard);
}

#[test]
fn h_toggles_help() {
    let mut app = AppState::default();

    assert!(!app.is_help_visible());
    assert_eq!(
        app.handle_key(KeyCommand::Character('h')),
        AppAction::Continue
    );
    assert!(app.is_help_visible());

    app.handle_key(KeyCommand::Character('h'));
    assert!(!app.is_help_visible());
}
