use std::num::NonZeroU32;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;

pub struct DecisionClusterParams {
    pub agents: Vec<AgentConfig>,
    pub context_size: NonZeroU32,
    pub model_card: ModelCard,
    pub n_batch: BatchSize,
    pub pointer_head_fixture: &'static str,
    pub wait_for_slots_ready: bool,
}

impl Default for DecisionClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 2),
            context_size: ModelRuntimeParameters::default().context_size,
            model_card: qwen3_5_0_8b(),
            n_batch: BatchSize::DEFAULT,
            pointer_head_fixture: "qwen3_5_0_8b_synthetic_pointer_head.gguf",
            wait_for_slots_ready: true,
        }
    }
}
