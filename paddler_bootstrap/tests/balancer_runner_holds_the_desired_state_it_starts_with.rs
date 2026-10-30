use tokio_util::sync::CancellationToken;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_state_database::state_database_type::StateDatabaseType;
use paddler_state_database::stored_balancer_desired_state::StoredBalancerDesiredState;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_holds_the_desired_state_it_starts_with() {
    let initial_desired_state = BalancerDesiredState {
        decision: DecisionSettings {
            pointer_head: AgentDesiredModel::Uri("initial-pointer-head".to_owned()),
        },
        model: AgentDesiredModel::Uri("initial-embedding-model".to_owned()),
        ..BalancerDesiredState::default()
    };
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.bootstrap_config.state_database_type = StateDatabaseType::Memory(
        StoredBalancerDesiredState::Stored(Box::new(initial_desired_state.clone())),
    );

    let mut runner = BalancerRunner::start(params)
        .await
        .expect("a runner starting with a desired state must start");

    assert_eq!(
        runner
            .balancer_applicable_state_holder
            .get_balancer_desired_state(),
        initial_desired_state
    );

    runner.cancel();
    runner
        .wait_for_completion()
        .await
        .expect("a cancelled runner must complete cleanly");
}
