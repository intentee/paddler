use std::error::Error;

use paddler_error_chain::error_chain::ErrorChain;

pub fn rejection_description(agent_name: Option<&str>, rejection: &dyn Error) -> String {
    let rejection_chain = ErrorChain { error: rejection };

    agent_name.map_or_else(
        || rejection_chain.to_string(),
        |agent_name| format!("{agent_name}: {rejection_chain}"),
    )
}

#[cfg(test)]
mod tests {
    use thiserror::Error;

    use super::rejection_description;

    #[derive(Debug, Error)]
    #[error("the model file is truncated")]
    struct ModelFileTruncated;

    #[derive(Debug, Error)]
    #[error("no model is loaded")]
    struct ModelNotLoaded {
        #[source]
        source: ModelFileTruncated,
    }

    #[test]
    fn prefixes_the_error_chain_with_the_agent_name() {
        assert_eq!(
            rejection_description(
                Some("agent"),
                &ModelNotLoaded {
                    source: ModelFileTruncated
                }
            ),
            "agent: no model is loaded: the model file is truncated"
        );
    }

    #[test]
    fn describes_the_error_chain_alone_for_an_unnamed_agent() {
        assert_eq!(
            rejection_description(
                None,
                &ModelNotLoaded {
                    source: ModelFileTruncated
                }
            ),
            "no model is loaded: the model file is truncated"
        );
    }
}
