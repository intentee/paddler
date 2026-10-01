#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use hf_hub::Cache;
use hf_hub::Repo;
use hf_hub::RepoType;
use hf_hub::api::tokio::ApiBuilder;

use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::every_model_card::every_model_card;

#[tokio::test]
async fn every_model_card_resolves_from_the_hugging_face_cache() -> Result<()> {
    let hugging_face_cache = Cache::from_env();
    let hugging_face_api = ApiBuilder::from_cache(hugging_face_cache.clone()).build()?;

    for ModelCard {
        reference:
            HuggingFaceModelReference {
                filename,
                repo_id,
                revision,
            },
    } in every_model_card()
    {
        let repo = Repo::with_revision(repo_id, RepoType::Model, revision);
        let fetched_path = hugging_face_api.repo(repo.clone()).get(&filename).await?;

        assert_eq!(
            hugging_face_cache.repo(repo).get(&filename),
            Some(fetched_path)
        );
    }

    Ok(())
}
