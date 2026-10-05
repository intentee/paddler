use tokio::process::Command;

#[must_use]
pub fn paddler_command(binary_path: &str) -> Command {
    let mut command = Command::new(binary_path);

    command.kill_on_drop(true);

    command
}
