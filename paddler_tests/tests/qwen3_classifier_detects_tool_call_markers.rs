#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_classifier_detects_tool_call_markers() -> Result<()> {
    let ModelCard {
        reference:
            HuggingFaceModelReference {
                filename,
                repo_id,
                revision,
            },
        ..
    } = qwen3_0_6b();

    let api = hf_hub::api::sync::ApiBuilder::from_env()
        .build()
        .context("failed to build hf-hub API client")?;
    let model_path = api
        .repo(hf_hub::Repo::with_revision(
            repo_id,
            hf_hub::RepoType::Model,
            revision,
        ))
        .get(&filename)
        .context("failed to fetch Qwen3 model from Hugging Face")?;

    let backend = LlamaBackend::init().context("failed to init llama backend")?;
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
        .context("failed to load Qwen3 model")?;

    let classifier = model
        .sampled_token_classifier()
        .context("failed to build sampled-token classifier for Qwen3")?;

    assert!(
        classifier.markers().reasoning.is_some(),
        "expected Qwen3 to expose reasoning markers; got {:?}",
        classifier.markers()
    );

    let (no_tools, with_tools) = model
        .diagnose_tool_call_synthetic_renders()
        .context("failed to render synthetic templates for diagnosis")?;

    assert!(
        classifier.markers().tool_call.is_some(),
        "expected Qwen3 to expose tool-call markers; got markers={:?}\n--- no_tools render ---\n{no_tools}\n--- with_tools render ---\n{with_tools}",
        classifier.markers()
    );

    Ok(())
}
