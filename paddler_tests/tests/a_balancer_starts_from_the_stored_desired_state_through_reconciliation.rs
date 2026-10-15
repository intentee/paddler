use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_state_database::file::File;
use paddler_state_database::state_database::StateDatabase as _;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::state_database_file::StateDatabaseFile;
use paddler_tests::start_cluster::start_cluster;

const STORED_MODEL_URI: &str = "a stored uri that does not parse";

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_starts_from_the_stored_desired_state_through_reconciliation() {
    let database = StateDatabaseFile::new().expect("the state database file must be created");

    File::new(database.path.clone())
        .store_balancer_desired_state(&BalancerDesiredState {
            model: AgentDesiredModel::Uri(STORED_MODEL_URI.to_owned()),
            ..BalancerDesiredState::default()
        })
        .await
        .expect("the desired state must be stored before the balancer starts");

    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::KeepStored,
        state_database_url: database.url.clone(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start from its stored desired state");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ModelUriIsUnparseable(model_path) if model_path.model_path == STORED_MODEL_URI)
        })
        .await
        .expect("the agent must reconcile the stored desired state");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
