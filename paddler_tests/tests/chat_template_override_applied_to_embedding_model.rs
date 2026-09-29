#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::nomic_embed_desired_state_with_chat_template_override::nomic_embed_desired_state_with_chat_template_override;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn chat_template_override_applied_to_embedding_model() -> Result<()> {
    let chat_template = ChatTemplate {
        content: "{{ messages[0].content }}".to_owned(),
    };

    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: false,
        desired_state: Some(nomic_embed_desired_state_with_chat_template_override(
            chat_template.clone(),
        )),
        ..ClusterParams::default()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let retrieved = cluster
        .client_management
        .get_chat_template_override(CancellationToken::new(), &agent_id)
        .await
        .map_err(anyhow::Error::new)
        .context("failed to read chat template override")?;

    assert_eq!(retrieved, Some(chat_template));

    cluster.shutdown().await?;

    Ok(())
}
