use std::error::Error as _;

use nix::errno::Errno;
use nix::sys::signal::Signal;

use paddler_cli_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_cli_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;
use paddler_cli_tests::subprocess_process::SubprocessProcess;
use paddler_cli_tests::unreachable_management_addr::UNREACHABLE_MANAGEMENT_ADDR;
use paddler_test_cluster_harness::managed_process::ManagedProcess as _;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_signals_report_a_reaped_process_that_cannot_be_signalled() {
    let agent_process = SubprocessProcess::new(
        spawn_agent_subprocess(SpawnAgentSubprocessParams {
            binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
            management_addr: UNREACHABLE_MANAGEMENT_ADDR,
            name: "agent-that-is-reaped".to_owned(),
            slots: 1,
        })
        .expect("the agent subprocess must start"),
    );
    let signals = agent_process
        .signals()
        .expect("a running agent must be signallable");

    signals.kill().expect("the agent must be killed");

    Box::new(agent_process)
        .shutdown()
        .await
        .expect("the killed agent must be reaped");

    let pause_error = signals
        .pause()
        .expect_err("a reaped process must not be signallable");

    assert_eq!(
        pause_error.to_string(),
        SubprocessClusterError::SignalUndeliverable {
            pid: signals.pid,
            signal: Signal::SIGSTOP,
            source: Errno::ESRCH,
        }
        .to_string()
    );
    assert_eq!(
        pause_error
            .source()
            .and_then(|source| source.downcast_ref::<Errno>()),
        Some(&Errno::ESRCH)
    );
}
