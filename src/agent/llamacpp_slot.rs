use std::sync::Arc;

use actix::Actor;
use actix::Handler;
use actix::SyncContext;
use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_2::DecodeError;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::Special;
use llama_cpp_2::sampling::LlamaSampler;
use log::debug;
use log::error;
use log::info;
use minijinja::context;
use rand::Rng as _;
use rand::rngs::ThreadRng;
use tokio::sync::mpsc;

use crate::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::agent::kv_cache_repair_action::KVCacheRepairAction;
use crate::agent::llamacpp_slot_context::LlamaCppSlotContext;
use crate::embedding::Embedding;
use crate::embedding_input_tokenized::EmbeddingInputTokenized;
use crate::embedding_normalization_method::EmbeddingNormalizationMethod;
use crate::embedding_result::EmbeddingResult;
use crate::generated_token_result::GeneratedTokenResult;
use crate::request_params::ContinueFromConversationHistoryParams;
use crate::request_params::ContinueFromRawPromptParams;
use crate::request_params::GenerateEmbeddingBatchParams;
use crate::slot_status::SlotStatus;

pub struct LlamaCppSlot {
    index: u32,
    llama_context: LlamaContext<'static>,
    rng: ThreadRng,
    slot_context: Arc<LlamaCppSlotContext>,
    status: Arc<SlotStatus>,
}

impl LlamaCppSlot {
    pub fn new(
        index: u32,
        llama_backend: Arc<LlamaBackend>,
        llama_ctx_params: Arc<LlamaContextParams>,
        slot_context: Arc<LlamaCppSlotContext>,
        status: Arc<SlotStatus>,
    ) -> Result<Self> {
        debug_assert!(
            Arc::strong_count(&slot_context.model) >= 1,
            "Model Arc must have at least one reference"
        );

        let llama_context = unsafe {
            // SAFETY: Extending the lifetime of the model reference to 'static.
            // This should be safe because:
            // 1. The model is stored in an Arc, so it won't be deallocated
            // 2. We store the Arc in the same struct, ensuring it lives as long as the context
            // 3. The context cannot outlive the struct that contains both it and the model
            let model_ref: &'static LlamaModel = std::mem::transmute(slot_context.model.as_ref());

            model_ref.new_context(&llama_backend, (*llama_ctx_params).clone())?
        };

        Ok(Self {
            index,
            llama_context,
            rng: rand::rng(),
            slot_context,
            status,
        })
    }

    fn continuation_batch_decode(
        &mut self,
        batch: &mut LlamaBatch,
        taken_kv_cache_repair_actions: &mut Vec<KVCacheRepairAction>,
    ) -> Result<()> {
        if let Err(err) = self.llama_context.decode(batch) {
            match err {
                DecodeError::NoKvCacheSlot => {
                    if !taken_kv_cache_repair_actions.contains(&KVCacheRepairAction::Clear) {
                        debug!(
                            "{:?}: slot {} has no KV cache slot, defragmenting",
                            self.slot_context.agent_name, self.index
                        );

                        taken_kv_cache_repair_actions.push(KVCacheRepairAction::Clear);
                        self.llama_context.clear_kv_cache();

                        return self
                            .continuation_batch_decode(batch, taken_kv_cache_repair_actions);
                    }

                    Err(err.into())
                }
                DecodeError::NTokensZero => {
                    debug!(
                        "{:?}: slot {} - the number of tokens in the batch was 0",
                        self.slot_context.agent_name, self.index,
                    );

                    Err(err.into())
                }
                DecodeError::Unknown(error_code) => {
                    Err(anyhow!("Unknown error code: {error_code}"))
                }
            }
        } else {
            Ok(())
        }
    }

