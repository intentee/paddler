use paddler_balancer::state_database::StateDatabase as _;
use paddler_balancer::state_database::file::File as StateDatabaseFile;
use paddler_balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::inference_parameters::InferenceParameters;
use tempfile::NamedTempFile;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_preserves_the_persisted_desired_state() {
    let state_database_file =
        NamedTempFile::new().expect("a temporary state database file must be creatable");
    let persisted_state = BalancerDesiredState {
        chat_template_override: Some(ChatTemplate {
            content: "persisted-chat-template".to_owned(),
        }),
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::LocalToAgent("persisted-model".to_owned()),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: true,
    };
    let (balancer_desired_state_tx, _balancer_desired_state_rx) =
        watch::channel(BalancerDesiredState::default());
    let state_database = StateDatabaseFile::new(
        balancer_desired_state_tx,
        state_database_file.path().to_path_buf(),
    );

    state_database
        .store_balancer_desired_state(&persisted_state)
        .await
        .expect("the desired state must be persisted before the runner starts");

    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.state_database_type = StateDatabaseType::File(state_database_file.path().to_path_buf());

    let mut runner = BalancerRunner::start(params)
        .await
        .expect("a runner with a valid state database must start");

    assert_eq!(*runner.balancer_desired_state_tx.borrow(), persisted_state);

    runner.cancel();
    runner
        .wait_for_completion()
        .await
        .expect("a cancelled runner must complete cleanly");

    assert_eq!(
        state_database
            .read_balancer_desired_state()
            .await
            .expect("the state database must stay readable after the runner stops"),
        persisted_state
    );
}
