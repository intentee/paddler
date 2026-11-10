use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;

use paddler_agent_runtime::shared_llama_backend::SharedLlamaBackend;

pub struct VocabularyOnlyLlamaModel {
    llama_model: LlamaModel,
    _llama_backend: Arc<LlamaBackend>,
}

impl VocabularyOnlyLlamaModel {
    /// # Errors
    ///
    /// Returns an error when the llama backend cannot initialize, and when the vocabulary of the model cannot load.
    pub fn load(shared_llama_backend: &SharedLlamaBackend, model_path: &Path) -> Result<Self> {
        shared_llama_backend
            .get_or_init()
            .context("the llama backend must initialize")
            .and_then(|llama_backend| {
                #[expect(
                    clippy::disallowed_methods,
                    reason = "a vocabulary-only load allocates no tensors, so it holds no VRAM residency"
                )]
                let llama_model = LlamaModel::load_from_file(
                    &llama_backend,
                    model_path,
                    &LlamaModelParams::default().with_vocab_only(true),
                );

                llama_model
                    .context("the model vocabulary must load")
                    .map(|llama_model| Self {
                        llama_model,
                        _llama_backend: llama_backend,
                    })
            })
    }
}

impl Deref for VocabularyOnlyLlamaModel {
    type Target = LlamaModel;

    fn deref(&self) -> &LlamaModel {
        &self.llama_model
    }
}
