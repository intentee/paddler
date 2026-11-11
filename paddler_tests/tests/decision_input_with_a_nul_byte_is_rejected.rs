#![cfg(feature = "tests_that_use_llms")]

use std::ffi::CString;

use llama_cpp_bindings::error::string_to_token_error::StringToTokenError;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::with_nul_byte::with_nul_byte;

struct DecisionRejectedForANulByte {
    decision: RawDecideParams,
    text_with_nul_byte: String,
}

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

    let leading_instructions_with_nul_byte = leading_question.instructions.clone();
    let last_option_with_nul_byte = last_option.clone();

    for DecisionRejectedForANulByte {
        decision,
        text_with_nul_byte,
    } in [
        DecisionRejectedForANulByte {
            decision: RawDecideParams {
                questions: questions.clone(),
                state: with_nul_byte(&state),
            },
            text_with_nul_byte: with_nul_byte(&state),
        },
        DecisionRejectedForANulByte {
            decision: RawDecideParams {
                questions: nul_in_leading_instructions,
                state: state.clone(),
            },
            text_with_nul_byte: leading_instructions_with_nul_byte,
        },
        DecisionRejectedForANulByte {
            decision: RawDecideParams {
                questions: nul_in_last_option,
                state: state.clone(),
            },
            text_with_nul_byte: last_option_with_nul_byte,
        },
    ] {
        let collected = cluster
            .decide(CancellationToken::new(), &decision)
            .await
            .expect("the decision must be answered with a rejection");
        let nul_error =
            CString::new(text_with_nul_byte).expect_err("the text must contain a nul byte");

        assert_eq!(collected.answers, Vec::<DecisionAnswer>::new());
        assert_eq!(
            collected.terminal_result,
            DecisionResult::InputTokenizationFailed(rejection_description(
                Some("test-agent-0"),
                &DecisionError::InputTokenizationFailed(StringToTokenError::NulError(nul_error))
            ))
        );
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
