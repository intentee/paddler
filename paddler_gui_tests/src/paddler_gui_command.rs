use anyhow::Result;
use tokio::process::Command;

use crate::paddler_gui_binary_path::paddler_gui_binary_path;

pub fn paddler_gui_command() -> Result<Command> {
    Ok(Command::new(paddler_gui_binary_path()?))
}
