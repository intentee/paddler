#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::pin::Pin;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::Stream;
use futures_util::StreamExt;
use paddler_client::Result as ClientResult;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer_params::ManagedBalancerParams;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_integration_tests::pick_free_port::pick_free_port;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::grammar_constraint::GrammarConstraint;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serial_test::file_serial;
use tempfile::NamedTempFile;

struct CollectedTokens {
    text: String,
    count: usize,
    has_grammar_incompatible_with_thinking: bool,
}

async fn collect_tokens_from_websocket_stream(
    mut stream: Pin<Box<dyn Stream<Item = ClientResult<Message>> + Send>>,
) -> Result<CollectedTokens> {
    let mut text = String::new();
    let mut count = 0;
    let mut has_grammar_incompatible_with_thinking = false;

    while let Some(message) = stream.next().await {
        let message = message.context("message should deserialize")?;

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(GeneratedTokenResult::Token(token)) => {
                    text.push_str(&token);
                    count += 1;
                }
                Response::GeneratedToken(GeneratedTokenResult::Done) => break,
                Response::GeneratedToken(
                    GeneratedTokenResult::GrammarIncompatibleWithThinking(_),
                ) => {
                    has_grammar_incompatible_with_thinking = true;

                    break;
                }
                Response::GeneratedToken(GeneratedTokenResult::GrammarRejectedModelOutput(_)) => {
                    break;
                }
                other => return Err(anyhow!("unexpected response: {other:?}")),
            },
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "unexpected error: {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    Ok(CollectedTokens {
        text,
        count,
        has_grammar_incompatible_with_thinking,
    })
}

fn collect_tokens_from_ndjson_body(body: &str) -> Result<CollectedTokens> {
    let mut text = String::new();
    let mut count = 0;
    let mut has_grammar_incompatible_with_thinking = false;

    for line in body.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let message: Message = serde_json::from_str(line)
            .context("each line should be valid inference message JSON")?;

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(GeneratedTokenResult::Token(token)) => {
                    text.push_str(&token);
                    count += 1;
                }
                Response::GeneratedToken(GeneratedTokenResult::Done) => break,
                Response::GeneratedToken(
                    GeneratedTokenResult::GrammarIncompatibleWithThinking(_),
                ) => {
                    has_grammar_incompatible_with_thinking = true;

                    break;
                }
                Response::GeneratedToken(GeneratedTokenResult::GrammarRejectedModelOutput(_)) => {
                    break;
                }
                other => return Err(anyhow!("unexpected response: {other:?}")),
            },
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "unexpected error: {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    Ok(CollectedTokens {
        text,
        count,
        has_grammar_incompatible_with_thinking,
    })
}

