#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;
use hf_hub::api::tokio::ApiBuilder;

use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::every_model_card::every_model_card;

#[tokio::test]
async fn every_model_card_resolves_from_the_hugging_face_cache() {
    let hugging_face_cache = Cache::from_env();
    let hugging_face_api = ApiBuilder::from_cache(hugging_face_cache.clone())
        .build()
        .expect("the Hugging Face API client must build");

    for ModelCard { reference } in every_model_card() {
        let fetched_path = hugging_face_api
            .repo(reference.repo())
            .get(&reference.filename)
            .await
            .expect("every model card must resolve through the Hugging Face API");

        assert_eq!(
            reference.cached_path(&hugging_face_cache),
            Some(fetched_path)
        );
    }
}
