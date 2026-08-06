use std::{
    collections::VecDeque,
    thread,
    time::{Duration, Instant, SystemTime},
};

use termxboard::weather::{WeatherClient, WeatherMonitor, WeatherReport, WeatherView};

struct ScriptedClient {
    results: VecDeque<Result<WeatherReport, String>>,
    delay: Duration,
}

impl WeatherClient for ScriptedClient {
    fn fetch(&mut self, _city: &str) -> Result<WeatherReport, String> {
        thread::sleep(self.delay);
        self.results.pop_front().expect("scripted result")
    }
}

fn report(temperature_c: i16) -> WeatherReport {
    WeatherReport {
        condition: "Clear".into(),
        temperature_c,
        feels_like_c: temperature_c,
        humidity_percent: 50,
        wind_kmh: 8,
        updated_at: SystemTime::UNIX_EPOCH,
    }
}

fn wait_for_temperature(monitor: &mut WeatherMonitor, now: Instant, expected: i16) {
    for _ in 0..100 {
        monitor.tick(now, "Turku, Finland");
        if matches!(monitor.view(), WeatherView::Ready { report, .. } if report.temperature_c == expected)
        {
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("weather did not become ready");
}

#[test]
fn fetching_weather_never_blocks_dashboard_tick() {
    let now = Instant::now();
    let client = ScriptedClient {
        results: [Ok(report(4))].into(),
        delay: Duration::from_millis(100),
    };
    let mut monitor = WeatherMonitor::new(client, Duration::from_secs(900), now);

    let started = Instant::now();
    monitor.tick(now, "Turku, Finland");

    assert!(started.elapsed() < Duration::from_millis(20));
    assert_eq!(monitor.view(), &WeatherView::Loading);
}

#[test]
fn weather_refreshes_at_fifteen_minutes_and_on_manual_request() {
    let now = Instant::now();
    let client = ScriptedClient {
        results: [Ok(report(1)), Ok(report(2)), Ok(report(3))].into(),
        delay: Duration::ZERO,
    };
    let mut monitor = WeatherMonitor::new(client, Duration::from_secs(900), now);

    wait_for_temperature(&mut monitor, now, 1);
    monitor.tick(now + Duration::from_secs(899), "Turku, Finland");
    assert!(
        matches!(monitor.view(), WeatherView::Ready { report, .. } if report.temperature_c == 1)
    );
    wait_for_temperature(&mut monitor, now + Duration::from_secs(900), 2);

    monitor.refresh_now(now + Duration::from_secs(901));
    wait_for_temperature(&mut monitor, now + Duration::from_secs(901), 3);
}

#[test]
fn failure_retains_last_successful_session_result() {
    let now = Instant::now();
    let client = ScriptedClient {
        results: [Ok(report(7)), Err("network offline".into())].into(),
        delay: Duration::ZERO,
    };
    let mut monitor = WeatherMonitor::new(client, Duration::from_secs(900), now);
    wait_for_temperature(&mut monitor, now, 7);

    for _ in 0..100 {
        monitor.tick(now + Duration::from_secs(900), "Turku, Finland");
        if let WeatherView::Error {
            message,
            last_good: Some((city, cached)),
        } = monitor.view()
        {
            assert_eq!(message, "network offline");
            assert_eq!(city, "Turku, Finland");
            assert_eq!(cached.temperature_c, 7);
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("weather error was not visible");
}
