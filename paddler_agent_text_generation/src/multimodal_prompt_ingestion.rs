use std::sync::Arc;

use llama_cpp_bindings::SampledTokenClassifier;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::error::EvalMultimodalChunksError;
use llama_cpp_bindings::mtmd::FittedMtmdInputChunk;
use llama_cpp_bindings::mtmd::FittedMtmdInputChunks;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdInputChunks;

use paddler_agent_runtime::sequence_id::SequenceId;

use crate::multimodal_ingestion_progress::MultimodalIngestionProgress;

#[derive(Debug)]
pub struct MultimodalPromptIngestion {
    pub chunks: FittedMtmdInputChunks<MtmdInputChunks>,
    pub multimodal_context: Arc<MtmdContext>,
    pub n_batch: i32,
    pub next_chunk_index: usize,
}

impl MultimodalPromptIngestion {
    pub fn ingest_next_chunk(
        &mut self,
        token_classifier: &mut SampledTokenClassifier<'_>,
        llama_context: &LlamaContext,
        sequence_id: SequenceId,
        start_position: i32,
    ) -> Result<MultimodalIngestionProgress, EvalMultimodalChunksError> {
        let chunk_index = self.next_chunk_index;
        let chunk = FittedMtmdInputChunk::at(&self.chunks, chunk_index)
            .ok_or(EvalMultimodalChunksError::ChunkOutOfBounds(chunk_index))?;
        let is_last_chunk = chunk_index + 1 == self.chunks.len();
        let next_position = chunk.eval_single(
            &self.multimodal_context,
            llama_context,
            start_position,
            sequence_id.llama_sequence_id(),
            self.n_batch,
            is_last_chunk,
        )?;

        token_classifier.ingest_fitted_chunk(&chunk);

        self.next_chunk_index += 1;

        Ok(if is_last_chunk {
            MultimodalIngestionProgress::PromptIngested { next_position }
        } else {
            MultimodalIngestionProgress::ChunksRemain { next_position }
        })
    }
}
