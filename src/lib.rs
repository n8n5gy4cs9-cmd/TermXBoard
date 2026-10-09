//! TermXBoard's behavior seams.

pub mod news;
pub mod prd;
pub mod preferences;
pub mod progress;
pub mod sound;
pub mod telemetry;
pub mod ui;
pub mod visuals;
pub mod weather;

pub const MIN_WIDTH: u16 = 110;
pub const MIN_HEIGHT: u16 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenMode {
    Dashboard,
    Resize,
}

pub fn screen_mode(width: u16, height: u16) -> ScreenMode {
    if width < MIN_WIDTH || height < MIN_HEIGHT {
        ScreenMode::Resize
    } else {
        ScreenMode::Dashboard
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCommand {
    Character(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Backspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Continue,
    Quit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AppView {
    #[default]
    Dashboard,
    Task,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsField {
    #[default]
    City,
    Theme,
    ReducedMotion,
    VisualMode,
    TaskSound,
}

impl SettingsField {
    fn next(self) -> Self {
        match self {
            Self::City => Self::Theme,
            Self::Theme => Self::ReducedMotion,
            Self::ReducedMotion => Self::VisualMode,
            Self::VisualMode => Self::TaskSound,
            Self::TaskSound => Self::City,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::City => Self::TaskSound,
            Self::Theme => Self::City,
            Self::ReducedMotion => Self::Theme,
            Self::VisualMode => Self::ReducedMotion,
            Self::TaskSound => Self::VisualMode,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AppState {
    help_visible: bool,
    settings_visible: bool,
    settings_field: SettingsField,
    preferences: preferences::Preferences,
    preferences_changed: bool,
    first_run: bool,
    city_draft: String,
    city_untouched: bool,
    city_editing: bool,
    warning: Option<String>,
    weather_refresh_requested: bool,
    view: AppView,
}

impl AppState {
    pub fn new(
        preferences: preferences::Preferences,
        first_run: bool,
        warning: Option<String>,
    ) -> Self {
        Self {
            city_draft: preferences.city.clone(),
            preferences,
            first_run,
            warning,
            help_visible: false,
            settings_visible: false,
            settings_field: SettingsField::City,
            preferences_changed: false,
            city_untouched: true,
            city_editing: false,
            weather_refresh_requested: false,
            view: AppView::Dashboard,
        }
    }

    pub fn is_help_visible(&self) -> bool {
        self.help_visible
    }

    pub fn is_settings_visible(&self) -> bool {
        self.settings_visible
    }

    pub fn is_first_run(&self) -> bool {
        self.first_run
    }

    pub fn settings_field(&self) -> SettingsField {
        self.settings_field
    }

    pub fn preferences(&self) -> &preferences::Preferences {
        &self.preferences
    }

    pub fn view(&self) -> AppView {
        self.view
    }

    pub fn show_task_view(&mut self) {
        self.view = AppView::Task;
    }

    pub fn remember_project(&mut self, path: std::path::PathBuf) {
        self.preferences.remembered_project = Some(path);
        self.preferences_changed = true;
    }

    pub fn city_draft(&self) -> &str {
        &self.city_draft
    }

    pub fn is_city_editing(&self) -> bool {
        self.city_editing
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    pub fn set_warning(&mut self, warning: Option<String>) {
        self.warning = warning;
    }

    pub fn take_preferences_changed(&mut self) -> Option<preferences::Preferences> {
        self.preferences_changed.then(|| {
            self.preferences_changed = false;
            self.preferences.clone()
        })
    }

    pub fn take_weather_refresh_requested(&mut self) -> bool {
        std::mem::take(&mut self.weather_refresh_requested)
    }

    fn commit_city(&mut self) {
        let city = self.city_draft.trim();
        if !city.is_empty() {
            self.preferences.city = city.to_string();
            self.preferences_changed = true;
            self.first_run = false;
            self.city_editing = false;
            self.city_untouched = true;
        }
    }

    fn adjust_selected_setting(&mut self, forward: bool) {
        match self.settings_field {
            SettingsField::City => return,
            SettingsField::Theme => {
                self.preferences.theme = if forward {
                    self.preferences.theme.next()
                } else {
                    self.preferences.theme.previous()
                };
            }
            SettingsField::ReducedMotion => {
                self.preferences.reduced_motion = !self.preferences.reduced_motion;
            }
            SettingsField::VisualMode => {
                self.preferences.visual_mode = if forward {
                    self.preferences.visual_mode.next()
                } else {
                    self.preferences.visual_mode.previous()
                };
            }
            SettingsField::TaskSound => {
                self.preferences.task_sound = !self.preferences.task_sound;
            }
        }
        self.preferences_changed = true;
    }

    fn activate_selected_setting(&mut self) {
        if self.settings_field == SettingsField::City {
            self.city_draft = self.preferences.city.clone();
            self.city_editing = true;
        } else {
            self.adjust_selected_setting(true);
        }
    }

    pub fn handle_key(&mut self, key: KeyCommand) -> AppAction {
        if key == KeyCommand::Character('q') && !self.first_run && !self.city_editing {
            return AppAction::Quit;
        }

        if self.first_run {
            match key {
                KeyCommand::Enter => self.commit_city(),
                KeyCommand::Backspace => {
                    self.city_draft.pop();
                    self.city_untouched = false;
                }
                KeyCommand::Character(character) => {
                    if self.city_untouched {
                        self.city_draft.clear();
                        self.city_untouched = false;
                    }
                    self.city_draft.push(character);
                }
                _ => {}
            }
            return AppAction::Continue;
        }

        if self.city_editing {
            match key {
                KeyCommand::Enter => self.commit_city(),
                KeyCommand::Escape => {
                    self.city_draft = self.preferences.city.clone();
                    self.city_editing = false;
                }
                KeyCommand::Backspace => {
                    self.city_draft.pop();
                }
                KeyCommand::Character(character) => self.city_draft.push(character),
                _ => {}
            }
            return AppAction::Continue;
        }

        match key {
            KeyCommand::Character('h') => {
                self.help_visible = !self.help_visible;
                self.settings_visible = false;
                AppAction::Continue
            }
            KeyCommand::Character('s') => {
                self.settings_visible = !self.settings_visible;
                self.help_visible = false;
                AppAction::Continue
            }
            KeyCommand::Character('w') => {
                self.weather_refresh_requested = true;
                AppAction::Continue
            }
            KeyCommand::Character('d') => {
                self.view = AppView::Dashboard;
                AppAction::Continue
            }
            KeyCommand::Escape => {
                self.help_visible = false;
                self.settings_visible = false;
                AppAction::Continue
            }
            KeyCommand::Up if self.settings_visible => {
                self.settings_field = self.settings_field.previous();
                AppAction::Continue
            }
            KeyCommand::Down if self.settings_visible => {
                self.settings_field = self.settings_field.next();
                AppAction::Continue
            }
            KeyCommand::Left if self.settings_visible => {
                self.adjust_selected_setting(false);
                AppAction::Continue
            }
            KeyCommand::Right if self.settings_visible => {
                self.adjust_selected_setting(true);
                AppAction::Continue
            }
            KeyCommand::Enter if self.settings_visible => {
                self.activate_selected_setting();
                AppAction::Continue
            }
            _ => AppAction::Continue,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(preferences::Preferences::default(), false, None)
    }
}
