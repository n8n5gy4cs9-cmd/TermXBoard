use std::time::SystemTime;

use ratatui::{Terminal, backend::TestBackend, style::Color};
use termxboard::{
    AppState,
    preferences::Theme,
    telemetry::TelemetryView,
    ui,
    visuals::{ColorSupport, SemanticTone, theme_color},
    weather::{WeatherReport, WeatherView},
};

fn report() -> WeatherReport {
    WeatherReport {
        condition: "Light snow".into(),
        temperature_c: -2,
        feels_like_c: -6,
        humidity_percent: 82,
        wind_kmh: 19,
        updated_at: SystemTime::UNIX_EPOCH,
    }
}

fn rendered(view: &WeatherView) -> (String, usize, usize) {
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| ui::render(frame, &AppState::default(), &TelemetryView::loading(), view))
        .expect("render");
    let buffer = terminal.backend().buffer();
    let output = buffer.content().iter().map(|cell| cell.symbol()).collect();
    let orange = buffer
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
    let red = buffer
        .content()
        .iter()
        .filter(|cell| cell.symbol() == "●" && cell.fg == Color::Red)
        .count();
    (output, orange, red)
}

#[test]
fn dashboard_renders_loading_weather_with_orange_led() {
    let (output, orange, _) = rendered(&WeatherView::loading());
    assert!(output.contains("WEATHER"));
    assert!(output.contains("Loading"));
    assert!(output.contains('▰'));
    assert!(orange >= 7);
}

#[test]
fn dashboard_renders_current_weather_values() {
    let (output, _, _) = rendered(&WeatherView::ready("Turku, Finland", report()));
    for expected in [
        "Turku, Finland",
        "Light snow",
        "-2°C",
        "Feels -6°C",
        "HUM  82%",
        "WIND 19km/h",
        "Updated",
    ] {
        assert!(output.contains(expected), "missing {expected:?}");
    }
}

#[test]
fn dashboard_renders_cached_weather_as_stale_with_red_led() {
    let view = WeatherView::Error {
        message: "network offline".into(),
        last_good: Some(("Turku, Finland".into(), report())),
    };
    let (output, _, red) = rendered(&view);
    assert!(output.contains("STALE"));
    assert!(output.contains("-2°C"));
    assert!(output.contains("network offline"));
    assert!(red >= 1);
}
