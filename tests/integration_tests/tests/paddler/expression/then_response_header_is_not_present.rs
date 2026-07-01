use anyhow::Result;
use anyhow::anyhow;
use cucumber::then;

use crate::paddler_world::PaddlerWorld;

#[then(expr = "{string} response header {string} is not present")]
pub async fn then_response_header_is(
    world: &mut PaddlerWorld,
    name: String,
    header_name: String,
) -> Result<()> {
    let response = world
        .responses
        .get(&name)
        .ok_or_else(|| anyhow!("No request found with the name: {}", name))?;

    let header_value = response.headers().get(&header_name);

    if header_value.is_some() {
        return Err(anyhow!(
            "Response header '{:?}' is present with value: {:?}",
            header_name,
            header_value.expect("Header should not be present")
        ));
    }

    Ok(())
}
