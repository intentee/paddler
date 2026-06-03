use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ConvertsToApplicableState {
    type ApplicableState;
    type DesiredState;

    async fn to_applicable_state(
        &self,
        desired_state: Self::DesiredState,
    ) -> Result<Self::ApplicableState>;
}
