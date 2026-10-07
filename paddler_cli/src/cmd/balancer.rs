use anyhow::Result;
use async_trait::async_trait;
use clap::Subcommand;
use command_handler::handler::Handler;
use tokio_util::sync::CancellationToken;

use super::balancer_decision::BalancerDecision;
use super::balancer_embeddings::BalancerEmbeddings;
use super::balancer_text_generation::BalancerTextGeneration;

#[derive(Subcommand)]
pub enum Balancer {
    /// Answers questions about a document with a kev decision model
    Decision(Box<BalancerDecision>),
    /// Serves embeddings for documents
    Embeddings(Box<BalancerEmbeddings>),
    /// Generates tokens from conversations and raw prompts
    TextGeneration(Box<BalancerTextGeneration>),
}

#[async_trait(?Send)]
impl Handler for Balancer {
    async fn handle(self, shutdown: CancellationToken) -> Result<()> {
        match self {
            Self::Decision(balancer_decision) => (*balancer_decision).handle(shutdown).await,
            Self::Embeddings(balancer_embeddings) => (*balancer_embeddings).handle(shutdown).await,
            Self::TextGeneration(balancer_text_generation) => {
                (*balancer_text_generation).handle(shutdown).await
            }
        }
    }
}
