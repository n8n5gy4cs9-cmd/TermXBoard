use std::time::Duration;

use termxboard::telemetry::{
    LedState, MacTelemetrySource, TelemetrySnapshot, TelemetrySource, TelemetryView,
};

fn snapshot() -> TelemetrySnapshot {
    TelemetrySnapshot {
        cpu_percent: 42.4,
        memory_used_bytes: 8 * 1024_u64.pow(3),
        memory_total_bytes: 16 * 1024_u64.pow(3),
        battery_percent: Some(81.6),
        disk_used_bytes: 250 * 1024_u64.pow(3),
        disk_total_bytes: 500 * 1024_u64.pow(3),
        network_received_bytes: 3 * 1024_u64.pow(2),
        network_transmitted_bytes: 512 * 1024,
        uptime: Duration::from_secs(2 * 86_400 + 3 * 3_600 + 4 * 60),
    }
}

#[test]
fn ready_telemetry_formats_honest_compact_cards() {
    let cards = TelemetryView::ready(snapshot()).cards();

    assert_eq!(cards[0].label, "CPU");
    assert_eq!(cards[0].value, "42%");
    assert_eq!(cards[0].led, LedState::Green);
    assert_eq!(cards[1].value, "8.0 / 16.0 GiB");
    assert_eq!(cards[2].value, "82%");
    assert_eq!(cards[3].value, "250.0 / 500.0 GiB");
    assert_eq!(cards[4].value, "↓ 3.0 MiB  ↑ 512.0 KiB");
    assert_eq!(cards[5].value, "2d 3h 4m");
}

#[test]
fn absent_battery_is_explicitly_unavailable() {
    let mut sample = snapshot();
    sample.battery_percent = None;

    let battery = &TelemetryView::ready(sample).cards()[2];

    assert_eq!(battery.value, "Unavailable");
    assert_eq!(battery.led, LedState::Red);
}

#[test]
fn loading_and_error_leds_are_consistent() {
    assert!(
        TelemetryView::loading()
            .cards()
            .iter()
            .all(|card| card.led == LedState::Orange && card.value == "Loading…")
    );
    assert!(
        TelemetryView::error("collection failed")
            .cards()
            .iter()
            .all(|card| card.led == LedState::Red && card.value == "Error")
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_source_reports_real_core_metrics() {
    let sample = MacTelemetrySource::new()
        .collect()
        .expect("macOS telemetry");

    assert!(sample.cpu_percent.is_finite());
    assert!(sample.memory_total_bytes > 0);
    assert!(sample.disk_total_bytes > 0);
}
