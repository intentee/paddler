use std::env;

use tokio::process::Command;

#[must_use]
pub fn paddler_command(binary_path: &str) -> Command {
    let mut command = Command::new(binary_path);

    command.kill_on_drop(true);

    if let Ok(profile_file) = env::var("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile_file);
    }

    command
}
