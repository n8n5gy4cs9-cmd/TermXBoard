use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    SignatureNeon,
    Cyberpunk,
    Matrix,
    Nord,
    Dracula,
    SolarizedDark,
    AmberCrt,
}

impl Theme {
    pub const ALL: [Self; 7] = [
        Self::SignatureNeon,
        Self::Cyberpunk,
        Self::Matrix,
        Self::Nord,
        Self::Dracula,
        Self::SolarizedDark,
        Self::AmberCrt,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::SignatureNeon => "Signature Neon",
            Self::Cyberpunk => "Cyberpunk",
            Self::Matrix => "Matrix",
            Self::Nord => "Nord",
            Self::Dracula => "Dracula",
            Self::SolarizedDark => "Solarized Dark",
            Self::AmberCrt => "Amber CRT",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|theme| *theme == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|theme| *theme == self)
            .unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Glyph set used for icons and terminal decoration.
pub enum VisualMode {
    #[default]
    Unicode,
    NerdFont,
    Ascii,
}

impl VisualMode {
    /// All selectable glyph presets.
    pub const ALL: [Self; 3] = [Self::Unicode, Self::NerdFont, Self::Ascii];

    /// Human-readable preset name.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unicode => "Unicode",
            Self::NerdFont => "Nerd Font",
            Self::Ascii => "ASCII",
        }
    }

    /// Next preset in menu order.
    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    /// Previous preset in menu order.
    pub fn previous(self) -> Self {
        let index = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub city: String,
    pub theme: Theme,
    pub reduced_motion: bool,
    /// Preferred icon and decoration glyph set.
    pub visual_mode: VisualMode,
    /// Play a bleep when the Task list changes. Off unless switched on here.
    pub task_sound: bool,
    pub remembered_project: Option<PathBuf>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            city: "Turku, Finland".into(),
            theme: Theme::SignatureNeon,
            reduced_motion: false,
            visual_mode: VisualMode::Unicode,
            task_sound: false,
            remembered_project: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedPreferences {
    pub preferences: Preferences,
    pub is_first_run: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved,
    SessionOnly(String),
}

impl SaveOutcome {
    pub fn is_saved(&self) -> bool {
        matches!(self, Self::Saved)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferencesStore {
    path: PathBuf,
}

impl PreferencesStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn beside_executable() -> std::io::Result<Self> {
        let executable = std::env::current_exe()?;
        let directory = executable.parent().ok_or_else(|| {
            std::io::Error::other("TermXBoard executable has no parent directory")
        })?;
        Ok(Self::new(directory.join("config.json")))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> LoadedPreferences {
        if !self.path.exists() {
            return LoadedPreferences {
                preferences: Preferences::default(),
                is_first_run: true,
                warning: None,
            };
        }

        match fs::read_to_string(&self.path)
            .map_err(|error| error.to_string())
            .and_then(|contents| serde_json::from_str(&contents).map_err(|error| error.to_string()))
        {
            Ok(preferences) => LoadedPreferences {
                preferences,
                is_first_run: false,
                warning: None,
            },
            Err(error) => LoadedPreferences {
                preferences: Preferences::default(),
                is_first_run: false,
                warning: Some(format!("Invalid config; using session defaults: {error}")),
            },
        }
    }

    pub fn save(&self, preferences: &Preferences) -> SaveOutcome {
        let result = (|| -> Result<(), String> {
            let parent = self
                .path
                .parent()
                .ok_or_else(|| "config path has no parent".to_string())?;
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            let contents =
                serde_json::to_string_pretty(preferences).map_err(|error| error.to_string())?;
            fs::write(&self.path, format!("{contents}\n")).map_err(|error| error.to_string())
        })();

        match result {
            Ok(()) => SaveOutcome::Saved,
            Err(error) => SaveOutcome::SessionOnly(format!(
                "Settings are session-only; could not write {}: {error}",
                self.path.display()
            )),
        }
    }
}
