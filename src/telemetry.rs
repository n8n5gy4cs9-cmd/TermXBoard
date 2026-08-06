use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use battery::units::ratio::percent as ratio_percent;
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TelemetrySnapshot {
    pub cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub battery_percent: Option<f32>,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub network_received_bytes: u64,
    pub network_transmitted_bytes: u64,
    pub uptime: Duration,
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
    Error(String),
}

pub trait TelemetrySource: Send + 'static {
    fn collect(&mut self) -> Result<TelemetrySnapshot, String>;
}

pub struct MacTelemetrySource {
    system: System,
    disks: Disks,
    networks: Networks,
}

impl MacTelemetrySource {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
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
        if memory_total_bytes == 0 || disk_total_bytes == 0 {
            return Err("macOS telemetry is temporarily unavailable".into());
        }

        Ok(TelemetrySnapshot {
            cpu_percent: self.system.global_cpu_usage(),
            memory_used_bytes: self.system.used_memory(),
            memory_total_bytes,
            battery_percent: Self::battery_percent(),
            disk_used_bytes: disk_total_bytes.saturating_sub(disk_available_bytes),
            disk_total_bytes,
            network_received_bytes,
            network_transmitted_bytes,
            uptime: Duration::from_secs(System::uptime()),
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
        }
    }

    pub fn view(&self) -> &TelemetryView {
        &self.view
    }

    pub fn tick(&mut self, now: Instant) {
        while let Ok(result) = self.result_receiver.try_recv() {
            self.in_flight = false;
            self.view = match result {
                Ok(snapshot) => TelemetryView::Ready(snapshot),
                Err(message) => TelemetryView::Error(message),
            };
        }
        if !self.in_flight && now >= self.next_refresh {
            if self.command_sender.send(WorkerCommand::Collect).is_ok() {
                self.in_flight = true;
                self.next_refresh = now + self.interval;
            } else {
                self.view = TelemetryView::Error("telemetry worker stopped".into());
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
        Self::Error(message.into())
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error(message) => Some(message),
            _ => None,
        }
    }

    pub fn cards(&self) -> Vec<MetricCard> {
        match self {
            Self::Loading => uniform_cards("Loading…", LedState::Orange),
            Self::Error(_) => uniform_cards("Error", LedState::Red),
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
        available("CPU", format!("{:.0}%", snapshot.cpu_percent)),
        available(
            "MEMORY",
            format_capacity_pair(snapshot.memory_used_bytes, snapshot.memory_total_bytes),
        ),
        match snapshot.battery_percent {
            Some(percent) => available("BATTERY", format!("{percent:.0}%")),
            None => MetricCard {
                label: "BATTERY",
                value: "Unavailable".into(),
                led: LedState::Red,
            },
        },
        available(
            "DISK",
            format_capacity_pair(snapshot.disk_used_bytes, snapshot.disk_total_bytes),
        ),
        available(
            "NETWORK",
            format!(
                "↓ {}  ↑ {}",
                format_bytes(snapshot.network_received_bytes),
                format_bytes(snapshot.network_transmitted_bytes)
            ),
        ),
        available("UPTIME", format_uptime(snapshot.uptime)),
    ]
}

fn available(label: &'static str, value: String) -> MetricCard {
    MetricCard {
        label,
        value,
        led: LedState::Green,
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
