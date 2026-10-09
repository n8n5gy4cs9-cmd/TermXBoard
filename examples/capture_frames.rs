use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use chrono::{TimeZone, Utc};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use serde_json::json;
use termxboard::{
    AppState,
    news::{Headline, NewsFeed, NewsSelection, NewsView},
    preferences::{Preferences, Theme},
    progress::{LoadProjectMenu, LoadedProject, Project, TaskControls, TaskSelection},
    telemetry::{Capacity, NetworkActivity, TelemetrySnapshot, TelemetryView},
    ui::{self, ApplicationData},
    visuals::ColorSupport,
    weather::{WeatherReport, WeatherView},
};

const WIDTH: u16 = 110;
const HEIGHT: u16 = 32;

fn main() {
    unsafe { std::env::set_var("TERMXBOARD_RENDER_TIME", "2026-08-07T18:42:00Z") };
    unsafe { std::env::set_var("TZ", "UTC") };
    unsafe { std::env::set_var("COLORTERM", "truecolor") };
    let output = PathBuf::from(std::env::args_os().nth(1).expect("output directory"));
    fs::create_dir_all(&output).expect("create output directory");
    for (theme, slug) in [
        (Theme::SignatureNeon, "signature-neon"),
        (Theme::AmberCrt, "amber-crt"),
    ] {
        capture(&output.join(format!("dashboard-{slug}.json")), theme, false);
        capture(&output.join(format!("tasks-{slug}.json")), theme, true);
    }
}

fn capture(path: &Path, theme: Theme, task_view: bool) {
    let project = fixture_project();
    let selection = TaskSelection::for_project(&project.project);
    let mut app = AppState::new(
        Preferences {
            theme,
            reduced_motion: true,
            ..Preferences::default()
        },
        false,
        None,
    );
    if task_view {
        app.show_task_view();
    }
    let telemetry = TelemetryView::ready(TelemetrySnapshot {
        cpu_percent: Some(37.0),
        memory: Some(Capacity {
            used_bytes: 11 * 1024 * 1024 * 1024,
            total_bytes: 16 * 1024 * 1024 * 1024,
        }),
        battery_percent: Some(84.0),
        disk: Some(Capacity {
            used_bytes: 380 * 1024 * 1024 * 1024,
            total_bytes: 1000 * 1024 * 1024 * 1024,
        }),
        network: Some(NetworkActivity {
            received_bytes: 8_400_000,
            transmitted_bytes: 1_200_000,
        }),
        uptime: Some(Duration::from_secs(345_600)),
    });
    let now = Utc.with_ymd_and_hms(2026, 8, 7, 18, 42, 0).unwrap();
    let weather = WeatherView::ready(
        "Turku, Finland",
        WeatherReport {
            condition: "Clear sky".into(),
            temperature_c: 18,
            feels_like_c: 17,
            humidity_percent: 61,
            wind_kmh: 12,
            updated_at: SystemTime::UNIX_EPOCH
                + Duration::from_secs(now.timestamp().saturating_sub(120) as u64),
        },
    );
    let news = NewsView::ready(NewsFeed::ALL.map(|feed| {
        vec![Headline {
            source: feed,
            title: format!("{} engineering dispatch", feed.label()),
            url: "https://example.com/story".into(),
            published_at: now - chrono::Duration::hours(2),
        }]
    }));
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            ui::render_application_with_color_support(
                frame,
                ApplicationData {
                    app: &app,
                    telemetry: &telemetry,
                    weather: &weather,
                    news: &news,
                    news_selection: &NewsSelection::default(),
                    active_project: task_view.then_some(&project),
                    load_menu: &LoadProjectMenu::default(),
                    task_selection: &selection,
                    task_controls: &TaskControls::default(),
                    project_monitor: None,
                    cpu_history: &[12, 25, 18, 40, 58, 37],
                    memory_history: &[54, 57, 60, 62, 66, 69],
                },
                ColorSupport::TrueColor,
            )
        })
        .expect("draw");
    let cells = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| {
            json!({
                "s": cell.symbol(), "fg": foreground_rgb(cell.fg), "bg": background_rgb(cell.bg)
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        path,
        serde_json::to_vec(&json!({"width": WIDTH, "height": HEIGHT, "cells": cells})).unwrap(),
    )
    .unwrap();
}

fn fixture_project() -> LoadedProject {
    let project: Project = serde_json::from_str(r#"{
      "project":"Nodus","currentTask":"T-2","verifyCommand":"cargo test",
      "tasks":[
        {"id":"T-1","title":"Repair sensor fault","status":"blocked","dependsOn":[]},
        {"id":"T-2","title":"Render command deck","status":"in-progress","milestone":"M1","phase":"P1","dependsOn":["T-1"]},
        {"id":"T-3","title":"Review visual states","status":"awaiting-review","milestone":"M1","phase":"P1","dependsOn":[]},
        {"id":"T-4","title":"Wire fallback mode","status":"todo","milestone":"M2","phase":"P2","dependsOn":[]},
        {"id":"T-5","title":"Ship release","status":"done","milestone":"M2","phase":"P2","dependsOn":[]}
      ]
    }"#).unwrap();
    LoadedProject {
        path: PathBuf::from("/fixture/progress.json"),
        project,
    }
}

fn foreground_rgb(color: Color) -> [u8; 3] {
    match color {
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Black => [5, 8, 12],
        Color::Reset => [205, 215, 225],
        Color::Red | Color::LightRed => [255, 70, 90],
        Color::Green | Color::LightGreen => [60, 225, 110],
        Color::Yellow | Color::LightYellow => [255, 190, 40],
        Color::Blue | Color::LightBlue => [70, 145, 255],
        Color::Magenta | Color::LightMagenta => [210, 90, 255],
        Color::Cyan | Color::LightCyan => [30, 220, 245],
        Color::Gray | Color::White => [220, 225, 230],
        Color::DarkGray => [90, 105, 120],
        Color::Indexed(value) => [value, value, value],
    }
}

fn background_rgb(color: Color) -> [u8; 3] {
    match color {
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Red | Color::LightRed => [255, 70, 90],
        Color::Green | Color::LightGreen => [60, 225, 110],
        Color::Yellow | Color::LightYellow => [255, 190, 40],
        Color::Blue | Color::LightBlue => [70, 145, 255],
        Color::Magenta | Color::LightMagenta => [210, 90, 255],
        Color::Cyan | Color::LightCyan => [30, 220, 245],
        Color::Gray | Color::White => [220, 225, 230],
        Color::DarkGray => [90, 105, 120],
        Color::Indexed(value) => [value, value, value],
        Color::Black | Color::Reset => [5, 8, 12],
    }
}
