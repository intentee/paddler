use std::io;

use anyhow::Context;
use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;

pub struct RawTerminalGuard;

impl RawTerminalGuard {
    pub fn enter() -> Result<Self> {
        enable_raw_mode().context("enabling raw mode")?;
        if let Err(enter_alt_screen_error) = io::stdout().execute(EnterAlternateScreen) {
            if let Err(rollback_error) = disable_raw_mode() {
                log::error!(
                    "failed to disable raw mode while rolling back alt-screen entry: {rollback_error}"
                );
            }
            return Err(
                anyhow::Error::from(enter_alt_screen_error).context("entering alternate screen")
            );
        }
        if let Err(enable_mouse_error) = io::stdout().execute(EnableMouseCapture) {
            if let Err(leave_alt_screen_error) = io::stdout().execute(LeaveAlternateScreen) {
                log::error!(
                    "failed to leave alt screen while rolling back mouse-capture: {leave_alt_screen_error}"
                );
            }
            if let Err(rollback_error) = disable_raw_mode() {
                log::error!(
                    "failed to disable raw mode while rolling back mouse-capture: {rollback_error}"
                );
            }
            return Err(anyhow::Error::from(enable_mouse_error).context("enabling mouse capture"));
        }
        Ok(Self)
    }
}

impl Drop for RawTerminalGuard {
    fn drop(&mut self) {
        if let Err(disable_mouse_error) = io::stdout().execute(DisableMouseCapture) {
            log::error!("failed to disable mouse capture: {disable_mouse_error}");
        }
        if let Err(leave_alt_screen_error) = io::stdout().execute(LeaveAlternateScreen) {
            log::error!("failed to leave alternate screen: {leave_alt_screen_error}");
        }
        if let Err(disable_raw_mode_error) = disable_raw_mode() {
            log::error!("failed to disable raw mode: {disable_raw_mode_error}");
        }
    }
}
