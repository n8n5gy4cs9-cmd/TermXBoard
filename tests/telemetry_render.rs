use std::time::Duration;

use ratatui::{Terminal, backend::TestBackend};
use termxboard::{
    AppState,
    preferences::Theme,
    telemetry::{TelemetrySnapshot, TelemetryView},
    ui,
    visuals::{ColorSupport, SemanticTone, theme_color},
    weather::WeatherView,
};

fn rendered(view: &TelemetryView) -> String {
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| ui::render(frame, &AppState::default(), view, &WeatherView::loading()))
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
fn dashboard_renders_loading_telemetry() {
    let output = rendered(&TelemetryView::loading());

    assert!(output.contains("SYSTEM TELEMETRY"));
    assert!(output.contains("CPU"));
    assert!(output.contains("Loading…"));
    assert!(output.contains('▰'));
}

#[test]
fn dashboard_renders_loading_leds_in_orange() {
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| {
            ui::render(
                frame,
                &AppState::default(),
                &TelemetryView::loading(),
                &WeatherView::loading(),
            )
        })
        .expect("render");

    let orange_leds = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .filter(|cell| {
            cell.symbol() == "●"
                && [
                    theme_color(
                        Theme::SignatureNeon,
                        SemanticTone::Warning,
                        ColorSupport::TrueColor,
                    ),
                    theme_color(
                        Theme::SignatureNeon,
                        SemanticTone::Warning,
                        ColorSupport::Ansi,
                    ),
                ]
                .contains(&cell.fg)
        })
        .count();
    assert!(orange_leds >= 10);
}

#[test]
fn dashboard_renders_unavailable_and_error_states() {
    let sample = TelemetrySnapshot {
        cpu_percent: Some(1.0),
        memory: None,
        battery_percent: None,
        disk: None,
        network: None,
        uptime: Some(Duration::ZERO),
    };

    assert!(rendered(&TelemetryView::ready(sample)).contains("Unavailable"));
    assert!(rendered(&TelemetryView::error("sensor failure")).contains("Error"));
}
