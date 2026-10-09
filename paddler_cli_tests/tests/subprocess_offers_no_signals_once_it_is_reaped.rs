use std::net::SocketAddr;

use paddler_cli_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_cli_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_cli_tests::subprocess_process::SubprocessProcess;

const UNREACHABLE_MANAGEMENT_ADDR: &str = "127.0.0.1:1";

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_offers_no_signals_once_it_is_reaped() {
    let mut agent = spawn_agent_subprocess(SpawnAgentSubprocessParams {
        binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node").to_owned(),
        management_addr: UNREACHABLE_MANAGEMENT_ADDR
            .parse::<SocketAddr>()
            .expect("the unreachable address is valid"),
        name: "agent-that-is-reaped-first".to_owned(),
        slots: 1,
    })
    .expect("the agent subprocess must start");

    agent
        .kill()
        .await
        .expect("the agent must be killed and reaped");

    assert!(SubprocessProcess::new(agent).signals().is_none());
}