    fn embedding_batch_decode(
        &mut self,
        batch: &mut LlamaBatch,
        current_batch_embeddings: &Vec<&EmbeddingInputTokenized>,
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
        normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<()> {
        self.llama_context.clear_kv_cache();
        self.llama_context.decode(batch)?;

        for (index, embedding_input_tokenized) in current_batch_embeddings.iter().enumerate() {
            let embedding = self
                .llama_context
                .embeddings_seq_ith(index as i32)
                .context("Failed to get embeddings")?;

            generated_embedding_tx.send(EmbeddingResult::Embedding(
                Embedding {
                    embedding: embedding.to_vec(),
                    normalization_method: EmbeddingNormalizationMethod::None,
                    pooling_type: self.slot_context.inference_parameters.pooling_type.clone(),
                    source_document_id: embedding_input_tokenized.id.clone(),
                }
                .normalize(normalization_method)?,
            ))?;
        }

        batch.clear();

        Ok(())
    }

    fn continue_from_raw_prompt(
        &mut self,
        mut generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
        generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
        max_tokens: i32,
        prompt: String,
    ) -> Result<()> {
        let _guard = self.status.take_slot_with_guard();

        self.llama_context.clear_kv_cache();

        let tokens_list = self
            .slot_context
            .model
            .str_to_token(&prompt, AddBos::Always)?;
        let mut batch = LlamaBatch::new(self.slot_context.inference_parameters.batch_n_tokens, 1);
        let last_index = tokens_list.len() as i32 - 1;

        for (i, token) in (0_i32..).zip(tokens_list.into_iter()) {
            let is_last = i == last_index;

            batch.add(token, i, &[0], is_last)?;
        }

        self.continuation_batch_decode(&mut batch, &mut vec![])?;

        let mut n_cur = batch.n_tokens();
        let mut decoder = encoding_rs::UTF_8.new_decoder();

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::penalties(
                self.slot_context.inference_parameters.penalty_last_n,
                self.slot_context.inference_parameters.penalty_repeat,
                self.slot_context.inference_parameters.penalty_frequency,
                self.slot_context.inference_parameters.penalty_presence,
            ),
            LlamaSampler::top_k(self.slot_context.inference_parameters.top_k),
            LlamaSampler::top_p(self.slot_context.inference_parameters.top_p, 0),
            LlamaSampler::min_p(self.slot_context.inference_parameters.min_p, 0),
            LlamaSampler::temp(self.slot_context.inference_parameters.temperature),
            LlamaSampler::dist(self.rng.random::<u32>()),
            LlamaSampler::greedy(),
        ]);

        while n_cur <= max_tokens {
            if generate_tokens_stop_rx.try_recv().is_ok() {
                break;
            }

            // sample the next token
            {
                let token = sampler.sample(&self.llama_context, batch.n_tokens() - 1);

                sampler.accept(token);

                if token == self.slot_context.model.token_eos() {
                    break;
                }

                let output_bytes = self
                    .slot_context
                    .model
                    .token_to_bytes(token, Special::Tokenize)?;
                let mut output_string = String::with_capacity(32);
                let _decode_result =
                    decoder.decode_to_string(&output_bytes, &mut output_string, false);

                generated_tokens_tx.send(GeneratedTokenResult::Token(output_string))?;

                batch.clear();
                batch.add(token, n_cur, &[0], true)?;
            }

            n_cur += 1;

            self.continuation_batch_decode(&mut batch, &mut vec![])?;
        }

        generated_tokens_tx.send(GeneratedTokenResult::Done)?;

        Ok(())
    }

    fn generate_embedding_batch(
        &mut self,
        GenerateEmbeddingBatchRequest {
            mut generate_embedding_stop_rx,
            generated_embedding_tx,
            params:
                GenerateEmbeddingBatchParams {
                    input_batch,
                    normalization_method,
                },
        }: GenerateEmbeddingBatchRequest,
    ) -> Result<()> {
        if !self.slot_context.inference_parameters.enable_embeddings {
            return Err(anyhow!(
                "Embeddings are not enabled for this slot: {:?}",
                self.slot_context.agent_name
            ));
        }

        let _guard = self.status.take_slot_with_guard();

        self.llama_context.clear_kv_cache();

        let tokens_lines_list = input_batch
            .into_iter()
            .map(|input| {
                match self
                    .slot_context
                    .model
                    .str_to_token(&input.content, AddBos::Always)
                {
                    Ok(llama_tokens) => Ok(EmbeddingInputTokenized {
                        id: input.id,
                        llama_tokens,
                    }),
                    Err(err) => Err(anyhow!("Failed to tokenize input: {err:?}")),
                }
            })
            .collect::<Result<Vec<EmbeddingInputTokenized>, _>>()
            .context("failed to tokenize embedding input batch")?;

        let mut batch = LlamaBatch::new(self.slot_context.inference_parameters.batch_n_tokens, 1);
        let mut current_batch_embeddings: Vec<&EmbeddingInputTokenized> = Vec::new();

        for embedding_input_tokenized in &tokens_lines_list {
            if generate_embedding_stop_rx.try_recv().is_ok() {
                break;
            }

            // Flush the batch if the next prompt would exceed our batch size
            if (batch.n_tokens() as usize + embedding_input_tokenized.llama_tokens.len())
                > self.slot_context.inference_parameters.batch_n_tokens
            {
                self.embedding_batch_decode(
                    &mut batch,
                    &current_batch_embeddings,
                    &generated_embedding_tx,
                    &normalization_method,
                )?;

                current_batch_embeddings.clear();
            }

            batch.add_sequence(
                &embedding_input_tokenized.llama_tokens,
                current_batch_embeddings.len() as i32,
                false,
            )?;
            current_batch_embeddings.push(embedding_input_tokenized);
        }

        if generate_embedding_stop_rx.try_recv().is_ok() {
            return Ok(());
        }

        self.embedding_batch_decode(
            &mut batch,
            &current_batch_embeddings,
            &generated_embedding_tx,
            &normalization_method,
        )?;

        Ok(())
    }
}

