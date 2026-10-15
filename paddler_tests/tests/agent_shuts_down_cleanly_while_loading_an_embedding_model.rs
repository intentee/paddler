#![cfg(feature = "tests_that_use_llms")]

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

use crate::state_transition_interrupted_by_shutdown::StateTransitionInterruptedByShutdown;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_while_loading_an_embedding_model() {
    StateTransitionInterruptedByShutdown {
        initial_desired_state: BalancerDesiredState::default(),
        next_desired_state: nomic_embed_text_v1_5()
            .into_embeddings_desired_state(EmbeddingParameters::default()),
        slot_count: 1,
    }
    .shut_down_while_the_agent_applies_the_next_state()
    .await;
}
