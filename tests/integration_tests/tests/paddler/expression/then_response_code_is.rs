use anyhow::Result;
use anyhow::anyhow;
use cucumber::then;

use crate::paddler_world::PaddlerWorld;

#[then(expr = "{string} response code is {int}")]
pub async fn then_response_code_is(
    world: &mut PaddlerWorld,
    name: String,
    expected_code: u16,
) -> Result<()> {
    let response = world
        .responses
        .get(&name)
        .ok_or_else(|| anyhow!("No request found with the name: {name}"))?;

    let status = response.status();
    if status.as_u16() != expected_code {
        return Err(anyhow!(
            "Expected status code {expected_code}, but got {status}"
        ));
    }

    Ok(())
}
