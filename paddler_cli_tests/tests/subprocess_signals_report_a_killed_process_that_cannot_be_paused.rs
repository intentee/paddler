use nix::sys::signal::Signal;
use nix::sys::wait::WaitStatus;

use paddler_cli_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_cli_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;
use paddler_cli_tests::subprocess_process::SubprocessProcess;
use paddler_cli_tests::unreachable_management_addr::UNREACHABLE_MANAGEMENT_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_signals_report_a_killed_process_that_cannot_be_paused() {
    let agent_process = SubprocessProcess::new(
        spawn_agent_subprocess(SpawnAgentSubprocessParams {
            binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
            management_addr: UNREACHABLE_MANAGEMENT_ADDR,
            name: "agent-that-is-killed".to_owned(),
            slots: 1,
        })
        .expect("the agent subprocess must start"),
    );
    let signals = agent_process
        .signals()
        .expect("a running agent must be signallable");

    signals.kill().expect("the agent must be killed");

    assert_eq!(
        signals.pause().err().as_ref().map(ToString::to_string),
        Some(
            SubprocessClusterError::ProcessDidNotStop {
                pid: signals.pid,
                status: WaitStatus::Signaled(signals.pid, Signal::SIGKILL, false),
            }
            .to_string()
        )
    );
}
