#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_504_when_no_agents_registered() -> Result<()> {
    let ModelCard { reference, .. } = qwen3_0_6b();

    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        buffered_request_timeout: Duration::from_millis(50),
        max_buffered_requests: 1,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..ClusterParams::default()
    })
    .await?;

    let mut stream = cluster
        .continue_from_raw_prompt_stream(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await?;

    let message = stream
        .next()
        .await
        .context("inference stream must yield a message")??;

    match message {
        Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 504);
        }
        Message::Response(_) => {
            anyhow::bail!("expected an error response, got success");
        }
        Message::Notification(_) => {
            anyhow::bail!("unexpected token-generation-mode notification");
        }
    }

    cluster.shutdown().await?;

    Ok(())
}