#[tokio::test]
#[file_serial]
async fn test_inference_health_endpoint() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: format!("127.0.0.1:{}", pick_free_port().context("pick port")?),
        inference_addr: format!("127.0.0.1:{}", pick_free_port().context("pick port")?),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: format!("127.0.0.1:{}", pick_free_port().context("pick port")?),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: state_db_url,
    })
    .await
    .context("failed to spawn balancer")?;

    let response = balancer
        .client()
        .inference()
        .get_health()
        .await
        .context("health request should succeed")?;

    assert_eq!(response, "OK");

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_continue_from_raw_prompt() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "inference-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "The capital of France is".to_owned(),
        })
        .await
        .context("raw prompt request should succeed")?;

    let collected = collect_tokens_from_websocket_stream(stream).await?;

    assert!(
        !collected.text.is_empty(),
        "should have received at least one token"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_continue_from_conversation_history() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "inference-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let params = ContinueFromConversationHistoryParams::<ValidatedParametersSchema> {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![ConversationMessage {
            content: ConversationMessageContent::Text("Say hello".to_owned()),
            role: "user".to_owned(),
        }]),
        enable_thinking: true,
        grammar: None,
        max_tokens: 50,
        tools: vec![],
    };

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!(
            "http://{}/api/v1/continue_from_conversation_history",
            cluster.balancer.inference_addr()
        ))
        .json(&params)
        .send()
        .await
        .context("conversation history request should succeed")?;

    assert_eq!(response.status(), 200);

    let body = response.text().await.context("should read response body")?;
    let collected = collect_tokens_from_ndjson_body(&body)?;

    assert!(
        !collected.text.is_empty(),
        "should have received at least one token"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_raw_prompt_respects_max_tokens() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "inference-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let max_tokens = 20;

    let stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens,
            raw_prompt: "Once upon a time in a land far far away there lived".to_owned(),
        })
        .await
        .context("raw prompt request should succeed")?;

    let collected = collect_tokens_from_websocket_stream(stream).await?;

    assert!(
        collected.count > 0,
        "should have received at least one token"
    );
    let max_tokens_usize =
        usize::try_from(max_tokens).context("max_tokens must be non-negative")?;
    assert!(
        collected.count <= max_tokens_usize,
        "received {} tokens, expected at most {max_tokens}",
        collected.count
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_conversation_history_respects_max_tokens() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "inference-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let max_tokens = 20;

    let params = ContinueFromConversationHistoryParams::<ValidatedParametersSchema> {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![ConversationMessage {
            content: ConversationMessageContent::Text(
                "Tell me a long story about a dragon".to_owned(),
            ),
            role: "user".to_owned(),
        }]),
        enable_thinking: true,
        grammar: None,
        max_tokens,
        tools: vec![],
    };

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!(
            "http://{}/api/v1/continue_from_conversation_history",
            cluster.balancer.inference_addr()
        ))
        .json(&params)
        .send()
        .await
        .context("conversation history request should succeed")?;

    assert_eq!(response.status(), 200);

    let body = response.text().await.context("should read response body")?;
    let collected = collect_tokens_from_ndjson_body(&body)?;

    assert!(
        !collected.text.is_empty(),
        "should have received at least one token"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_openai_chat_completions_non_streaming() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "openai-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!("{}/v1/chat/completions", cluster.openai_base_url))
        .json(&serde_json::json!({
            "model": "test",
            "messages": [
                {
                    "role": "user",
                    "content": "Say hello"
                }
            ],
            "max_completion_tokens": 10,
            "stream": false
        }))
        .send()
        .await
        .context("openai request should succeed")?;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response
        .json()
        .await
        .context("should parse json response")?;

    assert_eq!(body["object"], "chat.completion");
    assert!(body["choices"].is_array(), "should have choices array");
    assert!(
        !body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .is_empty(),
        "response content should not be empty"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_openai_chat_completions_streaming() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "openai-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!("{}/v1/chat/completions", cluster.openai_base_url))
        .json(&serde_json::json!({
            "model": "test",
            "messages": [
                {
                    "role": "user",
                    "content": "Say hello"
                }
            ],
            "max_completion_tokens": 10,
            "stream": true
        }))
        .send()
        .await
        .context("openai streaming request should succeed")?;

    assert_eq!(response.status(), 200);

    let body = response.text().await.context("should read response body")?;
    let chunks: Vec<serde_json::Value> = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("each line should be valid JSON"))
        .collect::<Result<_>>()?;

    assert!(!chunks.is_empty(), "should have received streaming chunks");
    assert_eq!(chunks[0]["object"], "chat.completion.chunk");

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_openai_health_endpoint() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );

    let compat_openai_addr = format!("127.0.0.1:{}", pick_free_port().context("pick port")?);

    let _balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: compat_openai_addr.clone(),
        inference_addr: format!("127.0.0.1:{}", pick_free_port().context("pick port")?),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: format!("127.0.0.1:{}", pick_free_port().context("pick port")?),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: state_db_url,
    })
    .await
    .context("failed to spawn balancer")?;

    let response = reqwest::get(format!("http://{compat_openai_addr}/health"))
        .await
        .context("openai health request should succeed")?;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.context("should read body")?, "OK");

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_conversation_history_with_gbnf_grammar() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "grammar-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let params = ContinueFromConversationHistoryParams::<ValidatedParametersSchema> {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![ConversationMessage {
            content: ConversationMessageContent::Text(
                "Is the sky blue? Answer with exactly yes or no.".to_owned(),
            ),
            role: "user".to_owned(),
        }]),
        enable_thinking: false,
        grammar: Some(GrammarConstraint::Gbnf {
            grammar: "root ::= [Yy] [Ee] [Ss] | [Nn] [Oo]".to_owned(),
            root: "root".to_owned(),
        }),
        max_tokens: 200,
        tools: vec![],
    };

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!(
            "http://{}/api/v1/continue_from_conversation_history",
            cluster.balancer.inference_addr()
        ))
        .json(&params)
        .send()
        .await
        .context("gbnf grammar request should succeed")?;

    assert_eq!(response.status(), 200);

    let body = response.text().await.context("should read response body")?;

    let collected = collect_tokens_from_ndjson_body(&body)?;
    let lowercase = collected.text.to_lowercase();

    assert!(
        lowercase == "yes" || lowercase == "no",
        "GBNF grammar should constrain output to 'yes' or 'no', got: '{}'",
        collected.text
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_conversation_history_with_json_schema_grammar() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "grammar-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let params = ContinueFromConversationHistoryParams::<ValidatedParametersSchema> {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![ConversationMessage {
            content: ConversationMessageContent::Text("What is 2+2?".to_owned()),
            role: "user".to_owned(),
        }]),
        enable_thinking: false,
        grammar: Some(GrammarConstraint::JsonSchema {
            schema: r#"{"type": "object", "properties": {"answer": {"type": "string"}}, "required": ["answer"]}"#.to_owned(),
        }),
        max_tokens: 200,
        tools: vec![],
    };

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!(
            "http://{}/api/v1/continue_from_conversation_history",
            cluster.balancer.inference_addr()
        ))
        .json(&params)
        .send()
        .await
        .context("json schema grammar request should succeed")?;

    assert_eq!(response.status(), 200);

    let body = response.text().await.context("should read response body")?;
    let collected = collect_tokens_from_ndjson_body(&body)?;

    let parsed: serde_json::Value = serde_json::from_str(&collected.text).with_context(|| {
        format!(
            "JSON schema grammar should produce valid JSON, got '{}'",
            collected.text
        )
    })?;

    assert!(
        parsed.get("answer").is_some(),
        "JSON schema grammar should produce object with 'answer' field, got: '{}'",
        collected.text
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_raw_prompt_with_gbnf_grammar_small_model() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "small-grammar-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: r#"root ::= "yes" | "no""#.to_owned(),
                root: "root".to_owned(),
            }),
            max_tokens: 10,
            raw_prompt: "<|im_start|>user\nIs the sky blue?<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n".to_owned(),
        })
        .await
        .context("grammar request should succeed")?;

    let collected = collect_tokens_from_websocket_stream(stream).await?;

    assert!(
        collected.text == "yes" || collected.text == "no",
        "GBNF grammar should constrain output to 'yes' or 'no', got: '{}'",
        collected.text
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_grammar_with_thinking_returns_incompatible_error() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "thinking-grammar-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let params = ContinueFromConversationHistoryParams::<ValidatedParametersSchema> {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![ConversationMessage {
            content: ConversationMessageContent::Text("What is 2+2?".to_owned()),
            role: "user".to_owned(),
        }]),
        enable_thinking: true,
        grammar: Some(GrammarConstraint::JsonSchema {
            schema: r#"{"type": "object", "properties": {"answer": {"type": "string"}}, "required": ["answer"]}"#.to_owned(),
        }),
        max_tokens: 50,
        tools: vec![],
    };

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!(
            "http://{}/api/v1/continue_from_conversation_history",
            cluster.balancer.inference_addr()
        ))
        .json(&params)
        .send()
        .await
        .context("request should succeed")?;

    assert_eq!(response.status(), 200);

    let body = response.text().await.context("should read response body")?;
    let collected = collect_tokens_from_ndjson_body(&body)?;

    assert!(
        collected.has_grammar_incompatible_with_thinking,
        "Expected GrammarIncompatibleWithThinking error when using grammar with thinking enabled, got text: '{}'",
        collected.text
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_raw_prompt_without_grammar_field_is_backwards_compatible() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "compat-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!(
            "http://{}/api/v1/continue_from_raw_prompt",
            cluster.balancer.inference_addr()
        ))
        .json(&serde_json::json!({
            "max_tokens": 10,
            "raw_prompt": "Hello"
        }))
        .send()
        .await
        .context("request without grammar field should succeed")?;

    assert_eq!(response.status(), 200);

    let body = response.text().await.context("should read response body")?;
    let collected = collect_tokens_from_ndjson_body(&body)?;

    assert!(
        !collected.text.is_empty(),
        "should have received at least one token"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_conversation_history_without_grammar_field_is_backwards_compatible() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "compat-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!(
            "http://{}/api/v1/continue_from_conversation_history",
            cluster.balancer.inference_addr()
        ))
        .json(&serde_json::json!({
            "add_generation_prompt": true,
            "conversation_history": [
                {"content": "Say hello", "role": "user"}
            ],
            "enable_thinking": false,
            "max_tokens": 10,
            "tools": []
        }))
        .send()
        .await
        .context("request without grammar field should succeed")?;

    assert_eq!(response.status(), 200);

    let body = response.text().await.context("should read response body")?;
    let collected = collect_tokens_from_ndjson_body(&body)?;

    assert!(
        !collected.text.is_empty(),
        "should have received at least one token"
    );

    Ok(())
}
