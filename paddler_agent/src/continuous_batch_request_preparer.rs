use std::sync::Arc;
use std::sync::mpsc::Sender;

use llama_cpp_bindings::ChatTools;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::token::LlamaToken;
use log::warn;
use minijinja::context;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_image_decoder::decoded_image::DecodedImage;
use paddler_tool_call_validator::tool_call_validator::ToolCallValidator;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;

use crate::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::continuous_batch_arbiter_command::ContinuousBatchArbiterCommand;
use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::converts_to_mtmd_bitmap::ConvertsToMtmdBitmap;
use crate::embedding_batch_rejection::EmbeddingBatchRejection;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::forward_scheduler_command::forward_scheduler_command;
use crate::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::generation_request_rejection::GenerationRequestRejection;
use crate::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use crate::prepared_generation_request::PreparedGenerationRequest;
use crate::prepared_multimodal_prompt::PreparedMultimodalPrompt;
use crate::prepared_prompt::PreparedPrompt;
use crate::require_embeddings_enabled::require_embeddings_enabled;
use crate::require_prompt_fits_sequence_context::require_prompt_fits_sequence_context;
use crate::resolve_grammar::resolve_grammar;
use crate::tool_call_pipeline::ToolCallPipeline;

#[derive(Clone)]
pub struct ContinuousBatchRequestPreparer {
    pub scheduler_command_tx: Sender<ContinuousBatchSchedulerCommand>,
    pub scheduler_context: Arc<ContinuousBatchSchedulerContext>,
}

impl ContinuousBatchRequestPreparer {
    pub async fn run(
        self,
        mut arbiter_command_rx: mpsc::UnboundedReceiver<ContinuousBatchArbiterCommand>,
    ) {
        while let Some(command) = arbiter_command_rx.recv().await {
            let preparer = self.clone();

            match command {
                ContinuousBatchArbiterCommand::ContinueFromConversationHistory(request) => {
                    tokio::task::spawn_blocking(move || {
                        preparer.accept_conversation_history(request);
                    });
                }
                ContinuousBatchArbiterCommand::ContinueFromRawPrompt(request) => {
                    tokio::task::spawn_blocking(move || preparer.accept_raw_prompt(request));
                }
                ContinuousBatchArbiterCommand::GenerateEmbeddingBatch(request) => {
                    tokio::task::spawn_blocking(move || preparer.accept_embedding_batch(request));
                }
                ContinuousBatchArbiterCommand::Shutdown => {
                    self.forward(ContinuousBatchSchedulerCommand::Shutdown);

                    return;
                }
            }
        }
    }

    fn agent_name(&self) -> Option<&str> {
        self.scheduler_context.agent_name.as_deref()
    }

    fn forward(&self, command: ContinuousBatchSchedulerCommand) {
        forward_scheduler_command(&self.scheduler_command_tx, self.agent_name(), command);
    }

    fn forward_generation(&self, prepared: PreparedGenerationRequest) {
        self.forward(ContinuousBatchSchedulerCommand::Generate(Box::new(
            prepared,
        )));
    }

    fn accept_raw_prompt(&self, request: ContinueFromRawPromptRequest) {
        let generated_tokens_tx = request.generated_tokens_tx.clone();

        match self.prepare_raw_prompt(request) {
            Ok(prepared) => self.forward_generation(prepared),
            Err(rejection) => rejection.report(self.agent_name(), &generated_tokens_tx),
        }
    }

    fn accept_conversation_history(&self, request: ContinueFromConversationHistoryRequest) {
        let generated_tokens_tx = request.generated_tokens_tx.clone();

        match self.prepare_conversation_history(request) {
            Ok(prepared) => self.forward_generation(prepared),
            Err(rejection) => rejection.report(self.agent_name(), &generated_tokens_tx),
        }
    }

    fn accept_embedding_batch(&self, request: GenerateEmbeddingBatchRequest) {
        let generated_embedding_tx = request.generated_embedding_tx.clone();

        match self.prepare_embedding_batch(request) {
            Ok(prepared) => {
                self.forward(ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(
                    prepared,
                ));
            }
            Err(rejection) => rejection.report(self.agent_name(), &generated_embedding_tx),
        }
    }

