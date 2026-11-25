#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation_tests::gbnf_literal::gbnf_literal;
use paddler_agent_text_generation_tests::image_description_conversation::image_description_conversation;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::start_cluster_with_smolvlm2::start_cluster_with_smolvlm2;
use paddler_file_fixture::image_fixture::ImageFixture;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

const PLAIN_ANSWER: &str = "The sea is calm tonight.";
const MULTIMODAL_ANSWER: &str = "The image shows a picture.";

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_plain_and_multimodal_run_concurrently() {
    let cluster = start_cluster_with_smolvlm2(vec![AgentConfig::single(4)])
        .await
        .expect("the cluster must start");

    let image_data_uri = ImageFixture::LlamasJpeg
        .data_uri()
        .expect("the test image must load");

    let multimodal_conversation = image_description_conversation(image_data_uri);

    let plain_params = ContinueFromRawPromptParams {
        grammar: Some(gbnf_literal(PLAIN_ANSWER)),
        max_tokens: NonZeroU32::new(64).unwrap(),
        raw_prompt: "Write a long poem about the sea.".to_owned(),
    };
    let multimodal_params = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: multimodal_conversation,
        enable_thinking: ModelDefaultOr::Explicit(false),
        grammar: Some(gbnf_literal(MULTIMODAL_ANSWER)),
        max_tokens: NonZeroU32::new(32).unwrap(),
        parallel_tool_calls: ModelDefaultOr::ModelDefault,
        parse_tool_calls: false,
        tools: vec![],
    };
    let (plain_collected, multimodal_collected) = join!(
        cluster.continue_from_raw_prompt(CancellationToken::new(), &plain_params),
        cluster.continue_from_conversation_history(CancellationToken::new(), &multimodal_params),
    );

    let plain_collected = plain_collected.expect("the plain request must complete");
    let multimodal_collected = multimodal_collected.expect("the multimodal request must complete");

    assert_eq!(plain_collected.text, PLAIN_ANSWER);
    assert_eq!(
        plain_collected
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::EndOfGeneration
    );
    assert_eq!(multimodal_collected.text, MULTIMODAL_ANSWER);
    assert_eq!(
        multimodal_collected
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::EndOfGeneration
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
