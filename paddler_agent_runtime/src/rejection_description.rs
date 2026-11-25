use std::error::Error;

use paddler_error_chain::error_chain::ErrorChain;

pub fn rejection_description(agent_name: Option<&str>, rejection: &dyn Error) -> String {
    let rejection_chain = ErrorChain { error: rejection };

    agent_name.map_or_else(
        || rejection_chain.to_string(),
        |agent_name| format!("{agent_name}: {rejection_chain}"),
    )
}
