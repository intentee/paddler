#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::chat_template::ChatTemplate;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::nomic_embed_desired_state_with_chat_template_override::nomic_embed_desired_state_with_chat_template_override;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn chat_template_override_applied_to_embedding_model() {
    let chat_template = ChatTemplate {
        content: "{{ messages[0].content }}".to_owned(),
    };

    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: false,
        desired_state: ClusterDesiredState::Apply(Box::new(
            nomic_embed_desired_state_with_chat_template_override(chat_template.clone())
                .expect("a text generation state must accept a chat template override"),
        )),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    cluster
        .wait_for_chat_template_override_in_use(&agent_id)
        .await
        .expect("the agent must use the chat template override");

    let retrieved = cluster
        .client_management
        .get_chat_template_override(CancellationToken::new(), &agent_id)
        .await
        .expect("failed to read chat template override");

    assert_eq!(retrieved, Some(chat_template));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