impl Actor for LlamaCppSlot {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.status.started();

        info!(
            "{:?}: slot {} ready with model {:?}",
            self.slot_context.agent_name,
            self.index,
            self.slot_context.model_path.display(),
        );
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.status.stopped();

        info!(
            "{:?}: slot {} stopped",
            self.slot_context.agent_name, self.index,
        );
    }
}

impl Handler<ContinueFromConversationHistoryRequest> for LlamaCppSlot {
    type Result = Result<()>;

    fn handle(
        &mut self,
        ContinueFromConversationHistoryRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params:
                ContinueFromConversationHistoryParams {
                    add_generation_prompt,
                    enable_thinking,
                    conversation_history,
                    max_tokens,
                    tools,
                },
        }: ContinueFromConversationHistoryRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let raw_prompt = match self.slot_context.chat_template_renderer.render(context! {
            // Known uses:
            // https://huggingface.co/unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF
            add_generation_prompt,
            // Known uses:
            // https://huggingface.co/bartowski/Mistral-7B-Instruct-v0.3-GGUF
            // https://huggingface.co/unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF
            bos_token => self.slot_context.token_bos_str,
            // Known uses:
            // https://huggingface.co/Qwen/Qwen3-0.6B-GGUF
            enable_thinking,
            // Known uses:
            // https://huggingface.co/bartowski/Mistral-7B-Instruct-v0.3-GGUF
            eos_token => self.slot_context.token_eos_str,
            messages => conversation_history,
            nl_token => self.slot_context.token_nl_str,
            tools => tools,
        }) {
            Ok(raw_prompt) => raw_prompt,
            Err(err) => {
                let msg = format!(
                    "{:?}: slot {} failed to render chat template: {err:?}",
                    self.slot_context.agent_name, self.index
                );

                error!("{msg}");

                generated_tokens_tx.send(GeneratedTokenResult::ChatTemplateError(msg))?;

                return Err(err);
            }
        };

        debug!(
            "{:?}: slot {} generating from raw prompt: {:?}",
            self.slot_context.agent_name, self.index, raw_prompt
        );

        self.continue_from_raw_prompt(
            generate_tokens_stop_rx,
            generated_tokens_tx,
            max_tokens,
            raw_prompt,
        )
    }
}

impl Handler<ContinueFromRawPromptRequest> for LlamaCppSlot {
    type Result = Result<()>;

    fn handle(
        &mut self,
        ContinueFromRawPromptRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params:
                ContinueFromRawPromptParams {
                    max_tokens,
                    raw_prompt,
                },
        }: ContinueFromRawPromptRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.continue_from_raw_prompt(
            generate_tokens_stop_rx,
            generated_tokens_tx,
            max_tokens,
            raw_prompt,
        )
    }
}

impl Handler<GenerateEmbeddingBatchRequest> for LlamaCppSlot {
    type Result = Result<()>;

    // Based on the example from the llama-cpp-rs repository:
    // https://github.com/utilityai/llama-cpp-rs/blob/main/examples/embeddings/src/main.rs
    fn handle(
        &mut self,
        request: GenerateEmbeddingBatchRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let generated_embedding_tx_clone = request.generated_embedding_tx.clone();

        if let Err(err) = self.generate_embedding_batch(request) {
            let msg = format!(
                "{:?}: slot {} failed to generate embeddings: {err:#}",
                self.slot_context.agent_name, self.index
            );

            error!("{msg}");

            generated_embedding_tx_clone.send(EmbeddingResult::Error(msg))?;

            return Err(err);
        }

        generated_embedding_tx_clone.send(EmbeddingResult::Done)?;

        Ok(())
    }
}
