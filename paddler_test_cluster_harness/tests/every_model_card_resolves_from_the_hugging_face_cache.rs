#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_model_source::desired_model_resolution::DesiredModelResolution;
use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
use paddler_model_source::huggingface_hub::HuggingFaceHub;
use paddler_model_source::huggingface_model_source::HuggingFaceModelSource;
use paddler_model_source::resolves_model_source::ResolvesModelSource as _;
use paddler_test_cluster_harness::model_card::every_model_card::every_model_card;

#[tokio::test]
async fn every_model_card_resolves_from_the_hugging_face_cache() {
    for model_card in every_model_card() {
        let resolution = HuggingFaceModelSource {
            client: HuggingFaceHub {
                cache_location: HuggingFaceCacheLocation::HubDefault,
                endpoint: HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
            }
            .client()
            .expect("the Hugging Face client must start"),
            reference: model_card.reference.clone(),
        }
        .resolve(
            &CancellationToken::new(),
            Arc::new(SlotAggregatedStatus::new(1)),
        )
        .await
        .expect("every model card must resolve through the Hugging Face hub");

        assert_eq!(
            resolution,
            DesiredModelResolution::Resolved(
                model_card
                    .cached_path()
                    .expect("every resolved model card must be in the Hugging Face cache")
            )
        );
    }
}
