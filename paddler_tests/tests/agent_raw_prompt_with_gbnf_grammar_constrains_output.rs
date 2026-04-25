#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_types::grammar_constraint::GrammarConstraint;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn agent_raw_prompt_with_gbnf_grammar_constrains_output() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(2, 1).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: r#"root ::= "yes" | "no""#.to_owned(),
                root: "root".to_owned(),
            }),
            max_tokens: 10,
            raw_prompt:
                "<|im_start|>user\nIs the sky blue? Answer yes or no.<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
                    .to_owned(),
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    assert!(
        collected.text == "yes" || collected.text == "no",
        "GBNF grammar should constrain output to yes/no; got {:?}",
        collected.text
    );

    cluster.shutdown().await?;

    Ok(())
}
