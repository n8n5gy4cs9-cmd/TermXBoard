use std::{
    collections::VecDeque,
    thread,
    time::{Duration, Instant},
};

use termxboard::telemetry::{
    Capacity, NetworkActivity, TelemetryMonitor, TelemetrySnapshot, TelemetrySource, TelemetryView,
};

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
        cpu_percent: Some(cpu_percent),
        memory: Some(Capacity {
            used_bytes: 1,
            total_bytes: 2,
        }),
        battery_percent: None,
        disk: Some(Capacity {
            used_bytes: 1,
            total_bytes: 2,
        }),
        network: Some(NetworkActivity {
            received_bytes: 0,
            transmitted_bytes: 0,
        }),
        uptime: Some(Duration::ZERO),
    }
}

fn wait_for_cpu(monitor: &mut TelemetryMonitor, now: Instant, expected: f32) {
    for _ in 0..100 {
        monitor.tick(now);
        if matches!(monitor.view(), TelemetryView::Ready(value) if value.cpu_percent == Some(expected))
        {
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
    assert!(
        matches!(monitor.view(), TelemetryView::Ready(value) if value.cpu_percent == Some(10.0))
    );
    wait_for_cpu(&mut monitor, now + Duration::from_secs(3), 20.0);
}

#[test]
fn monitor_keeps_bounded_cpu_and_memory_history_for_sparklines() {
    let now = Instant::now();
    let source = ScriptedSource {
        results: (0..40).map(|value| Ok(snapshot(value as f32))).collect(),
        delay: Duration::ZERO,
    };
    let mut monitor = TelemetryMonitor::new(source, Duration::from_secs(1), now);

    for value in 0..40 {
        wait_for_cpu(&mut monitor, now + Duration::from_secs(value), value as f32);
    }

    assert_eq!(monitor.cpu_history().len(), 30);
    assert_eq!(monitor.cpu_history().first(), Some(&10));
    assert_eq!(monitor.cpu_history().last(), Some(&39));
    assert_eq!(monitor.memory_history(), &[50; 30]);
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

#[test]
fn transient_error_retains_last_good_readings_as_stale() {
    let now = Instant::now();
    let source = ScriptedSource {
        results: [Ok(snapshot(10.0)), Err("sensor failure".into())].into(),
        delay: Duration::ZERO,
    };
    let mut monitor = TelemetryMonitor::new(source, Duration::from_secs(3), now);
    wait_for_cpu(&mut monitor, now, 10.0);

    for _ in 0..100 {
        monitor.tick(now + Duration::from_secs(3));
        if monitor.view().error_message() == Some("sensor failure") {
            let cards = monitor.view().cards();
            assert!(cards[0].value.contains("10% (stale)"));
            assert!(
                cards
                    .iter()
                    .all(|card| card.led == termxboard::telemetry::LedState::Red)
            );
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("telemetry error was not visible");
}
