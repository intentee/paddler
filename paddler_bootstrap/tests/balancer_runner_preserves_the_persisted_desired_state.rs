use tempfile::TempDir;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_state_database::file::File as StateDatabaseFile;
use paddler_state_database::state_database::StateDatabase as _;
use paddler_state_database::state_database_type::StateDatabaseType;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_preserves_the_persisted_desired_state() {
    let state_database_directory =
        TempDir::new().expect("a temporary state database directory must be creatable");
    let state_database_path = state_database_directory.path().join("state.json");
    let persisted_state = BalancerDesiredState {
        model: AgentDesiredModel::Uri("persisted-model".to_owned()),
        text_generation: BalancerTextGenerationSettings {
            chat_template_override: Some(ChatTemplate {
                content: "persisted-chat-template".to_owned(),
            }),
            use_chat_template_override: true,
            ..BalancerTextGenerationSettings::default()
        },
        ..BalancerDesiredState::default()
    };
    let (balancer_desired_state_tx, _balancer_desired_state_rx) =
        watch::channel(BalancerDesiredState::default());
    let state_database =
        StateDatabaseFile::new(balancer_desired_state_tx, state_database_path.clone());

    state_database
        .store_balancer_desired_state(&persisted_state)
        .await
        .expect("the desired state must be persisted before the runner starts");

    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.bootstrap_config.state_database_type = StateDatabaseType::File(state_database_path);

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
