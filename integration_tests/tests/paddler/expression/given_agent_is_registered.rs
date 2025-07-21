use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use cucumber::given;
use tokio::time::sleep;

use crate::paddler_world::PaddlerWorld;

const MAX_ATTEMPTS: usize = 30;

async fn do_check(world: &mut PaddlerWorld, agent_name: String) -> Result<()> {
    if !world.agents.instances.contains_key(&agent_name) {
        return Err(anyhow!("Agent {agent_name} does not exist in the world"));
    }

    let agents_response = world.balancer.management_client.fetch_agents().await?;
    let agent = agents_response
        .agents
        .iter()
        .find(|agent| agent.status.agent_name == agent_name)
        .ok_or_else(|| anyhow!("not found in response"))?;

    if let Some(error_value) = &agent.status.error {
        return Err(anyhow!("agent reported error: {:?}", error_value));
    }

    Ok(())
}

#[given(expr = "agent {string} is registered")]
pub async fn given_agent_is_registered(world: &mut PaddlerWorld, agent_name: String) -> Result<()> {
    let mut attempts = 0;

    while attempts < MAX_ATTEMPTS {
        sleep(Duration::from_millis(100)).await;

        if do_check(world, agent_name.clone()).await.is_ok() {
            return Ok(());
        }

        attempts += 1;
    }

    Err(anyhow!(
        "Agent '{}' is not healthy after {} attempts",
        agent_name,
        MAX_ATTEMPTS
    ))
}
