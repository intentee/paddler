#![cfg(feature = "tests_that_use_llms")]

use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;

use paddler_agent_runtime::shared_llama_backend::SharedLlamaBackend;

pub struct VocabularyOnlyLlamaModel {
    llama_model: LlamaModel,
    _llama_backend: Arc<LlamaBackend>,
}

impl VocabularyOnlyLlamaModel {
    pub fn load(shared_llama_backend: &SharedLlamaBackend, model_path: &Path) -> Self {
        let llama_backend = shared_llama_backend
            .get_or_init()
            .expect("the llama backend must initialize");

        #[expect(
            clippy::disallowed_methods,
            reason = "a vocabulary-only load allocates no tensors, so it holds no VRAM residency"
        )]
        let llama_model = LlamaModel::load_from_file(
            &llama_backend,
            model_path,
            &LlamaModelParams::default().with_vocab_only(true),
        )
        .expect("the model vocabulary must load");

        Self {
            llama_model,
            _llama_backend: llama_backend,
        }
    }
}

impl Deref for VocabularyOnlyLlamaModel {
    type Target = LlamaModel;

    fn deref(&self) -> &LlamaModel {
        &self.llama_model
    }
}
