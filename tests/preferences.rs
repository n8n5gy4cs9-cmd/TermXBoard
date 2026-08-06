use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use termxboard::{
    AppState, KeyCommand, SettingsField,
    preferences::{Preferences, PreferencesStore, SaveOutcome, Theme},
};

fn temp_config(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("termxboard-{name}-{nonce}/config.json"))
}

#[test]
fn first_run_uses_turku_and_signature_theme() {
    let path = temp_config("defaults");
    let loaded = PreferencesStore::new(path).load();

    assert!(loaded.is_first_run);
    assert_eq!(loaded.preferences.city, "Turku, Finland");
    assert_eq!(loaded.preferences.theme, Theme::SignatureNeon);
    assert!(!loaded.preferences.reduced_motion);
}

#[test]
fn preferences_survive_restart() {
    let path = temp_config("restart");
    let store = PreferencesStore::new(path.clone());
    let preferences = Preferences {
        city: "Helsinki, Finland".into(),
        theme: Theme::Nord,
        reduced_motion: true,
    };

    assert!(store.save(&preferences).is_saved());
    assert_eq!(store.load().preferences, preferences);

    let _ = fs::remove_dir_all(path.parent().expect("parent"));
}

#[test]
fn invalid_config_falls_back_with_warning() {
    let path = temp_config("invalid");
    fs::create_dir_all(path.parent().expect("parent")).expect("temp directory");
    fs::write(&path, "not json").expect("invalid config fixture");

    let loaded = PreferencesStore::new(path.clone()).load();

    assert_eq!(loaded.preferences, Preferences::default());
    assert!(!loaded.is_first_run);
    assert!(loaded.warning.expect("warning").contains("Invalid config"));
    let _ = fs::remove_dir_all(path.parent().expect("parent"));
}

#[test]
fn unwritable_config_becomes_session_only() {
    let path = temp_config("unwritable");
    fs::create_dir_all(&path).expect("directory where file should be");

    let store = PreferencesStore::new(path.clone());
    let mut app = AppState::new(Preferences::default(), false, None);
    app.handle_key(KeyCommand::Character('s'));
    app.handle_key(KeyCommand::Down);
    assert_eq!(app.settings_field(), SettingsField::Theme);
    app.handle_key(KeyCommand::Right);
    let changed = app.take_preferences_changed().expect("session change");
    let outcome = store.save(&changed);

    let SaveOutcome::SessionOnly(warning) = outcome else {
        panic!("unwritable config should use session-only settings");
    };
    assert!(warning.contains("session-only"));
    assert!(warning.contains("could not write"));
    assert_eq!(app.preferences().theme, Theme::Cyberpunk);
    let _ = fs::remove_dir_all(path.parent().expect("parent"));
}
