//! TermXBoard's behavior seams.

pub mod ui;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Continue,
    Quit,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct AppState {
    help_visible: bool,
}

impl AppState {
    pub fn is_help_visible(&self) -> bool {
        self.help_visible
    }

    pub fn handle_key(&mut self, key: KeyCommand) -> AppAction {
        match key {
            KeyCommand::Character('q') => AppAction::Quit,
            KeyCommand::Character('?') => {
                self.help_visible = !self.help_visible;
                AppAction::Continue
            }
            KeyCommand::Character(_) => AppAction::Continue,
        }
    }
}