    fn prepare_raw_prompt(
        &self,
        ContinueFromRawPromptRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params:
                ContinueFromRawPromptParams {
                    grammar,
                    max_tokens,
                    raw_prompt,
                },
            slot_guard,
        }: ContinueFromRawPromptRequest,
    ) -> Result<PreparedGenerationRequest, GenerationRequestRejection> {
        Ok(PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            grammar_sampler: resolve_grammar(grammar.as_ref(), false)?,
            max_tokens,
            prompt: PreparedPrompt::TextTokens(self.tokenize_prompt(&raw_prompt)?),
            slot_guard,
            tool_call_pipeline: None,
        })
    }

    fn prepare_conversation_history(
        &self,
        ContinueFromConversationHistoryRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params:
                ContinueFromConversationHistoryParams {
                    add_generation_prompt,
                    enable_thinking,
                    grammar,
                    conversation_history,
                    max_tokens,
                    parse_tool_calls,
                    tools,
                },
            slot_guard,
        }: ContinueFromConversationHistoryRequest,
    ) -> Result<PreparedGenerationRequest, GenerationRequestRejection> {
        let grammar_sampler = resolve_grammar(grammar.as_ref(), enable_thinking)?;
        let chat_template_renderer = self
            .scheduler_context
            .chat_template_renderer
            .as_deref()
            .ok_or(GenerationRequestRejection::TokenGenerationDisabled)?;

        let bitmaps = conversation_history
            .extract_image_urls()
            .into_iter()
            .map(|image_url| self.decode_image_bitmap(image_url))
            .collect::<Result<Vec<MtmdBitmap>, GenerationRequestRejection>>()?;

        let chat_template_messages =
            conversation_history.replace_images_with_marker(&self.scheduler_context.media_marker);

        let raw_prompt = chat_template_renderer
            .render(context! {
                add_generation_prompt,
                bos_token => self.scheduler_context.token_bos_str,
                enable_thinking,
                eos_token => self.scheduler_context.token_eos_str,
                messages => chat_template_messages.messages,
                nl_token => self.scheduler_context.token_nl_str,
                tools => tools,
            })
            .map_err(GenerationRequestRejection::ChatTemplateRenderingFailed)?;

        let tool_call_pipeline = if parse_tool_calls && !tools.is_empty() {
            Some(self.build_tool_call_pipeline(&tools)?)
        } else {
            None
        };

        let prompt = if bitmaps.is_empty() {
            PreparedPrompt::TextTokens(self.tokenize_prompt(&raw_prompt)?)
        } else {
            PreparedPrompt::Multimodal(PreparedMultimodalPrompt {
                bitmaps,
                multimodal_context: self
                    .scheduler_context
                    .multimodal_context
                    .clone()
                    .ok_or(GenerationRequestRejection::MultimodalNotSupported)?,
                n_batch: i32::try_from(self.scheduler_context.inference_parameters.n_batch)
                    .map_err(GenerationRequestRejection::BatchSizeOutOfRange)?,
                text: raw_prompt,
            })
        };

        Ok(PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            grammar_sampler,
            max_tokens,
            prompt,
            slot_guard,
            tool_call_pipeline,
        })
    }

    fn decode_image_bitmap(
        &self,
        image_url: &ImageUrl,
    ) -> Result<MtmdBitmap, GenerationRequestRejection> {
        DecodedImage::from_data_uri(
            image_url,
            self.scheduler_context
                .inference_parameters
                .image_resize_to_fit,
        )
        .map_err(GenerationRequestRejection::ImageDecodingFailed)?
        .to_mtmd_bitmap()
        .map_err(GenerationRequestRejection::ImageBitmapCreationFailed)
    }

    fn build_tool_call_pipeline(
        &self,
        tools: &[Tool<ValidatedParametersSchema>],
    ) -> Result<ToolCallPipeline, GenerationRequestRejection> {
        let tools_json = serde_json::to_string(tools)
            .map_err(GenerationRequestRejection::ToolsSerializationFailed)?;

        Ok(ToolCallPipeline::new(
            self.scheduler_context.model.clone(),
            ChatTools::from_json(tools_json)
                .map_err(GenerationRequestRejection::ChatToolsInvalid)?,
            ToolCallValidator::from_tools(tools)
                .map_err(GenerationRequestRejection::ToolSchemaInvalid)?,
        ))
    }

    fn tokenize_prompt(&self, prompt: &str) -> Result<Vec<LlamaToken>, GenerationRequestRejection> {
        let prompt_tokens = self
            .scheduler_context
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(GenerationRequestRejection::PromptTokenizationFailed)?;

        require_prompt_fits_sequence_context(
            prompt_tokens.len(),
            self.scheduler_context.sequence_context_size,
        )?;

        Ok(prompt_tokens)
    }

    fn prepare_embedding_batch(
        &self,
        GenerateEmbeddingBatchRequest {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            params:
                GenerateEmbeddingBatchParams {
                    input_batch,
                    normalization_method,
                },
            slot_guard,
        }: GenerateEmbeddingBatchRequest,
    ) -> Result<PreparedEmbeddingBatchRequest, EmbeddingBatchRejection> {
        let inference_parameters = &self.scheduler_context.inference_parameters;

        require_embeddings_enabled(inference_parameters.enable_embeddings)?;

        let n_batch = inference_parameters.n_batch;
        let mut inputs = Vec::with_capacity(input_batch.len());

        for input in input_batch {
            let tokens = self
                .scheduler_context
                .model
                .str_to_token(&input.content, AddBos::Always)
                .map_err(|source| EmbeddingBatchRejection::InputTokenizationFailed {
                    source_document_id: input.id.clone(),
                    source,
                })?;

            if tokens.len() > n_batch {
                let details = OversizedEmbeddingDocumentDetails {
                    document_tokens: tokens.len(),
                    n_batch,
                    source_document_id: input.id,
                };

                warn!(
                    "{:?}: skipped embedding document {:?}: {} tokens exceeds n_batch {}",
                    self.agent_name(),
                    details.source_document_id,
                    details.document_tokens,
                    details.n_batch,
                );

                generated_embedding_tx
                    .send(EmbeddingResult::DocumentExceedsBatchSize(details))
                    .map_err(EmbeddingBatchRejection::ClientDisconnected)?;
            } else {
                inputs.push(EmbeddingInputTokenized {
                    id: input.id,
                    tokens,
                });
            }
        }

        Ok(PreparedEmbeddingBatchRequest {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            inputs,
            normalization_method,
            slot_guard,
        })
    }
}
