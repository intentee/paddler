#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;
use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_request_params_fixture::sample_decision::sample_decision;
use paddler_request_params_fixture::with_nul_byte::with_nul_byte;

#[tokio::test(flavor = "multi_thread")]
async fn decision_input_with_a_nul_byte_is_rejected() {
    let cluster = DecisionClusterParams::default()
        .start_cluster()
        .await
        .expect("the cluster must start");
    let RawDecideParams { questions, state } = sample_decision();
    let mut nul_in_leading_instructions = questions.clone();
    let mut nul_in_last_option = questions.clone();

    let leading_question = nul_in_leading_instructions
        .first_mut()
        .expect("the sample decision has questions");

    leading_question.instructions = with_nul_byte(&leading_question.instructions);

    let last_question = nul_in_last_option
        .last_mut()
        .expect("the sample decision has questions");
    let last_option = last_question
        .options
        .first_mut()
        .expect("the last question of the sample decision has options");

    *last_option = with_nul_byte(last_option);

    for decision in [
        RawDecideParams {
            questions: questions.clone(),
            state: with_nul_byte(&state),
        },
        RawDecideParams {
            questions: nul_in_leading_instructions,
            state: state.clone(),
        },
        RawDecideParams {
            questions: nul_in_last_option,
            state: state.clone(),
        },
    ] {
        let collected = cluster
            .decide(CancellationToken::new(), &decision)
            .await
            .expect("the decision must be answered with a rejection");

        assert_eq!(collected.answers, Vec::<DecisionAnswer>::new());
        assert!(matches!(
            collected.terminal_result,
            DecisionResult::InputTokenizationFailed(_)
        ));
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
