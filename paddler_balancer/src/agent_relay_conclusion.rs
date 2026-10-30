use crate::relay_failure::RelayFailure;

pub enum AgentRelayConclusion<TFinal> {
    AgentConcluded {
        final_result: TFinal,
        generated_by: Option<String>,
    },
    Failed(RelayFailure),
}
