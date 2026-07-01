use anyhow::Result;
use cucumber::when;

use crate::paddler_world::PaddlerWorld;

#[when(expr = "request {string} is sent to management endpoint {string}")]
pub async fn when_request_is_sent_to_management_endpoint(
    world: &mut PaddlerWorld,
    name: String,
    path: String,
) -> Result<()> {
    let request = world
        .request_builder
        .get(&name, format!("http://127.0.0.1:8095{path}"));
    let response = request.send().await?;

    world.responses.insert(name, response);

    Ok(())
}
