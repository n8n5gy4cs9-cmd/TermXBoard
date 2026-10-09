use ratatui::style::Color;

use crate::{
    preferences::{Theme, VisualMode},
    progress::{ProjectActivity, TaskStatus},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Theme-independent semantic color role.
pub enum SemanticTone {
    Primary,
    Secondary,
    Accent,
    Muted,
    Danger,
    Info,
    Review,
    Warning,
    Success,
}

impl SemanticTone {
    pub const ALL: [Self; 9] = [
        Self::Primary,
        Self::Secondary,
        Self::Accent,
        Self::Muted,
        Self::Danger,
        Self::Info,
        Self::Review,
        Self::Warning,
        Self::Success,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Color capability available in the active terminal.
pub enum ColorSupport {
    TrueColor,
    Ansi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Deterministic frame for shared dashboard animations.
pub struct AnimationFrame {
    step: usize,
    reduced_motion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Bright-head and dim-tail positions in the 24-cell Task View scanner.
pub struct TaskScanner {
    /// First cell of the three-cell bright head.
    pub head: usize,
    /// Single dim tail cell.
    pub tail: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Semantic motion and color choices for Task View refresh activity.
pub struct TaskActivityVisual {
    /// Scanner color role.
    pub scanner_tone: SemanticTone,
    /// Heartbeat color role.
    pub heartbeat_tone: SemanticTone,
    /// Whether the scanner uses its faster cadence.
    pub scanner_fast: bool,
    /// Whether scanner motion is frozen.
    pub static_motion: bool,
    /// Heartbeat period, or `None` for a steady indicator.
    pub heartbeat_period_ms: Option<i64>,
}

impl TaskActivityVisual {
    /// Maps refresh activity and motion preference to visual behavior.
    pub const fn for_activity(activity: ProjectActivity, reduced_motion: bool) -> Self {
        let (scanner_tone, heartbeat_tone, scanner_fast, heartbeat_period_ms) = match activity {
            ProjectActivity::Healthy | ProjectActivity::Recovered => (
                SemanticTone::Accent,
                SemanticTone::Success,
                false,
                Some(1200),
            ),
            ProjectActivity::Countdown(_) => (
                SemanticTone::Warning,
                SemanticTone::Warning,
                true,
                Some(600),
            ),
            ProjectActivity::Updating => (
                SemanticTone::Warning,
                SemanticTone::Warning,
                true,
                Some(300),
            ),
            ProjectActivity::Error => (SemanticTone::Danger, SemanticTone::Danger, false, None),
        };
        Self {
            scanner_tone,
            heartbeat_tone,
            scanner_fast,
            static_motion: reduced_motion || matches!(activity, ProjectActivity::Error),
            heartbeat_period_ms: if reduced_motion {
                None
            } else {
                heartbeat_period_ms
            },
        }
    }
}

impl TaskScanner {
    /// Calculates scanner positions from supplied time and motion settings.
    pub fn at(timestamp_millis: i64, reduced_motion: bool, fast: bool) -> Self {
        const TRAVEL: usize = 21;
        if reduced_motion {
            return Self { head: 10, tail: 9 };
        }
        let frame_ms = if fast { 80 } else { 125 };
        let step = (timestamp_millis.unsigned_abs() / frame_ms) as usize;
        let phase = step % (TRAVEL * 2);
        if phase <= TRAVEL {
            Self {
                head: phase,
                tail: phase.saturating_sub(1),
            }
        } else {
            let head = TRAVEL * 2 - phase;
            Self {
                head,
                tail: (head + 3).min(23),
            }
        }
    }
}

impl AnimationFrame {
    /// Builds a deterministic animation frame from supplied epoch milliseconds.
    pub fn at(timestamp_millis: i64, reduced_motion: bool) -> Self {
        Self {
            step: timestamp_millis.div_euclid(180) as usize,
            reduced_motion,
        }
    }

    /// Renders the compact loading scan for a glyph mode.
    pub fn loading_scan(self, mode: VisualMode) -> String {
        if self.reduced_motion {
            return match mode {
                VisualMode::Ascii => "[=   ]".into(),
                _ => "▰▱▱▱▱".into(),
            };
        }
        let position = self.step % 5;
        if mode == VisualMode::Ascii {
            let mut cells = [' '; 5];
            cells[position] = '>';
            format!("[{}]", cells.iter().collect::<String>())
        } else {
            (0..5)
                .map(|index| if index == position { '▰' } else { '▱' })
                .collect()
        }
    }

    /// Returns the current semantic tone for changed Task highlighting.
    pub fn changed_tone(self) -> SemanticTone {
        if self.reduced_motion || self.step.is_multiple_of(2) {
            SemanticTone::Accent
        } else {
            SemanticTone::Success
        }
    }
}

impl ColorSupport {
    /// Detects ANSI-only or true-color support from terminal environment values.
    pub fn detect(term: Option<&str>, colorterm: Option<&str>, no_color: bool) -> Self {
        if no_color || term == Some("dumb") {
            return Self::Ansi;
        }
        match colorterm.map(str::to_ascii_lowercase).as_deref() {
            Some("truecolor" | "24bit") | None => Self::TrueColor,
            _ => Self::Ansi,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Icon, label, and semantic tone for one supplied Task Status.
pub struct StatusVisual {
    /// Display label for the status.
    pub label: &'static str,
    /// Semantic color role for the status.
    pub tone: SemanticTone,
    unicode: &'static str,
    nerd_font: &'static str,
    ascii: &'static str,
}

impl StatusVisual {
    /// Returns the fixed visual language for a Task Status.
    pub const fn for_status(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Blocked => Self::new("Blocked", SemanticTone::Danger, "◆", "󰅖", "X"),
            TaskStatus::InProgress => Self::new("WIP", SemanticTone::Info, "▶", "󰐊", ">"),
            TaskStatus::AwaitingReview => {
                Self::new("User Review", SemanticTone::Review, "◉", "󰄬", "?")
            }
            TaskStatus::Todo => Self::new("Undone", SemanticTone::Warning, "○", "󰄱", "o"),
            TaskStatus::Done => Self::new("Done", SemanticTone::Success, "✓", "󰄵", "v"),
        }
    }

    const fn new(
        label: &'static str,
        tone: SemanticTone,
        unicode: &'static str,
        nerd_font: &'static str,
        ascii: &'static str,
    ) -> Self {
        Self {
            label,
            tone,
            unicode,
            nerd_font,
            ascii,
        }
    }

    /// Returns the status icon for the selected glyph mode.
    pub const fn glyph(self, mode: VisualMode) -> &'static str {
        match mode {
            VisualMode::Unicode => self.unicode,
            VisualMode::NerdFont => self.nerd_font,
            VisualMode::Ascii => self.ascii,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Temperature range used for honest current-condition coloring.
pub enum TemperatureBand {
    Cold,
    Mild,
    Warm,
    Hot,
}

impl TemperatureBand {
    /// Classifies a Celsius temperature using fixed display thresholds.
    pub const fn from_celsius(value: i16) -> Self {
        match value {
            ..=5 => Self::Cold,
            6..=20 => Self::Mild,
            21..=27 => Self::Warm,
            _ => Self::Hot,
        }
    }

    /// Returns the semantic color role for this temperature range.
    pub const fn tone(self) -> SemanticTone {
        match self {
            Self::Cold => SemanticTone::Info,
            Self::Mild => SemanticTone::Success,
            Self::Warm => SemanticTone::Warning,
            Self::Hot => SemanticTone::Danger,
        }
    }
}

/// Resolves a semantic tone to a terminal color for a theme and capability.
pub fn theme_color(theme: Theme, tone: SemanticTone, support: ColorSupport) -> Color {
    if support == ColorSupport::Ansi {
        return match tone {
            SemanticTone::Primary | SemanticTone::Info => Color::Cyan,
            SemanticTone::Secondary | SemanticTone::Review => Color::Magenta,
            SemanticTone::Accent | SemanticTone::Warning => Color::Yellow,
            SemanticTone::Muted => Color::DarkGray,
            SemanticTone::Danger => Color::Red,
            SemanticTone::Success => Color::Green,
        };
    }
    let value = match tone {
        SemanticTone::Danger => match theme {
            Theme::Nord => 0xbf616a,
            Theme::AmberCrt => 0xff5f00,
            _ => 0xff3b5c,
        },
        SemanticTone::Info => match theme {
            Theme::AmberCrt => 0xffc247,
            Theme::Matrix => 0x55ff88,
            _ => 0x00d9ff,
        },
        SemanticTone::Review => match theme {
            Theme::Nord => 0xb48ead,
            Theme::AmberCrt => 0xff8c00,
            _ => 0xd66bff,
        },
        SemanticTone::Warning => match theme {
            Theme::SolarizedDark => 0xb58900,
            _ => 0xffb000,
        },
        SemanticTone::Success => match theme {
            Theme::AmberCrt => 0xffd166,
            _ => 0x35e06f,
        },
        SemanticTone::Primary => match theme {
            Theme::SignatureNeon => 0x00f0ff,
            Theme::Cyberpunk => 0xffea00,
            Theme::Matrix => 0x00ff41,
            Theme::Nord => 0x88c0d0,
            Theme::Dracula => 0x8be9fd,
            Theme::SolarizedDark => 0x2aa198,
            Theme::AmberCrt => 0xffb000,
        },
        SemanticTone::Secondary => match theme {
            Theme::SignatureNeon => 0x9a4dff,
            Theme::Cyberpunk => 0x00ffd1,
            Theme::Matrix => 0x00b42d,
            Theme::Nord => 0x81a1c1,
            Theme::Dracula => 0xbd93f9,
            Theme::SolarizedDark => 0x268bd2,
            Theme::AmberCrt => 0xff8000,
        },
        SemanticTone::Accent => match theme {
            Theme::SignatureNeon => 0xff2d95,
            Theme::Cyberpunk => 0xff006e,
            Theme::Matrix => 0xaaffbe,
            Theme::Nord => 0xb48ead,
            Theme::Dracula => 0xff79c6,
            Theme::SolarizedDark => 0xd33682,
            Theme::AmberCrt => 0xffd666,
        },
        SemanticTone::Muted => match theme {
            Theme::SignatureNeon => 0x56687a,
            Theme::Cyberpunk => 0x6e6080,
            Theme::Matrix => 0x376441,
            Theme::Nord => 0x4c566a,
            Theme::Dracula => 0x6272a4,
            Theme::SolarizedDark => 0x586e75,
            Theme::AmberCrt => 0x805714,
        },
    };
    Color::Rgb(
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    )
}
