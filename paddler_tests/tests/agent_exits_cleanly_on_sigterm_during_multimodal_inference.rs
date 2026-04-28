#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::agents_status::AgentsStatus;
use paddler_tests::current_test_device::current_test_device;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen2_5_vl_3b::qwen2_5_vl_3b;
use paddler_tests::model_card::qwen2_5_vl_3b_mmproj::qwen2_5_vl_3b_mmproj;
use paddler_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;
use paddler_tests::terminate_child::terminate_child;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::image_url::ImageUrl;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_exits_cleanly_on_sigterm_during_multimodal_inference() -> Result<()> {
    let device = current_test_device()?;

    device.require_available()?;

    let ModelCard {
        gpu_layer_count,
        reference: primary_reference,
    } = qwen2_5_vl_3b();
    let ModelCard {
        reference: mmproj_reference,
        ..
    } = qwen2_5_vl_3b_mmproj();

    let mut cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: device.inference_parameters_for_full_offload(gpu_layer_count),
            model: AgentDesiredModel::HuggingFace(primary_reference),
            multimodal_projection: AgentDesiredModel::HuggingFace(mmproj_reference),
            use_chat_template_override: false,
        }),
        ..SubprocessClusterParams::default()
    })
    .await?;

    let mut agent_child = spawn_agent_subprocess(SpawnAgentSubprocessParams {
        management_addr: cluster.addresses.management,
        name: Some("multimodal-shutdown-agent".to_owned()),
        slots: 2,
    })?;

    let snapshot = cluster
        .agents
        .until(|snapshot| {
            snapshot.agents.len() == 1 && snapshot.agents.iter().any(|agent| agent.slots_total >= 2)
        })
        .await
        .context("agent should register with slots ready")?;

    let agent_id = snapshot
        .agents
        .first()
        .context("registered agent must be present in snapshot")?
        .id
        .clone();

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let image_data_uri = load_test_image_data_uri()?;

    let mut stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: image_data_uri,
                        },
                    },
                    ConversationMessageContentPart::Text {
                        text: "Describe this image in detail".to_owned(),
                    },
                ]),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 200,
            tools: vec![],
        })
        .await?;

    let _first_message = stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("multimodal stream must yield at least one message"))?;

    cluster
        .agents
        .until(AgentsStatus::slots_processing_is(&agent_id, 1))
        .await?;

    terminate_child(&mut agent_child)?;
    let exit_status = agent_child.wait().await?;

    cluster
        .agents
        .until(AgentsStatus::agent_count_is(0))
        .await?;

    drop(stream);

    cluster.shutdown().await?;

    assert!(
        exit_status.code().is_some() || exit_status.success(),
        "agent must exit cleanly during multimodal inference; got {exit_status:?}"
    );

    Ok(())
}
