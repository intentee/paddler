use paddler_cli_tests::signalled_subprocess::SignalledSubprocess;
use paddler_cli_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_cli_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;
use paddler_cli_tests::unreachable_management_addr::UNREACHABLE_MANAGEMENT_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn signalling_a_subprocess_refuses_one_already_reaped() {
    let mut agent = spawn_agent_subprocess(SpawnAgentSubprocessParams {
        binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
        management_addr: UNREACHABLE_MANAGEMENT_ADDR,
        name: "agent-that-is-reaped-first".to_owned(),
        slots: 1,
    })
    .expect("the agent subprocess must start");

    agent
        .kill()
        .await
        .expect("the agent must be killed and reaped");

    assert_eq!(
        SignalledSubprocess::new(agent)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(SubprocessClusterError::ProcessAlreadyReaped.to_string())
    );
}
