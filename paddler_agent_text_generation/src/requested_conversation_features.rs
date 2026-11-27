use llama_cpp_bindings_types::chat_template_capabilities::ChatTemplateCapabilities;
use llama_cpp_bindings_types::conversation_capability::ConversationCapability;
use llama_cpp_bindings_types::tool_calling_capability::ToolCallingCapability;

use paddler_inference_parameters::model_default_or::ModelDefaultOr;

use crate::conversation_features::ConversationFeatures;
use crate::text_generation_error::TextGenerationError;

pub struct RequestedConversationFeatures {
    pub enable_thinking: ModelDefaultOr<bool>,
    pub offers_tools: bool,
    pub parallel_tool_calls: ModelDefaultOr<bool>,
}

impl RequestedConversationFeatures {
    /// # Errors
    ///
    /// Returns [`TextGenerationError::ConversationUnsupported`] when the model cannot hold a conversation, [`TextGenerationError::ThinkingUnsupported`] when thinking is requested from a chat template that cannot think, [`TextGenerationError::ToolCallingUnsupported`] when tools are offered to a chat template that cannot call them, and [`TextGenerationError::ParallelToolCallingUnsupported`] when parallel tool calls are requested from a chat template that makes one call per turn.
    pub fn resolve(
        self,
        conversation: ConversationCapability,
    ) -> Result<ConversationFeatures, TextGenerationError> {
        let ConversationCapability::Supported(ChatTemplateCapabilities {
            thinking: template_thinks,
            tool_calling,
            ..
        }) = conversation
        else {
            return Err(TextGenerationError::ConversationUnsupported);
        };
        let enable_thinking = self.enable_thinking.resolve(template_thinks);

        if enable_thinking && !template_thinks {
            return Err(TextGenerationError::ThinkingUnsupported);
        }

        if self.offers_tools && tool_calling == ToolCallingCapability::Unsupported {
            return Err(TextGenerationError::ToolCallingUnsupported);
        }

        let template_calls_tools_in_parallel = tool_calling == ToolCallingCapability::ParallelCalls;
        let parallel_tool_calls = self
            .parallel_tool_calls
            .resolve(template_calls_tools_in_parallel);

        if parallel_tool_calls && !template_calls_tools_in_parallel {
            return Err(TextGenerationError::ParallelToolCallingUnsupported);
        }

        Ok(ConversationFeatures {
            enable_thinking,
            parallel_tool_calls,
        })
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::chat_template_capabilities::ChatTemplateCapabilities;
    use llama_cpp_bindings_types::conversation_capability::ConversationCapability;
    use llama_cpp_bindings_types::conversation_unsupported_reason::ConversationUnsupportedReason;
    use llama_cpp_bindings_types::tool_calling_capability::ToolCallingCapability;

    use paddler_inference_parameters::model_default_or::ModelDefaultOr;

    use super::RequestedConversationFeatures;
    use crate::conversation_features::ConversationFeatures;
    use crate::text_generation_error::TextGenerationError;

    const PLAIN_TEMPLATE: ConversationCapability =
        ConversationCapability::Supported(ChatTemplateCapabilities {
            system_role: true,
            thinking: false,
            tool_calling: ToolCallingCapability::OneCallPerTurn,
        });

    const THINKING_TEMPLATE_CALLING_TOOLS_IN_PARALLEL: ConversationCapability =
        ConversationCapability::Supported(ChatTemplateCapabilities {
            system_role: true,
            thinking: true,
            tool_calling: ToolCallingCapability::ParallelCalls,
        });

    const fn requesting(
        enable_thinking: ModelDefaultOr<bool>,
        offers_tools: bool,
        parallel_tool_calls: ModelDefaultOr<bool>,
    ) -> RequestedConversationFeatures {
        RequestedConversationFeatures {
            enable_thinking,
            offers_tools,
            parallel_tool_calls,
        }
    }

    #[test]
    fn a_model_without_a_conversation_refuses_one() {
        assert!(matches!(
            requesting(
                ModelDefaultOr::ModelDefault,
                false,
                ModelDefaultOr::ModelDefault
            )
            .resolve(ConversationCapability::Unsupported(
                ConversationUnsupportedReason::ModelHasNoChatTemplate
            )),
            Err(TextGenerationError::ConversationUnsupported)
        ));
    }

    #[test]
    fn model_defaults_follow_what_the_template_supports() {
        assert!(matches!(
            requesting(
                ModelDefaultOr::ModelDefault,
                true,
                ModelDefaultOr::ModelDefault
            )
            .resolve(THINKING_TEMPLATE_CALLING_TOOLS_IN_PARALLEL),
            Ok(ConversationFeatures {
                enable_thinking: true,
                parallel_tool_calls: true,
            })
        ));
        assert!(matches!(
            requesting(
                ModelDefaultOr::ModelDefault,
                true,
                ModelDefaultOr::ModelDefault
            )
            .resolve(PLAIN_TEMPLATE),
            Ok(ConversationFeatures {
                enable_thinking: false,
                parallel_tool_calls: false,
            })
        ));
    }

    #[test]
    fn explicitly_disabled_features_stay_off_on_a_template_that_supports_them() {
        assert!(matches!(
            requesting(
                ModelDefaultOr::Explicit(false),
                true,
                ModelDefaultOr::Explicit(false)
            )
            .resolve(THINKING_TEMPLATE_CALLING_TOOLS_IN_PARALLEL),
            Ok(ConversationFeatures {
                enable_thinking: false,
                parallel_tool_calls: false,
            })
        ));
    }

    #[test]
    fn thinking_is_refused_by_a_template_that_cannot_think() {
        assert!(matches!(
            requesting(
                ModelDefaultOr::Explicit(true),
                false,
                ModelDefaultOr::ModelDefault
            )
            .resolve(PLAIN_TEMPLATE),
            Err(TextGenerationError::ThinkingUnsupported)
        ));
    }

    #[test]
    fn tools_are_refused_by_a_template_that_cannot_call_them() {
        assert!(matches!(
            requesting(
                ModelDefaultOr::ModelDefault,
                true,
                ModelDefaultOr::ModelDefault
            )
            .resolve(ConversationCapability::Supported(
                ChatTemplateCapabilities {
                    system_role: true,
                    thinking: false,
                    tool_calling: ToolCallingCapability::Unsupported,
                }
            )),
            Err(TextGenerationError::ToolCallingUnsupported)
        ));
    }

    #[test]
    fn parallel_tool_calls_are_refused_by_a_template_making_one_call_per_turn() {
        assert!(matches!(
            requesting(
                ModelDefaultOr::ModelDefault,
                true,
                ModelDefaultOr::Explicit(true)
            )
            .resolve(PLAIN_TEMPLATE),
            Err(TextGenerationError::ParallelToolCallingUnsupported)
        ));
    }
}
