use std::time::Duration;

use ratatui::{Terminal, backend::TestBackend};
use termxboard::{
    AppState,
    telemetry::{TelemetrySnapshot, TelemetryView},
    ui,
};

fn rendered(view: &TelemetryView) -> String {
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| ui::render(frame, &AppState::default(), view))
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
}

#[test]
fn dashboard_renders_unavailable_and_error_states() {
    let sample = TelemetrySnapshot {
        cpu_percent: 1.0,
        memory_used_bytes: 1,
        memory_total_bytes: 2,
        battery_percent: None,
        disk_used_bytes: 1,
        disk_total_bytes: 2,
        network_received_bytes: 0,
        network_transmitted_bytes: 0,
        uptime: Duration::ZERO,
    };

    assert!(rendered(&TelemetryView::ready(sample)).contains("Unavailable"));
    assert!(rendered(&TelemetryView::error("sensor failure")).contains("Error"));
}
