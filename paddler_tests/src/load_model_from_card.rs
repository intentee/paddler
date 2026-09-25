use anyhow::Result;
use hf_hub::api::sync::ApiBuilder;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;

use crate::model_card::ModelCard;

pub fn load_model_from_card(
    llama_backend: &LlamaBackend,
    ModelCard {
        gpu_layer_count,
        reference,
    }: ModelCard,
) -> Result<LlamaModel> {
    let model_path = ApiBuilder::from_env()
        .build()?
        .model(reference.repo_id)
        .get(&reference.filename)?;

    Ok(LlamaModel::load_from_file(
        llama_backend,
        &model_path,
        &LlamaModelParams::default().with_n_gpu_layers(gpu_layer_count),
    )?)
}
