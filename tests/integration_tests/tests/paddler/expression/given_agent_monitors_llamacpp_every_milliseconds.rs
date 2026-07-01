use anyhow::Result;
use cucumber::given;

use crate::paddler_world::PaddlerWorld;

#[given(expr = "agent monitors llama.cpp every {int} millisecond(s)")]
pub async fn given_agent_monitors_llamacpp_every_milliseconds(
    world: &mut PaddlerWorld,
    monitoring_interval: i64,
) -> Result<()> {
    world.agents.monitoring_interval = Some(monitoring_interval);

    Ok(())
}
