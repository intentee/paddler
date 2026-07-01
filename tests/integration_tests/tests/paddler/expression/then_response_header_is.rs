use anyhow::Result;
use anyhow::anyhow;
use cucumber::then;

use crate::paddler_world::PaddlerWorld;

#[then(expr = "{string} response header {string} is {string}")]
pub async fn then_response_header_is(
    world: &mut PaddlerWorld,
    name: String,
    header_name: String,
    expected_header_value: String,
) -> Result<()> {
    let response = world
        .responses
        .get(&name)
        .ok_or_else(|| anyhow!("No request found with the name: {}", name))?;

    let header = response.headers().get(&header_name).ok_or_else(|| {
        anyhow!(
            "Header {} not found in response, got headers: {:?}",
            header_name,
            response.headers()
        )
    })?;

    let header_value_str = header
        .to_str()
        .map_err(|_| anyhow!("Failed to convert header value to string"))?;

    assert_eq!(
        header_value_str, expected_header_value,
        "Expected header value '{expected_header_value}' but got '{header_value_str}'",
    );

    Ok(())
}
