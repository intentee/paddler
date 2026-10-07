use std::num::NonZeroU32;

use tokio::sync::mpsc;

use paddler_agent_runtime::scheduler_command::SchedulerCommand;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::generated_token_result::GeneratedTokenResult;

use crate::prepared_prompt::PreparedPrompt;
use crate::text_generation_error::TextGenerationError;
use crate::token_classification::TokenClassification;
use crate::token_sampling::TokenSampling;
use crate::tool_call_handling::ToolCallHandling;

pub struct PreparedGenerationRequest {
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub max_tokens: NonZeroU32,
    pub prompt: PreparedPrompt,
    pub slot_guard: SlotGuard,
    pub token_classification: TokenClassification,
    pub token_sampling: TokenSampling,
    pub tool_call_handling: ToolCallHandling,
}

impl SchedulerCommand for PreparedGenerationRequest {
    fn reject_because_the_scheduler_stopped(self, agent_name: Option<&str>) {
        TextGenerationError::SchedulerUnavailable.report(agent_name, &self.generated_tokens_tx);
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::sync::Arc;

    use llama_cpp_bindings::BareJsonToolCalls;
    use llama_cpp_bindings::StreamingMarkers;
    use tokio::sync::mpsc;

    use paddler_agent_runtime::scheduler_command::SchedulerCommand as _;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_status::slot_guard::SlotGuard;
    use paddler_inference_parameters::sampling_parameters::SamplingParameters;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    use super::PreparedGenerationRequest;
    use crate::grammar_sampling::GrammarSampling;
    use crate::prepared_prompt::PreparedPrompt;
    use crate::sampler_chain_factory::SamplerChainFactory;
    use crate::token_classification::TokenClassification;
    use crate::token_sampling::TokenSampling;
    use crate::tool_call_handling::ToolCallHandling;

    #[test]
    fn rejects_generation_when_the_scheduler_stopped() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            max_tokens: NonZeroU32::new(1).unwrap(),
            prompt: PreparedPrompt::TextTokens(Vec::new()),
            slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
            token_classification: TokenClassification {
                bare_json_tool_calls: BareJsonToolCalls::Ignore,
                streaming_markers: Arc::new(StreamingMarkers::default()),
            },
            token_sampling: TokenSampling {
                chain: SamplerChainFactory {
                    sampling_parameters: SamplingParameters::default(),
                    n_vocab: 1,
                }
                .create(0)
                .unwrap(),
                grammar: GrammarSampling::Unconstrained,
            },
            tool_call_handling: ToolCallHandling::Streamed,
        }
        .reject_because_the_scheduler_stopped(Some("agent"));

        assert_eq!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::SchedulerUnavailable(
                "agent: the scheduler is no longer accepting requests".to_owned()
            ))
        );
    }
}
