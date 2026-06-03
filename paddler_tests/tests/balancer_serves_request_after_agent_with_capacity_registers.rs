#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_client::Message;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::ContinueFromRawPromptParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_request_after_agent_with_capacity_registers() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        buffered_request_timeout: Duration::from_millis(50),
        max_buffered_requests: 10,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: gpu_layer_count,
                ..InferenceParameters::default()
            },
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        ..ClusterParams::default()
    })
    .await?;

    let mut early_stream = cluster
        .continue_from_raw_prompt_stream(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await?;

    let early_message = early_stream
        .next()
        .await
        .context("inference stream must yield a message")??;

    match early_message {
        Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 504);
        }
        Message::Response(_) => {
            anyhow::bail!("expected timeout before agent registered");
        }
    }

    cluster.spawn_additional_agent(&AgentConfig {
        name: "capacity-agent".to_owned(),
        slot_count: 4,
    })?;

    cluster
        .agents_watcher
        .until(|snapshot| {
            snapshot.agents.len() == 1 && snapshot.agents.iter().any(|agent| agent.slots_total >= 4)
        })
        .await
        .context("agent should register with 4 slots")?;

    let mut later_stream = cluster
        .continue_from_raw_prompt_stream(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await?;

    let later_message = later_stream
        .next()
        .await
        .context("inference stream must yield a message")??;

    match later_message {
        Message::Error(envelope) => {
            assert_ne!(
                envelope.error.code, 503,
                "request should not overflow after agent registered with capacity"
            );
        }
        Message::Response(_) => {}
    }

    cluster.shutdown().await?;

    Ok(())
}
