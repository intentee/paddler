use libc::EPERM;
use libc::SYS_kill;
use nix::errno::Errno;
use nix::sys::signal::Signal;

use paddler_cli_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_cli_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;
use paddler_cli_tests::subprocess_process::SubprocessProcess;
use paddler_cli_tests::unreachable_management_addr::UNREACHABLE_MANAGEMENT_ADDR;
use paddler_test_cluster_harness::managed_process::ManagedProcess;

use crate::refused_syscall::RefusedSyscall;
use crate::refused_syscalls::RefusedSyscalls;

const SLOTS_THE_AGENT_REJECTS_ON_ITS_OWN: u16 = 0;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_shutdown_reports_a_termination_it_cannot_signal() {
    let agent_process = SubprocessProcess::new(
        spawn_agent_subprocess(SpawnAgentSubprocessParams {
            binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
            management_addr: UNREACHABLE_MANAGEMENT_ADDR,
            name: "agent-that-cannot-be-terminated".to_owned(),
            slots: SLOTS_THE_AGENT_REJECTS_ON_ITS_OWN,
        })
        .expect("the agent subprocess must start"),
    );
    let agent_pid = agent_process
        .signals()
        .expect("a running agent must be signallable")
        .pid;

    RefusedSyscalls {
        errno: EPERM,
        refused_syscalls: vec![RefusedSyscall {
            syscall: SYS_kill,
            rules: vec![],
        }],
    }
    .apply_to_every_thread();

    let shutdown_error = <dyn ManagedProcess>::shutdown(Box::new(agent_process))
        .await
        .expect_err("an agent that cannot be signalled must not shut down");

    assert_eq!(
        shutdown_error.to_string(),
        SubprocessClusterError::SignalUndeliverable {
            pid: agent_pid,
            signal: Signal::SIGTERM,
            source: Errno::EPERM,
        }
        .to_string()
    );
    assert_eq!(
        shutdown_error
            .source()
            .and_then(|source| source.downcast_ref::<Errno>()),
        Some(&Errno::EPERM)
    );
}
