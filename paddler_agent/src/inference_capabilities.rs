use tokio::join;

use paddler_agent_runtime::shuts_down::ShutsDown;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InferenceCapabilities<TDecisions, TEmbeddings, TTextGeneration> {
    pub decisions: TDecisions,
    pub embeddings: TEmbeddings,
    pub text_generation: TTextGeneration,
}

impl<TDecisions, TEmbeddings, TTextGeneration> ShutsDown
    for InferenceCapabilities<TDecisions, TEmbeddings, TTextGeneration>
where
    TDecisions: ShutsDown + Send,
    TEmbeddings: ShutsDown + Send,
    TTextGeneration: ShutsDown + Send,
{
    async fn shut_down(self) {
        let Self {
            decisions,
            embeddings,
            text_generation,
        } = self;

        join!(
            decisions.shut_down(),
            embeddings.shut_down(),
            text_generation.shut_down()
        );
    }
}
