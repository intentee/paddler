use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_http_client_releases_the_buffered_request() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::without_request_expiry()
    })
    .await?;

    let inference_addr = cluster.balancer.addresses.inference;
    let params: ContinueFromConversationHistoryParams<RawParametersSchema> =
        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("hi".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 2048,
            parse_tool_calls: false,
            tools: Vec::new(),
        };
    let mut client = HalfClosedClient::post_json_then_half_close(
        inference_addr,
        "/api/v1/continue_from_conversation_history",
        &params,
    )
    .await?;

    cluster
        .wait_for_buffered_request_count(1, ObservationWindow::model_load())
        .await
        .context("the request must be buffered while no agent is available")?;

    client.half_close().await?;

    cluster
        .wait_for_buffered_request_count(0, ObservationWindow::release())
        .await
        .context(
            "the balancer must notice the half-closed client and release the buffered request \
         instead of holding it until buffered_request_timeout",
        )?;

    drop(client);

    cluster.shutdown().await?;

    Ok(())
}
