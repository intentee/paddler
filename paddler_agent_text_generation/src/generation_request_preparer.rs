use std::num::NonZeroU32;

use llama_cpp_bindings::chat_assistant_turn::ChatAssistantTurn;
use llama_cpp_bindings::chat_session::ChatSession;
use llama_cpp_bindings::chat_session_options::ChatSessionOptions;
use llama_cpp_bindings::chat_tool_choice::ChatToolChoice;
use llama_cpp_bindings::mtmd::mtmd_bitmap::MtmdBitmap;
use llama_cpp_bindings_types::chat_tool::ChatTool;
use rand::RngExt as _;
use rand::rng;
use serde_json::Value;

use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;
use paddler_image_decoder::decoded_image::DecodedImage;
use paddler_messaging::chat_template_conversation::ChatTemplateConversation;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;

use crate::conversation_prompt_support::ConversationPromptSupport;
use crate::conversation_support::ConversationSupport;
use crate::converts_to_mtmd_bitmap::ConvertsToMtmdBitmap;
use crate::grammar_sampling::GrammarSampling;
use crate::media_marked_prompt::MediaMarkedPrompt;
use crate::prepared_generation_request::PreparedGenerationRequest;
use crate::prepared_prompt::PreparedPrompt;
use crate::prompt_modality::PromptModality;
use crate::prompt_tokenizer::PromptTokenizer;
use crate::request_media_marker::request_media_marker;
use crate::require_grammar_compatible_with_thinking::require_grammar_compatible_with_thinking;
use crate::resolve_grammar::resolve_grammar;
use crate::sampler_chain_factory::SamplerChainFactory;
use crate::text_generation_error::TextGenerationError;
use crate::text_generation_request::TextGenerationRequest;
use crate::token_classification::TokenClassification;
use crate::token_sampling::TokenSampling;
use crate::tool_call_handling::ToolCallHandling;
use crate::tool_call_pipeline::ToolCallPipeline;

const fn assistant_turn_of(add_generation_prompt: bool) -> ChatAssistantTurn {
    if add_generation_prompt {
        ChatAssistantTurn::New
    } else {
        ChatAssistantTurn::Omitted
    }
}

const fn tool_choice_of(chat_tools: &[ChatTool]) -> ChatToolChoice {
    if chat_tools.is_empty() {
        ChatToolChoice::None
    } else {
        ChatToolChoice::Auto
    }
}

pub struct GenerationRequestPreparer {
    pub conversation_support: ConversationSupport,
    pub image_resize_to_fit: NonZeroU32,
    pub loaded_llama_model: LoadedLlamaModel,
    pub prompt_tokenizer: PromptTokenizer,
    pub sampler_chain_factory: SamplerChainFactory,
}

