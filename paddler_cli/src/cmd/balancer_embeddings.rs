use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use command_handler::handler::Handler;
use tokio_util::sync::CancellationToken;

use paddler_balancer_runner::balancer_serving_mode::BalancerServingMode;

use super::balancer_shared_arguments::BalancerSharedArguments;

#[derive(Args)]
pub struct BalancerEmbeddings {
    #[command(flatten)]
    shared_arguments: BalancerSharedArguments,
}

#[async_trait(?Send)]
impl Handler for BalancerEmbeddings {
    async fn handle(self, shutdown: CancellationToken) -> Result<()> {
        self.shared_arguments
            .run(BalancerServingMode::Embeddings, shutdown)
            .await
    }
}
