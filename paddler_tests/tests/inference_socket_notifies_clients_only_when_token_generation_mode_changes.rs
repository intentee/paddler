use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

fn embeddings_state(temperature: f32) -> BalancerDesiredState {
    BalancerDesiredState {
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            temperature,
            ..InferenceParameters::default()
        },
        ..BalancerDesiredState::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_notifies_clients_only_when_token_generation_mode_changes() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        desired_state: Some(embeddings_state(0.5)),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer serving embeddings must start");
    let mut token_generation_mode_rx = cluster
        .client_inference
        .subscribe_to_token_generation_mode();

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
        .expect("the inference socket must accept a request in embeddings mode")
        .next()
        .await
        .expect("the inference socket must answer in embeddings mode")
        .expect("the embeddings-mode answer must be readable");

    let connect_notification = token_generation_mode_rx
        .recv()
        .await
        .expect("the client must be told on connect that token generation is disabled");

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &embeddings_state(0.7))
        .await
        .expect("the balancer must apply a state that keeps embeddings enabled");

    let mut notification_after = async |desired_state: BalancerDesiredState| {
        cluster
            .client_management
            .put_balancer_desired_state(CancellationToken::new(), &desired_state)
            .await
            .expect("the balancer must apply the desired state");

        token_generation_mode_rx
            .recv()
            .await
            .expect("the client must receive the token generation mode change")
    };

    let enabling_notification = notification_after(BalancerDesiredState::default()).await;
    let disabling_notification = notification_after(embeddings_state(0.7)).await;

    assert_eq!(
        [
            connect_notification,
            enabling_notification,
            disabling_notification
        ],
        [
            Notification::TokenGenerationDisabled,
            Notification::TokenGenerationEnabled,
            Notification::TokenGenerationDisabled,
        ]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
