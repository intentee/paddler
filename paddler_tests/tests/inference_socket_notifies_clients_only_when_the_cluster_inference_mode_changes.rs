use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

fn desired_state_serving(inference_mode: InferenceMode) -> BalancerDesiredState {
    BalancerDesiredState {
        inference_mode,
        ..BalancerDesiredState::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_notifies_clients_only_when_the_cluster_inference_mode_changes() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Embeddings))
        .await
        .expect("a balancer serving embeddings must start");
    let mut cluster_inference_mode_rx = cluster
        .client_inference
        .subscribe_to_cluster_inference_mode();

    cluster
        .client_inference
        .continue_from_raw_prompt(
            CancellationToken::new(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "open the inference socket".to_owned(),
            },
        )
        .await
        .expect("the inference socket must accept a request while serving embeddings")
        .next()
        .await
        .expect("the inference socket must answer while serving embeddings")
        .expect("the answer must be readable");

    let connect_notification = cluster_inference_mode_rx
        .recv()
        .await
        .expect("the client must be told on connect which inference mode the cluster serves");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                embeddings: EmbeddingParameters {
                    embedding_batch_size: NonZeroUsize::MIN,
                    ..EmbeddingParameters::default()
                },
                ..desired_state_serving(InferenceMode::Embeddings)
            },
        )
        .await
        .expect("the balancer must apply a state that keeps serving embeddings");

    let mut notification_after = async |inference_mode: InferenceMode| {
        cluster
            .client_management
            .put_balancer_desired_state(
                CancellationToken::new(),
                &desired_state_serving(inference_mode),
            )
            .await
            .expect("the balancer must apply the desired state");

        cluster_inference_mode_rx
            .recv()
            .await
            .expect("the client must receive the inference mode change")
    };

    let text_generation_notification = notification_after(InferenceMode::TextGeneration).await;
    let decision_notification = notification_after(InferenceMode::Decision).await;

    assert_eq!(
        [
            connect_notification,
            text_generation_notification,
            decision_notification,
        ],
        [
            Notification::ClusterInferenceMode(InferenceMode::Embeddings),
            Notification::ClusterInferenceMode(InferenceMode::TextGeneration),
            Notification::ClusterInferenceMode(InferenceMode::Decision),
        ]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
