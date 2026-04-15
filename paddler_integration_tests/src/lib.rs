use std::sync::LazyLock;
use std::time::Duration;

use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use tokio::process::Child;
use tokio::process::Command;

pub mod managed_agent;
pub mod managed_balancer;
pub mod managed_cluster;
pub mod managed_cluster_params;

pub const BALANCER_MANAGEMENT_ADDR: &str = "127.0.0.1:8060";
pub const BALANCER_INFERENCE_ADDR: &str = "127.0.0.1:8061";
pub const BALANCER_OPENAI_ADDR: &str = "127.0.0.1:8062";
pub const WAIT_FOR_STATE_CHANGE_TIMEOUT: Duration = Duration::from_secs(30);
pub const WAIT_FOR_STATE_CHANGE_POLL_INTERVAL: Duration = Duration::from_millis(10);

static PADDLER_BINARY_PATH: LazyLock<String> = LazyLock::new(|| {
    std::env::var("PADDLER_BINARY_PATH")
        .unwrap_or_else(|_| "../target/debug/paddler_cli".to_owned())
});

pub fn paddler_command() -> Command {
    let mut command = Command::new(PADDLER_BINARY_PATH.as_str());

    if let Ok(profile_file) = std::env::var("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile_file);
    }

    command
}

pub static AGENT_DESIRED_MODEL: LazyLock<AgentDesiredModel> = LazyLock::new(|| {
    AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
        filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
        repo_id: "Qwen/Qwen3-0.6B-GGUF".to_owned(),
        revision: "main".to_owned(),
    })
});

pub fn terminate_child(child: &mut Child) {
    let child_id = child.id();

    if let Some(raw_pid) = child_id {
        #[expect(clippy::cast_possible_wrap, reason = "PID values fit in i32")]
        let pid = Pid::from_raw(raw_pid as i32);
        let _ = kill(pid, Signal::SIGTERM);

        let deadline = std::time::Instant::now() + Duration::from_secs(1);

        loop {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => break,
            }
        }
    }

    let _ = child.start_kill();

    loop {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => break,
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}
