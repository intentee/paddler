use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_client::format_api_url::format_api_url;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

fn is_cancelled_request_to(
    request_error: Option<&ClusterHarnessError>,
    expected_url: &str,
) -> bool {
    matches!(
        request_error,
        Some(ClusterHarnessError::InferenceRequestFailed(ClientError::RequestCancelled { url }))
            if url == expected_url
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn cluster_harness_reports_streamed_requests_cancelled_before_they_are_sent() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let inference_base_url = &cluster.balancer.inference_base_url;
    let cancelled_requests = CancellationToken::new();

    cancelled_requests.cancel();

    let raw_prompt_stream_error = cluster
        .continue_from_raw_prompt_stream(
            cancelled_requests.clone(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "Paddler".to_owned(),
            },
        )
        .await
        .err();
    let conversation_history_stream_error = cluster
        .continue_from_conversation_history_stream(
            cancelled_requests,
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(Vec::new()),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: false,
                tools: Vec::new(),
            },
        )
        .await
        .err();

    assert!(
        is_cancelled_request_to(
            raw_prompt_stream_error.as_ref(),
            &format_api_url(inference_base_url, ApiPath::CONTINUE_FROM_RAW_PROMPT)
        ),
        "{raw_prompt_stream_error:?}"
    );
    assert!(
        is_cancelled_request_to(
            conversation_history_stream_error.as_ref(),
            &format_api_url(
                inference_base_url,
                ApiPath::CONTINUE_FROM_CONVERSATION_HISTORY
            )
        ),
        "{conversation_history_stream_error:?}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