impl GenerationRequestPreparer {
    pub fn prepare_raw_prompt(
        &self,
        AgentRequest {
            params:
                ContinueFromRawPromptParams {
                    grammar,
                    max_tokens,
                    raw_prompt,
                },
            response_tx: generated_tokens_tx,
            slot_guard,
            stop_rx: generate_tokens_stop_rx,
        }: AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>,
    ) -> Result<PreparedGenerationRequest, TextGenerationError> {
        let prompt = PreparedPrompt::TextTokens(self.prompt_tokenizer.tokenize(&raw_prompt)?);
        let token_sampling =
            self.build_token_sampling(resolve_grammar(grammar, &self.loaded_llama_model.model)?)?;

        Ok(PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            max_tokens,
            prompt,
            slot_guard,
            token_classification: TokenClassification::Undeterminable,
            token_sampling,
            tool_call_handling: ToolCallHandling::Streamed,
        })
    }

    pub fn prepare_conversation_history(
        &self,
        AgentRequest {
            response_tx: generated_tokens_tx,
            stop_rx: generate_tokens_stop_rx,
            params:
                ContinueFromConversationHistoryParams {
                    add_generation_prompt,
                    conversation_history,
                    enable_thinking,
                    grammar,
                    max_tokens,
                    parse_tool_calls,
                    tools,
                },
            slot_guard,
        }: AgentRequest<
            ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
            GeneratedTokenResult,
        >,
    ) -> Result<PreparedGenerationRequest, TextGenerationError> {
        let ConversationPromptSupport {
            chat_templates,
            image_input,
        } = self.conversation_support.prompt_support()?;

        require_grammar_compatible_with_thinking(grammar.as_ref(), enable_thinking)?;

        let model = &self.loaded_llama_model.model;
        let tools_with_schema_documents: Vec<Tool<Value>> =
            tools.into_iter().map(Tool::from).collect();
        let tool_call_handling = if parse_tool_calls {
            ToolCallHandling::Parsed(ToolCallPipeline::new(
                ToolCallValidator::from_tools(&tools_with_schema_documents)
                    .map_err(TextGenerationError::ToolSchemaInvalid)?,
            ))
        } else {
            ToolCallHandling::Streamed
        };
        let chat_tools: Vec<ChatTool> = tools_with_schema_documents
            .iter()
            .map(ChatTool::from)
            .collect();
        let media_marker = request_media_marker(&conversation_history, &chat_tools);
        let ChatTemplateConversation {
            image_urls,
            messages,
        } = conversation_history.into_chat_template_conversation(&media_marker);
        let prompt_modality = image_input.prompt_modality_for(&image_urls)?;
        let chat_session = ChatSession::new(
            model,
            chat_templates,
            &messages,
            &chat_tools,
            &ChatSessionOptions {
                assistant_turn: assistant_turn_of(add_generation_prompt),
                enable_thinking,
                parallel_tool_calls: true,
                tool_choice: tool_choice_of(&chat_tools),
            },
        )
        .map_err(TextGenerationError::ChatSessionCreationFailed)?;

        let prompt = match prompt_modality {
            PromptModality::Multimodal(multimodal_prompt_support) => PreparedPrompt::Multimodal(
                multimodal_prompt_support.prepare_prompt(
                    &image_urls
                        .iter()
                        .map(|image_url| self.decode_image_bitmap(image_url))
                        .collect::<Result<Vec<MtmdBitmap>, TextGenerationError>>()?,
                    &MediaMarkedPrompt {
                        media_marker: &media_marker,
                        prompt_parts: &chat_session
                            .prompt_parts()
                            .map_err(TextGenerationError::ChatPromptUnreadable)?,
                    },
                )?,
            ),
            PromptModality::TextOnly => PreparedPrompt::TextTokens(
                self.prompt_tokenizer.tokenize_chat_session(&chat_session)?,
            ),
        };

        let grammar_sampling = match grammar {
            requested_grammar @ Some(_) => resolve_grammar(requested_grammar, model)?,
            None => chat_session
                .grammar_sampler(model)
                .map_err(TextGenerationError::ChatSessionGrammarUnavailable)?
                .map_or(GrammarSampling::Unconstrained, GrammarSampling::Constrained),
        };
        let token_sampling = self.build_token_sampling(grammar_sampling)?;

        Ok(PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            max_tokens,
            prompt,
            slot_guard,
            token_classification: TokenClassification::ChatSession(chat_session),
            token_sampling,
            tool_call_handling,
        })
    }

    fn generation_command<TParams>(
        &self,
        agent_name: Option<&str>,
        request: AgentRequest<TParams, GeneratedTokenResult>,
        prepare: fn(
            &Self,
            AgentRequest<TParams, GeneratedTokenResult>,
        ) -> Result<PreparedGenerationRequest, TextGenerationError>,
    ) -> Option<PreparedGenerationRequest> {
        let generated_tokens_tx = request.response_tx.clone();

        match prepare(self, request) {
            Ok(prepared) => Some(prepared),
            Err(rejection) => {
                rejection.report(agent_name, &generated_tokens_tx);

                None
            }
        }
    }

    fn decode_image_bitmap(&self, image_url: &ImageUrl) -> Result<MtmdBitmap, TextGenerationError> {
        DecodedImage::from_data_uri(image_url, self.image_resize_to_fit)
            .map_err(TextGenerationError::ImageDecodingFailed)?
            .to_mtmd_bitmap()
            .map_err(TextGenerationError::ImageBitmapCreationFailed)
    }

    fn build_token_sampling(
        &self,
        grammar: GrammarSampling,
    ) -> Result<TokenSampling, TextGenerationError> {
        let chain = self
            .sampler_chain_factory
            .create(rng().random::<u32>())
            .map_err(TextGenerationError::SamplerChainCreationFailed)?;

        Ok(TokenSampling { chain, grammar })
    }
}

impl PreparesSchedulerCommand for GenerationRequestPreparer {
    type Command = PreparedGenerationRequest;
    type Request = TextGenerationRequest;

    fn prepare_scheduler_command(
        &self,
        agent_name: Option<&str>,
        request: Self::Request,
    ) -> Option<Self::Command> {
        match request {
            TextGenerationRequest::ContinueFromConversationHistory(request) => {
                self.generation_command(agent_name, request, Self::prepare_conversation_history)
            }
            TextGenerationRequest::ContinueFromRawPrompt(request) => {
                self.generation_command(agent_name, request, Self::prepare_raw_prompt)
            }
        }
    }
}
