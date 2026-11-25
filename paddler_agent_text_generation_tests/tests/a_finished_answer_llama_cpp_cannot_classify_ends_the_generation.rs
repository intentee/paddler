#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation_tests::weather_answer_classification_failing_after_reads::WeatherAnswerClassificationFailingAfterReads;
use paddler_messaging::generated_token_result::GeneratedTokenResult;

const READS_BEFORE_THE_FINISHED_ANSWER: usize = 2;

#[test]
fn a_finished_answer_llama_cpp_cannot_classify_ends_the_generation() {
    assert!(matches!(
        WeatherAnswerClassificationFailingAfterReads {
            reads_that_succeed: READS_BEFORE_THE_FINISHED_ANSWER,
        }
        .single_token_generation_outcome()
        .expect("the weather answer must reach its classification"),
        ContinuousBatchTerminalOutcome::EmitToClient(
            GeneratedTokenResult::AnswerClassificationFailed(_)
        )
    ));
}
