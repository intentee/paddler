use std::env;
use std::sync::LazyLock;

use tokio::process::Command;

static PADDLER_BINARY_PATH: LazyLock<String> = LazyLock::new(|| {
    env::var("PADDLER_BINARY_PATH").unwrap_or_else(|_| "../target/debug/paddler".to_owned())
});

pub fn paddler_command() -> Command {
    let mut command = Command::new(PADDLER_BINARY_PATH.as_str());

    if let Ok(profile_file) = env::var("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile_file);
    }

    command
}
