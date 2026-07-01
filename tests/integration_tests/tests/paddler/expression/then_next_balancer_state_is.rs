use std::time::Duration;
use std::time::SystemTime;

use anyhow::Result;
use anyhow::anyhow;
use cucumber::gherkin::Step;
use cucumber::then;
use tokio::time::sleep;

use crate::agent_response::AgentsResponse;
use crate::assert_balancer_table::assert_balancer_table;
use crate::paddler_world::PaddlerWorld;

const MAX_ATTEMPTS: usize = 30;

fn all_agents_are_updated(agents: &AgentsResponse, last_update: SystemTime) -> bool {
    for agent in &agents.agents {
        if agent.last_update < last_update {
            return false;
        }
    }

    true
}

#[then("next balancer state is:")]
pub async fn then_balancer_state_is(world: &mut PaddlerWorld, step: &Step) -> Result<()> {
    let last_update = world
        .balancer
        .last_update
        .expect("Last update does not exist");

    let mut attempts = 0;

    while attempts < MAX_ATTEMPTS {
        sleep(Duration::from_millis(100)).await;

        let agents_response = world.balancer.management_client.fetch_agents().await?;

        if all_agents_are_updated(&agents_response, last_update) {
            world.balancer.last_update = Some(SystemTime::now());

            if let Some(table) = step.table.as_ref() {
                assert_balancer_table(table, &agents_response)?;
            }

            return Ok(());
        }

        attempts += 1;
    }

    Err(anyhow!(
        "Balancer state did not update after {} attempts",
        MAX_ATTEMPTS
    ))
}
