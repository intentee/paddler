#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_model_card_fixture::laya::laya;
use paddler_request_params_fixture::weather_and_season_decision::weather_and_season_decision;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_of_several_questions_for_a_model_sharing_no_prompt_prefix_is_answered_by_one_slot()
 {
    let cluster = DecisionClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: DecisionDesiredState {
            model_card: laya(),
            ..DecisionDesiredState::default()
        },
        ..DecisionClusterParams::default()
    }
    .start_cluster()
    .await
    .expect("a cluster serving a model sharing no prompt prefix on one slot must start");
    let collected = cluster
        .decide(CancellationToken::new(), &weather_and_season_decision())
        .await
        .expect("the decision must be answered by the agent's only slot");

    let mut answered_question_ids = collected
        .answers
        .iter()
        .map(|AnsweredDecisionQuestion { id, .. }| id.clone())
        .collect::<Vec<_>>();

    answered_question_ids.sort();

    assert_eq!(
        answered_question_ids,
        vec!["season".to_owned(), "weather".to_owned()]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
