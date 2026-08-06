use termxboard::{
    AppAction, AppState, KeyCommand, SettingsField,
    preferences::{Preferences, Theme},
};

#[test]
fn first_run_accepts_turku_and_requests_persistence() {
    let mut app = AppState::new(Preferences::default(), true, None);

    assert!(app.is_first_run());
    assert_eq!(app.city_draft(), "Turku, Finland");
    assert_eq!(app.handle_key(KeyCommand::Enter), AppAction::Continue);

    assert!(!app.is_first_run());
    assert_eq!(
        app.take_preferences_changed()
            .expect("changed preferences")
            .city,
        "Turku, Finland"
    );
}

#[test]
fn first_run_can_replace_the_proposed_city() {
    let mut app = AppState::new(Preferences::default(), true, None);

    for character in "Helsinki, Finland".chars() {
        app.handle_key(KeyCommand::Character(character));
    }
    app.handle_key(KeyCommand::Enter);

    assert_eq!(app.preferences().city, "Helsinki, Finland");
}

#[test]
fn settings_select_theme_and_reduced_motion() {
    let mut app = AppState::new(Preferences::default(), false, None);
    app.handle_key(KeyCommand::Character('s'));
    assert!(app.is_settings_visible());

    app.handle_key(KeyCommand::Down);
    assert_eq!(app.settings_field(), SettingsField::Theme);
    app.handle_key(KeyCommand::Right);
    assert_eq!(app.preferences().theme, Theme::Cyberpunk);

    app.handle_key(KeyCommand::Down);
    assert_eq!(app.settings_field(), SettingsField::ReducedMotion);
    app.handle_key(KeyCommand::Enter);
    assert!(app.preferences().reduced_motion);
}
