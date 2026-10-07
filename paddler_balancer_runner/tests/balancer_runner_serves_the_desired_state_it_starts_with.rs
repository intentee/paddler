use tokio_util::sync::CancellationToken;

use paddler_balancer_runner::balancer_runner::BalancerRunner;
use paddler_balancer_runner::balancer_serving_mode::BalancerServingMode;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_state_database::state_database_type::StateDatabaseType;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_serves_the_desired_state_it_starts_with() {
    let initial_desired_state = BalancerDesiredState {
        model: AgentDesiredModel::LocalToAgent("initial-embedding-model".to_owned()),
        ..BalancerDesiredState::unconfigured(InferenceMode::Embeddings)
    };
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.runner_config.serving_mode = BalancerServingMode::Embeddings;
    params.runner_config.state_database_type =
        StateDatabaseType::MemoryStartingWith(Box::new(initial_desired_state.clone()));

    let runner = BalancerRunner::start(params)
        .await
        .expect("a runner starting with a desired state of its own mode must start");

    assert_eq!(
        *runner.balancer_desired_state_tx.borrow(),
        initial_desired_state
    );

    runner.cancel();
    runner
        .wait_for_completion()
        .await
        .expect("a cancelled runner must complete cleanly");
}
