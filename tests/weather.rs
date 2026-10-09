use std::time::{Duration, SystemTime};

use termxboard::weather::{WeatherLed, WeatherView, parse_wttr_response};

const RESPONSE: &str = r#"
{
  "current_condition": [{
    "FeelsLikeC": "-6",
    "humidity": "82",
    "temp_C": "-2",
    "weatherDesc": [{"value": "Light snow"}],
    "windspeedKmph": "19"
  }]
}
"#;

#[test]
fn wttr_response_parses_metric_current_conditions() {
    let updated_at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);

    let report = parse_wttr_response(RESPONSE, updated_at).expect("valid wttr response");

    assert_eq!(report.condition, "Light snow");
    assert_eq!(report.temperature_c, -2);
    assert_eq!(report.feels_like_c, -6);
    assert_eq!(report.humidity_percent, 82);
    assert_eq!(report.wind_kmh, 19);
    assert_eq!(report.updated_at, updated_at);
}

#[test]
fn malformed_or_incomplete_wttr_response_is_rejected() {
    assert!(parse_wttr_response("{}", SystemTime::now()).is_err());
    assert!(parse_wttr_response("not json", SystemTime::now()).is_err());
}

#[test]
fn weather_leds_match_loading_success_and_failure() {
    assert_eq!(WeatherView::loading().led(), WeatherLed::Orange);
    let ready = WeatherView::ready(
        "Turku, Finland",
        parse_wttr_response(RESPONSE, SystemTime::UNIX_EPOCH).unwrap(),
    );
    assert_eq!(ready.led(), WeatherLed::Green);
    assert_eq!(WeatherView::error("offline").led(), WeatherLed::Red);
}
