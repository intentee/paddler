use anyhow::Result;
use cucumber::given;

use crate::paddler_world::PaddlerWorld;

#[given(expr = "balancer reports metrics every {int} millisecond(s)")]
pub async fn given_agent_monitors_llamacpp_every_milliseconds(
    world: &mut PaddlerWorld,
    statsd_reporting_interval: i64,
) -> Result<()> {
    world.balancer.statsd_reporting_interval = Some(statsd_reporting_interval);

    Ok(())
}
