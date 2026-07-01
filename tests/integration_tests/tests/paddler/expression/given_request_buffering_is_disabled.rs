use anyhow::Result;
use cucumber::given;

use crate::paddler_world::PaddlerWorld;

#[given(expr = "request buffering is disabled")]
pub async fn given_request_buffering_is_disabled(world: &mut PaddlerWorld) -> Result<()> {
    world.balancer.max_buffered_requests = Some(0);

    Ok(())
}
