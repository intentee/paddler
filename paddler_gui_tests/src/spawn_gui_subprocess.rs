use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;
use tokio::process::Child;

use crate::paddler_gui_command::paddler_gui_command;
use crate::spawn_gui_subprocess_params::SpawnGuiSubprocessParams;

pub fn spawn_gui_subprocess(
    SpawnGuiSubprocessParams { display_name, args }: SpawnGuiSubprocessParams,
) -> Result<Child> {
    let mut command = paddler_gui_command()?;

    command
        .env_remove("WAYLAND_DISPLAY")
        .env("DISPLAY", display_name)
        .env("RUST_LOG", "paddler_gui=info")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    for argument in args {
        command.arg(argument);
    }

    command
        .spawn()
        .context("failed to spawn paddler_gui binary")
}
