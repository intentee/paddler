use std::num::NonZeroUsize;

use crate::agent_runtime_error::AgentRuntimeError;
use crate::inference_thread_count::inference_thread_count;

#[derive(Clone, Copy, Debug)]
pub struct InferenceParallelism {
    pub available_parallelism: NonZeroUsize,
    pub inference_thread_count: i32,
}

impl InferenceParallelism {
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::ThreadCountOutOfRange`] when the available parallelism does not fit the thread count llama.cpp takes.
    pub fn of_available_parallelism(
        available_parallelism: NonZeroUsize,
    ) -> Result<Self, AgentRuntimeError> {
        inference_thread_count(available_parallelism).map(|inference_thread_count| Self {
            available_parallelism,
            inference_thread_count,
        })
    }
}
