#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;
use paddler::request_params::ContinueFromRawPromptParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_rejects_second_request_when_only_slot_busy() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 1,
        }],
        max_buffered_requests: 0,
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
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let mut first_stream = cluster
        .continue_from_raw_prompt_stream(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 100,
            raw_prompt: "Tell me a long story about an explorer".to_owned(),
        })
        .await?;

    let _first_message = first_stream
        .next()
        .await
        .context("first stream must yield at least one message")?;

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .context("first request should occupy the only slot")?;

    let second_failed = cluster
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await
        .is_err();

    assert!(
        second_failed,
        "second request must be rejected when the only slot is busy and buffering is disabled"
    );

    drop(first_stream);

    cluster.shutdown().await?;

    Ok(())
}
