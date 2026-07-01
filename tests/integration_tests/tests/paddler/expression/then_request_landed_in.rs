use anyhow::Result;
use anyhow::anyhow;
use cucumber::then;

use crate::paddler_world::PaddlerWorld;

#[then(expr = "{string} request landed in {string}")]
pub async fn then_request_landed_in(
    world: &mut PaddlerWorld,
    request_name: String,
    llamacpp_name: String,
) -> Result<()> {
    let llamacpp = world
        .llamas
        .instances
        .get(&llamacpp_name)
        .ok_or_else(|| anyhow!("Llama.cpp server {} not found", llamacpp_name))?;

    let accepted_result = llamacpp.accepted_request(&request_name).await?;

    if !accepted_result.accepted {
        return Err(anyhow!(
            "Request '{}' did not land in Llama.cpp server '{}'.\nLogs: {}",
            request_name,
            llamacpp_name,
            accepted_result.contents,
        ));
    }

    Ok(())
}
