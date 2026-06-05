#![cfg(unix)]

use std::process::Command;

/// Recreates the original bug: under a low `RLIMIT_NOFILE`, starting the balancer used to spawn
/// dozens of actix worker runtimes, exhaust the descriptor table, and panic inside actix with an
/// opaque `RecvError`. The balancer must now refuse to start with a readable, actionable error
/// before any actix server is built.
#[test]
fn balancer_reports_readable_error_when_file_descriptor_limit_is_too_low() {
    let paddler_binary = env!("CARGO_BIN_EXE_paddler");

    // 64 is below the ~165 descriptors the three-server balancer requires, and well above the
    // descriptors the process opens before the pre-flight check runs. The child shell lowers its
    // own soft limit, then `exec`s the balancer, which inherits it.
    let invocation = format!(
        "ulimit -n 64; exec {paddler_binary} balancer \
         --inference-addr 127.0.0.1:0 --management-addr 127.0.0.1:0 --compat-openai-addr 127.0.0.1:0"
    );

    let output = Command::new("sh")
        .arg("-c")
        .arg(&invocation)
        .output()
        .expect("failed to spawn the paddler balancer subprocess");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "balancer should exit non-zero when the descriptor limit is too low; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("open file-descriptor limit is too low") && stderr.contains("ulimit -n"),
        "balancer should report a readable, actionable descriptor error; stderr was:\n{stderr}"
    );
    assert!(
        !stderr.contains("panicked")
            && !stderr.contains("RecvError")
            && !stderr.contains("Too many open files"),
        "balancer must not surface the actix panic cascade; stderr was:\n{stderr}"
    );
}
