use anyhow::Result;
use cucumber::given;

use crate::paddler_world::PaddlerWorld;

#[given(expr = "balancer allows CORS host {string}")]
pub async fn given_balancer_allows_cors_host(
    world: &mut PaddlerWorld,
    allowed_host: String,
) -> Result<()> {
    world.balancer.allowed_cors_hosts.push(allowed_host);

    Ok(())
}
