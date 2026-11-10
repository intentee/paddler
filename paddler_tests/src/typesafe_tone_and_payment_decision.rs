use anyhow::Context as _;
use anyhow::Result;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use serde_json::json;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::system_one_request::system_one_request;
use paddler_test_cluster_harness::typesafe_api_response::TypeSafeApiResponse;

use crate::constants::TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID;
use crate::decision_cluster_params::DecisionClusterParams;

async fn decide_the_tone_and_payment(cluster: Cluster) -> Result<TypeSafeApiResponse> {
    cluster
        .typesafe_system_one(
            TYPESAFE_TONE_AND_PAYMENT_REQUEST_ID,
            &system_one_request(
                &json!("The invoice was paid on time."),
                &json!({
                    "tone": {"type": "choice", "criteria": {"calm": null, "angry": "Shouting"}},
                    "paid": {"type": "noul", "instructions": "Was it paid?"},
                }),
            ),
        )
        .map(|answered| answered.context("the System One request must be answered"))
        .and_then(|typesafe_decision| {
            cluster
                .shutdown()
                .map(|shut_down| shut_down.context("the cluster must shut down cleanly"))
                .map_ok(|()| typesafe_decision)
        })
        .await
}

/// # Errors
///
/// Returns an error when the decision cluster cannot start, when the System One request is not answered, and when the cluster does not shut down cleanly.
pub async fn typesafe_tone_and_payment_decision() -> Result<TypeSafeApiResponse> {
    DecisionClusterParams::default()
        .start_cluster()
        .map(|started| started.context("the cluster must start"))
        .and_then(decide_the_tone_and_payment)
        .await
}
