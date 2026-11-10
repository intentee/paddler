use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tokio_util::sync::CancellationToken;

use paddler_client::client_inference::ClientInference;
use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::unending_generation::unending_generation;

pub struct SlotHeldByAnUnendingGeneration {
    generation_stop: CancellationToken,
    unending_generation_stream: InferenceMessageStream,
}

impl SlotHeldByAnUnendingGeneration {
    /// # Errors
    ///
    /// Returns an error when the cluster has no registered agent, when the unending generation is not accepted or streams no readable message, and when it does not hold one slot of that agent.
    pub async fn hold(cluster: &mut Cluster) -> Result<Self> {
        ready(
            cluster
                .agent_ids
                .first()
                .cloned()
                .context("the cluster must register its agent"),
        )
        .and_then(|agent_id| Self::hold_a_slot_of(cluster, agent_id))
        .await
    }

    async fn after_its_first_message(mut self) -> Result<Self> {
        self.unending_generation_stream
            .next()
            .await
            .context("the generation must stream a message while it holds its slot")
            .and_then(|first_message| first_message.context("the message must be readable"))
            .map(|_first_message| self)
    }

    async fn hold_a_slot_of(cluster: &mut Cluster, agent_id: String) -> Result<Self> {
        let streaming = Self::start(&cluster.client_inference)
            .and_then(Self::after_its_first_message)
            .await;

        ready(streaming)
            .and_then(|streaming_generation| {
                streaming_generation.once_it_holds_a_slot_of(cluster, &agent_id)
            })
            .await
    }

    async fn once_it_holds_a_slot_of(self, cluster: &mut Cluster, agent_id: &str) -> Result<Self> {
        cluster
            .wait_for_slots_processing(agent_id, 1)
            .await
            .context("the generation must hold one slot")
            .map(|_holding| self)
    }

    async fn start(client_inference: &ClientInference) -> Result<Self> {
        let generation_stop = CancellationToken::new();

        client_inference
            .continue_from_raw_prompt(generation_stop.clone(), unending_generation())
            .await
            .context("the generation must be accepted")
            .map(|unending_generation_stream| Self {
                generation_stop,
                unending_generation_stream,
            })
    }

    /// # Errors
    ///
    /// Returns an error when the cancelled generation does not end its stream.
    pub async fn release(mut self) -> Result<()> {
        self.generation_stop.cancel();

        self.unending_generation_stream
            .next()
            .await
            .is_none()
            .then_some(())
            .context("a cancelled generation must end its stream")
    }
}
