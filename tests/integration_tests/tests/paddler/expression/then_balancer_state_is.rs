use std::time::SystemTime;

use anyhow::Result;
use cucumber::gherkin::Step;
use cucumber::then;

use crate::assert_balancer_table::assert_balancer_table;
use crate::paddler_world::PaddlerWorld;

#[then("balancer state is:")]
pub async fn then_balancer_state_is(world: &mut PaddlerWorld, step: &Step) -> Result<()> {
    let agents_response = world.balancer.management_client.fetch_agents().await?;

    world.balancer.last_update = Some(SystemTime::now());

    if let Some(table) = step.table.as_ref() {
        assert_balancer_table(table, &agents_response)?;
    }

    Ok(())
}
