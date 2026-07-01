use anyhow::Result;
use cucumber::given;

use crate::paddler_world::PaddlerWorld;

#[given(expr = "request {string} has header {string} with value {string}")]
pub async fn given_request_has_header_with_value(
    world: &mut PaddlerWorld,
    name: String,
    header_name: String,
    header_value: String,
) -> Result<()> {
    world
        .request_builder
        .headers_to_be_set
        .insert_header(name, (header_name, header_value));

    Ok(())
}
