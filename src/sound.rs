//! Optional audible feedback when the Task list changes.
//!
//! The bleep is off unless Task sound is switched on in Settings. It is played
//! through the operating system's audio device rather than the terminal bell,
//! so it stays audible while another application holds focus.

use std::process::{Child, Command, Stdio};

/// Plays the Task change bleep.
pub trait Bleeper {
    fn play(&mut self) -> Result<(), String>;
}

#[cfg(target_os = "macos")]
const SOUND_CANDIDATES: [&str; 3] = [
    "/System/Library/Sounds/Glass.aiff",
    "/System/Library/Sounds/Ping.aiff",
    "/System/Library/Sounds/Tink.aiff",
];

/// Bleeper backed by the system audio player.
#[derive(Debug, Default)]
pub struct SystemBleeper {
    pending: Vec<Child>,
}

impl SystemBleeper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops finished players so repeated bleeps do not leave child processes.
    fn reap(&mut self) {
        self.pending
            .retain_mut(|child| matches!(child.try_wait(), Ok(None)));
    }
}

impl Bleeper for SystemBleeper {
    #[cfg(target_os = "macos")]
    fn play(&mut self) -> Result<(), String> {
        self.reap();
        let sound = SOUND_CANDIDATES
            .iter()
            .find(|candidate| std::path::Path::new(candidate).exists())
            .ok_or_else(|| "Task sound unavailable: no system sound file found".to_string())?;
        let child = Command::new("afplay")
            .arg(sound)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("could not play Task sound: {error}"))?;
        self.pending.push(child);
        Ok(())
    }

    /// Falls back to the terminal bell where no system player is assumed.
    /// Audibility while unfocused then depends on the terminal emulator.
    #[cfg(not(target_os = "macos"))]
    fn play(&mut self) -> Result<(), String> {
        use std::io::Write;
        self.reap();
        let mut stderr = std::io::stderr();
        stderr
            .write_all(b"\x07")
            .and_then(|()| stderr.flush())
            .map_err(|error| format!("could not play Task sound: {error}"))
    }
}
