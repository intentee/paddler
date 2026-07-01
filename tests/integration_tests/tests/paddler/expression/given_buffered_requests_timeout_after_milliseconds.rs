use anyhow::Result;
use cucumber::given;

use crate::paddler_world::PaddlerWorld;

#[given(expr = "buffered requests timeout after {int} millisecond(s)")]
pub async fn given_buffered_requests_timeout_after_milliseconds(
    world: &mut PaddlerWorld,
    buffered_request_timeout: i64,
) -> Result<()> {
    world.balancer.buffered_request_timeout = Some(buffered_request_timeout);

    Ok(())
}
