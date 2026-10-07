#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

fn with_nul_byte(text: &str) -> String {
    format!("{text} \0 byte")
}

#[tokio::test(flavor = "multi_thread")]
async fn decision_input_with_a_nul_byte_is_rejected() {
    let cluster = start_decision_cluster(DecisionClusterParams::default())
        .await
        .expect("the cluster must start");
    let RawDecideParams { questions, state } = sample_decision();
    let mut nul_in_leading_instructions = questions.clone();
    let mut nul_in_last_option = questions.clone();

    nul_in_leading_instructions[0].instructions =
        with_nul_byte(&nul_in_leading_instructions[0].instructions);

    let last_question = nul_in_last_option
        .last_mut()
        .expect("the sample decision has questions");

    last_question.options[0] = with_nul_byte(&last_question.options[0]);

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

        assert!(collected.answers.is_empty());
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
