use std::{
    collections::VecDeque,
    thread,
    time::{Duration, Instant},
};

use termxboard::telemetry::{TelemetryMonitor, TelemetrySnapshot, TelemetrySource, TelemetryView};

struct ScriptedSource {
    results: VecDeque<Result<TelemetrySnapshot, String>>,
    delay: Duration,
}

impl TelemetrySource for ScriptedSource {
    fn collect(&mut self) -> Result<TelemetrySnapshot, String> {
        thread::sleep(self.delay);
        self.results.pop_front().expect("scripted result")
    }
}

fn snapshot(cpu_percent: f32) -> TelemetrySnapshot {
    TelemetrySnapshot {
        cpu_percent,
        memory_used_bytes: 1,
        memory_total_bytes: 2,
        battery_percent: None,
        disk_used_bytes: 1,
        disk_total_bytes: 2,
        network_received_bytes: 0,
        network_transmitted_bytes: 0,
        uptime: Duration::ZERO,
    }
}

fn wait_for_cpu(monitor: &mut TelemetryMonitor, now: Instant, expected: f32) {
    for _ in 0..100 {
        monitor.tick(now);
        if matches!(monitor.view(), TelemetryView::Ready(value) if value.cpu_percent == expected) {
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("telemetry did not become ready");
}

#[test]
fn collection_never_blocks_the_ui_tick() {
    let now = Instant::now();
    let source = ScriptedSource {
        results: [Ok(snapshot(10.0))].into(),
        delay: Duration::from_millis(100),
    };
    let mut monitor = TelemetryMonitor::new(source, Duration::from_secs(3), now);

    let started = Instant::now();
    monitor.tick(now);

    assert!(started.elapsed() < Duration::from_millis(20));
    assert_eq!(monitor.view(), &TelemetryView::Loading);
}

#[test]
fn collection_refreshes_every_three_seconds() {
    let now = Instant::now();
    let source = ScriptedSource {
        results: [Ok(snapshot(10.0)), Ok(snapshot(20.0))].into(),
        delay: Duration::ZERO,
    };
    let mut monitor = TelemetryMonitor::new(source, Duration::from_secs(3), now);

    wait_for_cpu(&mut monitor, now, 10.0);
    monitor.tick(now + Duration::from_secs(2));
    assert!(matches!(monitor.view(), TelemetryView::Ready(value) if value.cpu_percent == 10.0));
    wait_for_cpu(&mut monitor, now + Duration::from_secs(3), 20.0);
}

#[test]
fn collection_errors_are_visible() {
    let now = Instant::now();
    let source = ScriptedSource {
        results: [Err("sensor failure".into())].into(),
        delay: Duration::ZERO,
    };
    let mut monitor = TelemetryMonitor::new(source, Duration::from_secs(3), now);

    for _ in 0..100 {
        monitor.tick(now);
        if monitor.view().error_message() == Some("sensor failure") {
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("telemetry error was not visible");
}
