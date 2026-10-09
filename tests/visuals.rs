use ratatui::style::Color;
use termxboard::{
    preferences::{Theme, VisualMode},
    progress::{ProjectActivity, TaskStatus},
    visuals::{
        AnimationFrame, ColorSupport, SemanticTone, StatusVisual, TaskActivityVisual, TaskScanner,
        TemperatureBand, theme_color,
    },
};

#[test]
fn task_status_has_stable_icon_label_and_tone_in_every_glyph_mode() {
    let cases = [
        (TaskStatus::Blocked, "◆", "󰅖", "X", SemanticTone::Danger),
        (TaskStatus::InProgress, "▶", "󰐊", ">", SemanticTone::Info),
        (
            TaskStatus::AwaitingReview,
            "◉",
            "󰄬",
            "?",
            SemanticTone::Review,
        ),
        (TaskStatus::Todo, "○", "󰄱", "o", SemanticTone::Warning),
        (TaskStatus::Done, "✓", "󰄵", "v", SemanticTone::Success),
    ];

    for (status, unicode, nerd, ascii, tone) in cases {
        let visual = StatusVisual::for_status(status);
        assert_eq!(visual.glyph(VisualMode::Unicode), unicode);
        assert_eq!(visual.glyph(VisualMode::NerdFont), nerd);
        assert_eq!(visual.glyph(VisualMode::Ascii), ascii);
        assert_eq!(visual.tone, tone);
        assert_eq!(visual.label, status.label());
    }
}

#[test]
fn task_scanner_bounces_and_reduced_motion_centers_it() {
    assert_eq!(TaskScanner::at(0, false, false).head, 0);
    assert_eq!(TaskScanner::at(21 * 125, false, false).head, 21);
    assert_eq!(TaskScanner::at(22 * 125, false, false).head, 20);
    assert_eq!(
        TaskScanner::at(0, true, false),
        TaskScanner { head: 10, tail: 9 }
    );
    assert_eq!(
        TaskScanner::at(9_999, true, true),
        TaskScanner { head: 10, tail: 9 }
    );
    assert_ne!(
        TaskScanner::at(800, false, true),
        TaskScanner::at(800, false, false)
    );
}

#[test]
fn task_activity_states_select_scanner_and_heartbeat_behavior() {
    let healthy = TaskActivityVisual::for_activity(ProjectActivity::Healthy, false);
    assert_eq!(healthy.scanner_tone, SemanticTone::Accent);
    assert_eq!(healthy.heartbeat_tone, SemanticTone::Success);
    assert_eq!(healthy.heartbeat_period_ms, Some(1200));

    let countdown = TaskActivityVisual::for_activity(ProjectActivity::Countdown(4), false);
    assert_eq!(countdown.scanner_tone, SemanticTone::Warning);
    assert!(countdown.scanner_fast);
    assert_eq!(countdown.heartbeat_period_ms, Some(600));

    let updating = TaskActivityVisual::for_activity(ProjectActivity::Updating, false);
    assert_eq!(updating.heartbeat_period_ms, Some(300));

    let error = TaskActivityVisual::for_activity(ProjectActivity::Error, false);
    assert_eq!(error.scanner_tone, SemanticTone::Danger);
    assert!(error.static_motion);
    assert_eq!(error.heartbeat_period_ms, None);

    let reduced = TaskActivityVisual::for_activity(ProjectActivity::Countdown(4), true);
    assert!(reduced.static_motion);
    assert_eq!(reduced.heartbeat_period_ms, None);
}

#[test]
fn loading_and_changed_task_animation_become_static_under_reduced_motion() {
    let first = AnimationFrame::at(0, false);
    let second = AnimationFrame::at(180, false);
    assert_ne!(
        first.loading_scan(VisualMode::Unicode),
        second.loading_scan(VisualMode::Unicode)
    );
    assert_ne!(first.changed_tone(), second.changed_tone());

    let reduced_first = AnimationFrame::at(0, true);
    let reduced_second = AnimationFrame::at(180, true);
    assert_eq!(reduced_first.loading_scan(VisualMode::Ascii), "[=   ]");
    assert_eq!(reduced_first.loading_scan(VisualMode::Unicode), "▰▱▱▱▱");
    assert_eq!(
        reduced_first.loading_scan(VisualMode::Unicode),
        reduced_second.loading_scan(VisualMode::Unicode)
    );
    assert_eq!(reduced_first.changed_tone(), reduced_second.changed_tone());
}

#[test]
fn temperature_bands_follow_the_agreed_boundaries() {
    assert_eq!(TemperatureBand::from_celsius(-20), TemperatureBand::Cold);
    assert_eq!(TemperatureBand::from_celsius(5), TemperatureBand::Cold);
    assert_eq!(TemperatureBand::from_celsius(6), TemperatureBand::Mild);
    assert_eq!(TemperatureBand::from_celsius(20), TemperatureBand::Mild);
    assert_eq!(TemperatureBand::from_celsius(21), TemperatureBand::Warm);
    assert_eq!(TemperatureBand::from_celsius(27), TemperatureBand::Warm);
    assert_eq!(TemperatureBand::from_celsius(28), TemperatureBand::Hot);
}

#[test]
fn every_theme_has_true_color_and_ansi_semantic_fallbacks() {
    for theme in Theme::ALL {
        for tone in SemanticTone::ALL {
            assert!(matches!(
                theme_color(theme, tone, ColorSupport::TrueColor),
                Color::Rgb(..)
            ));
            assert!(!matches!(
                theme_color(theme, tone, ColorSupport::Ansi),
                Color::Rgb(..)
            ));
        }
    }
}
