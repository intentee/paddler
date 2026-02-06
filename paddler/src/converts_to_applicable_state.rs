use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ConvertsToApplicableState {
    type ApplicableState;
    type Context;

    async fn to_applicable_state(
        &self,
        context: Self::Context,
    ) -> Result<Option<Self::ApplicableState>>;
}
