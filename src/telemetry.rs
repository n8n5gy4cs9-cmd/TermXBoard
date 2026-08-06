use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use battery::units::ratio::percent as ratio_percent;
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capacity {
    pub used_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetworkActivity {
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TelemetrySnapshot {
    pub cpu_percent: Option<f32>,
    pub memory: Option<Capacity>,
    pub battery_percent: Option<f32>,
    pub disk: Option<Capacity>,
    pub network: Option<NetworkActivity>,
    pub uptime: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedState {
    Green,
    Orange,
    Red,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricCard {
    pub label: &'static str,
    pub value: String,
    pub led: LedState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TelemetryView {
    Loading,
    Ready(TelemetrySnapshot),
    Error {
        message: String,
        last_good: Option<TelemetrySnapshot>,
    },
}

pub trait TelemetrySource: Send + 'static {
    fn collect(&mut self) -> Result<TelemetrySnapshot, String>;
}

pub struct MacTelemetrySource {
    system: System,
    disks: Disks,
    networks: Networks,
    cpu_primed: bool,
}

impl MacTelemetrySource {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            cpu_primed: false,
        }
    }

    fn battery_percent() -> Option<f32> {
        let manager = battery::Manager::new().ok()?;
        manager
            .batteries()
            .ok()?
            .filter_map(Result::ok)
            .next()
            .map(|battery| battery.state_of_charge().get::<ratio_percent>())
    }
}

impl Default for MacTelemetrySource {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetrySource for MacTelemetrySource {
    fn collect(&mut self) -> Result<TelemetrySnapshot, String> {
        if !self.cpu_primed {
            thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
            self.cpu_primed = true;
        }
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh(false);
        self.networks.refresh(false);

        let disk_total_bytes: u64 = self.disks.iter().map(|disk| disk.total_space()).sum();
        let disk_available_bytes: u64 = self.disks.iter().map(|disk| disk.available_space()).sum();
        let network_received_bytes = self
            .networks
            .values()
            .map(|network| network.received())
            .sum();
        let network_transmitted_bytes = self
            .networks
            .values()
            .map(|network| network.transmitted())
            .sum();

        let memory_total_bytes = self.system.total_memory();
        let cpu_percent = self.system.global_cpu_usage();

        Ok(TelemetrySnapshot {
            cpu_percent: cpu_percent.is_finite().then_some(cpu_percent),
            memory: (memory_total_bytes > 0).then_some(Capacity {
                used_bytes: self.system.used_memory(),
                total_bytes: memory_total_bytes,
            }),
            battery_percent: Self::battery_percent(),
            disk: (disk_total_bytes > 0).then_some(Capacity {
                used_bytes: disk_total_bytes.saturating_sub(disk_available_bytes),
                total_bytes: disk_total_bytes,
            }),
            network: (!self.networks.is_empty()).then_some(NetworkActivity {
                received_bytes: network_received_bytes,
                transmitted_bytes: network_transmitted_bytes,
            }),
            uptime: Some(Duration::from_secs(System::uptime())),
        })
    }
}

enum WorkerCommand {
    Collect,
    Stop,
}

pub struct TelemetryMonitor {
    command_sender: Sender<WorkerCommand>,
    result_receiver: Receiver<Result<TelemetrySnapshot, String>>,
    worker: Option<JoinHandle<()>>,
    view: TelemetryView,
    interval: Duration,
    next_refresh: Instant,
    in_flight: bool,
    last_good: Option<TelemetrySnapshot>,
}

impl TelemetryMonitor {
    pub fn new<S: TelemetrySource>(source: S, interval: Duration, now: Instant) -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut source = source;
            while let Ok(command) = command_receiver.recv() {
                match command {
                    WorkerCommand::Collect => {
                        if result_sender.send(source.collect()).is_err() {
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
            view: TelemetryView::Loading,
            interval,
            next_refresh: now,
            in_flight: false,
            last_good: None,
        }
    }

    pub fn view(&self) -> &TelemetryView {
        &self.view
    }

    pub fn tick(&mut self, now: Instant) {
        while let Ok(result) = self.result_receiver.try_recv() {
            self.in_flight = false;
            self.view = match result {
                Ok(snapshot) => {
                    self.last_good = Some(snapshot);
                    TelemetryView::Ready(snapshot)
                }
                Err(message) => TelemetryView::Error {
                    message,
                    last_good: self.last_good,
                },
            };
        }
        if !self.in_flight && now >= self.next_refresh {
            if self.command_sender.send(WorkerCommand::Collect).is_ok() {
                self.in_flight = true;
                self.next_refresh = now + self.interval;
            } else {
                self.view = TelemetryView::Error {
                    message: "telemetry worker stopped".into(),
                    last_good: self.last_good,
                };
            }
        }
    }
}

impl Drop for TelemetryMonitor {
    fn drop(&mut self) {
        let _ = self.command_sender.send(WorkerCommand::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

const LABELS: [&str; 6] = ["CPU", "MEMORY", "BATTERY", "DISK", "NETWORK", "UPTIME"];

impl TelemetryView {
    pub fn loading() -> Self {
        Self::Loading
    }

    pub fn ready(snapshot: TelemetrySnapshot) -> Self {
        Self::Ready(snapshot)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            last_good: None,
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error { message, .. } => Some(message),
            _ => None,
        }
    }

    pub fn cards(&self) -> Vec<MetricCard> {
        match self {
            Self::Loading => uniform_cards("Loading…", LedState::Orange),
            Self::Error {
                last_good: Some(snapshot),
                ..
            } => ready_cards(*snapshot)
                .into_iter()
                .map(|mut card| {
                    card.led = LedState::Red;
                    if card.value != "Unavailable" {
                        card.value.push_str(" (stale)");
                    }
                    card
                })
                .collect(),
            Self::Error {
                last_good: None, ..
            } => uniform_cards("Error", LedState::Red),
            Self::Ready(snapshot) => ready_cards(*snapshot),
        }
    }
}

fn uniform_cards(value: &str, led: LedState) -> Vec<MetricCard> {
    LABELS
        .into_iter()
        .map(|label| MetricCard {
            label,
            value: value.to_string(),
            led,
        })
        .collect()
}

fn ready_cards(snapshot: TelemetrySnapshot) -> Vec<MetricCard> {
    vec![
        match snapshot.cpu_percent {
            Some(value) => available("CPU", format!("{value:.0}%")),
            None => unavailable("CPU"),
        },
        match snapshot.memory {
            Some(value) => available(
                "MEMORY",
                format_capacity_pair(value.used_bytes, value.total_bytes),
            ),
            None => unavailable("MEMORY"),
        },
        match snapshot.battery_percent {
            Some(percent) => available("BATTERY", format!("{percent:.0}%")),
            None => unavailable("BATTERY"),
        },
        match snapshot.disk {
            Some(value) => available(
                "DISK",
                format_capacity_pair(value.used_bytes, value.total_bytes),
            ),
            None => unavailable("DISK"),
        },
        match snapshot.network {
            Some(value) => available(
                "NETWORK",
                format!(
                    "↓ {}  ↑ {}",
                    format_bytes(value.received_bytes),
                    format_bytes(value.transmitted_bytes)
                ),
            ),
            None => unavailable("NETWORK"),
        },
        match snapshot.uptime {
            Some(value) => available("UPTIME", format_uptime(value)),
            None => unavailable("UPTIME"),
        },
    ]
}

fn available(label: &'static str, value: String) -> MetricCard {
    MetricCard {
        label,
        value,
        led: LedState::Green,
    }
}

fn unavailable(label: &'static str) -> MetricCard {
    MetricCard {
        label,
        value: "Unavailable".into(),
        led: LedState::Red,
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes / KIB)
    } else {
        format!("{bytes:.0} B")
    }
}

fn format_capacity_pair(used: u64, total: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    format!("{:.1} / {:.1} GiB", used as f64 / GIB, total as f64 / GIB)
}

fn format_uptime(uptime: Duration) -> String {
    let minutes = uptime.as_secs() / 60;
    let days = minutes / (24 * 60);
    let hours = (minutes / 60) % 24;
    let minutes = minutes % 60;
    format!("{days}d {hours}h {minutes}m")
}
