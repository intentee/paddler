#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::grammar_constraint::GrammarConstraint;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_conversation_with_gbnf_grammar_constrains_output() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(2, 1).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text(
                    "Is the sky blue? Answer yes or no.".to_owned(),
                ),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: r"root ::= [Yy][Ee][Ss] | [Nn][Oo]".to_owned(),
                root: "root".to_owned(),
            }),
            max_tokens: 10,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let lower = collected.text.to_lowercase();

    assert!(
        lower == "yes" || lower == "no",
        "GBNF grammar should constrain output to yes/no; got {:?}",
        collected.text
    );

    cluster.shutdown().await?;

    Ok(())
}
