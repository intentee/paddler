#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::unending_generation::unending_generation;

pub struct SlotHeldByAnUnendingGeneration {
    generation_stop: CancellationToken,
    unending_generation_stream: InferenceMessageStream,
}

impl SlotHeldByAnUnendingGeneration {
    pub async fn hold(cluster: &mut Cluster) -> Self {
        let agent_id = cluster
            .agent_ids
            .first()
            .expect("the cluster must register its agent")
            .clone();
        let generation_stop = CancellationToken::new();
        let mut unending_generation_stream = cluster
            .client_inference
            .continue_from_raw_prompt(generation_stop.clone(), unending_generation())
            .await
            .expect("the generation must be accepted");

        unending_generation_stream
            .next()
            .await
            .expect("the generation must stream a message while it holds its slot")
            .expect("the message must be readable");
        cluster
            .wait_for_slots_processing(&agent_id, 1)
            .await
            .expect("the generation must hold one slot");

        Self {
            generation_stop,
            unending_generation_stream,
        }
    }

    pub async fn release(mut self) {
        self.generation_stop.cancel();

        assert!(
            self.unending_generation_stream.next().await.is_none(),
            "a cancelled generation must end its stream"
        );
    }
}
