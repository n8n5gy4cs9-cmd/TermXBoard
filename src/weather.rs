use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime},
};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherReport {
    pub condition: String,
    pub temperature_c: i16,
    pub feels_like_c: i16,
    pub humidity_percent: u8,
    pub wind_kmh: u16,
    pub updated_at: SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherLed {
    Green,
    Orange,
    Red,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeatherView {
    Loading,
    Ready {
        city: String,
        report: WeatherReport,
    },
    Error {
        message: String,
        last_good: Option<(String, WeatherReport)>,
    },
}

impl WeatherView {
    pub fn loading() -> Self {
        Self::Loading
    }

    pub fn ready(city: impl Into<String>, report: WeatherReport) -> Self {
        Self::Ready {
            city: city.into(),
            report,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            last_good: None,
        }
    }

    pub fn led(&self) -> WeatherLed {
        match self {
            Self::Loading => WeatherLed::Orange,
            Self::Ready { .. } => WeatherLed::Green,
            Self::Error { .. } => WeatherLed::Red,
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error { message, .. } => Some(message),
            _ => None,
        }
    }
}

pub trait WeatherClient: Send + 'static {
    fn fetch(&mut self, city: &str) -> Result<WeatherReport, String>;
}

pub struct WttrWeatherClient {
    agent: ureq::Agent,
}

impl WttrWeatherClient {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(8)))
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl Default for WttrWeatherClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherClient for WttrWeatherClient {
    fn fetch(&mut self, city: &str) -> Result<WeatherReport, String> {
        let city = urlencoding::encode(city);
        let url = format!("https://wttr.in/{city}?format=j1");
        let mut response = self
            .agent
            .get(&url)
            .header("User-Agent", "TermXBoard/0.1")
            .call()
            .map_err(|error| format!("weather request failed: {error}"))?;
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|error| format!("weather response failed: {error}"))?;
        parse_wttr_response(&body, SystemTime::now())
    }
}

enum WorkerCommand {
    Fetch(String),
    Stop,
}

struct WorkerResult {
    city: String,
    result: Result<WeatherReport, String>,
}

pub struct WeatherMonitor {
    command_sender: Sender<WorkerCommand>,
    result_receiver: Receiver<WorkerResult>,
    worker: Option<JoinHandle<()>>,
    view: WeatherView,
    interval: Duration,
    next_refresh: Instant,
    in_flight: bool,
    city: String,
    last_good: Option<(String, WeatherReport)>,
}

impl WeatherMonitor {
    pub fn new<C: WeatherClient>(client: C, interval: Duration, now: Instant) -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut client = client;
            while let Ok(command) = command_receiver.recv() {
                match command {
                    WorkerCommand::Fetch(city) => {
                        let result = client.fetch(&city);
                        if result_sender.send(WorkerResult { city, result }).is_err() {
                            break;
                        }
                    }
                    WorkerCommand::Stop => break,
                }
            }
        });
        Self {
            command_sender,
            result_receiver,
            worker: Some(worker),
            view: WeatherView::Loading,
            interval,
            next_refresh: now,
            in_flight: false,
            city: String::new(),
            last_good: None,
        }
    }

    pub fn view(&self) -> &WeatherView {
        &self.view
    }

    pub fn refresh_now(&mut self, now: Instant) {
        self.next_refresh = now;
    }

    pub fn tick(&mut self, now: Instant, city: &str) {
        while let Ok(received) = self.result_receiver.try_recv() {
            self.in_flight = false;
            if received.city != self.city {
                continue;
            }
            self.view = match received.result {
                Ok(report) => {
                    self.last_good = Some((received.city.clone(), report.clone()));
                    WeatherView::ready(received.city, report)
                }
                Err(message) => WeatherView::Error {
                    message,
                    last_good: self.last_good.clone(),
                },
            };
        }

        if city != self.city {
            self.city = city.to_string();
            self.view = WeatherView::Loading;
            self.next_refresh = now;
        }
        if !self.in_flight && now >= self.next_refresh {
            if self
                .command_sender
                .send(WorkerCommand::Fetch(self.city.clone()))
                .is_ok()
            {
                self.in_flight = true;
                self.next_refresh = now + self.interval;
            } else {
                self.view = WeatherView::Error {
                    message: "weather worker stopped".into(),
                    last_good: self.last_good.clone(),
                };
            }
        }
    }
}

impl Drop for WeatherMonitor {
    fn drop(&mut self) {
        let _ = self.command_sender.send(WorkerCommand::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Deserialize)]
struct WttrResponse {
    current_condition: Vec<WttrCondition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WttrCondition {
    #[serde(rename = "temp_C")]
    temperature_c: String,
    #[serde(rename = "FeelsLikeC")]
    feels_like_c: String,
    humidity: String,
    #[serde(rename = "windspeedKmph")]
    wind_kmh: String,
    #[serde(rename = "weatherDesc")]
    descriptions: Vec<WttrDescription>,
}

#[derive(Deserialize)]
struct WttrDescription {
    value: String,
}

pub fn parse_wttr_response(body: &str, updated_at: SystemTime) -> Result<WeatherReport, String> {
    let response: WttrResponse =
        serde_json::from_str(body).map_err(|error| format!("invalid wttr.in JSON: {error}"))?;
    let condition = response
        .current_condition
        .into_iter()
        .next()
        .ok_or_else(|| "wttr.in returned no current condition".to_string())?;
    let description = condition
        .descriptions
        .into_iter()
        .next()
        .map(|description| description.value.trim().to_string())
        .filter(|description| !description.is_empty())
        .ok_or_else(|| "wttr.in returned no condition description".to_string())?;
    let humidity_percent: u8 = parse_number(&condition.humidity, "humidity")?;
    if humidity_percent > 100 {
        return Err("wttr.in returned invalid humidity".into());
    }

    Ok(WeatherReport {
        condition: description,
        temperature_c: parse_number(&condition.temperature_c, "temperature")?,
        feels_like_c: parse_number(&condition.feels_like_c, "feels-like temperature")?,
        humidity_percent,
        wind_kmh: parse_number(&condition.wind_kmh, "wind speed")?,
        updated_at,
    })
}

fn parse_number<T: std::str::FromStr>(value: &str, field: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("wttr.in returned invalid {field}"))
}
