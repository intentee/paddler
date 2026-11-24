#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::weather_answer_classification_failing_after_reads::WeatherAnswerClassificationFailingAfterReads;

const READS_BEFORE_THE_GENERATED_TOKEN: usize = 1;

#[test]
fn a_generated_token_llama_cpp_cannot_classify_ends_the_generation() {
    assert!(matches!(
        WeatherAnswerClassificationFailingAfterReads {
            reads_that_succeed: READS_BEFORE_THE_GENERATED_TOKEN,
        }
        .single_token_generation_outcome()
        .expect("the weather answer must reach its classification"),
        ContinuousBatchTerminalOutcome::EmitToClient(
            GeneratedTokenResult::AnswerClassificationFailed(_)
        )
    ));
}
