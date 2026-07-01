use anyhow::Result;
use cucumber::given;

use crate::paddler_world::PaddlerWorld;

#[given(expr = "llama.cpp completes response after {int} millisecond(s)")]
pub async fn given_llamacpp_completes_response_after_milliseconds(
    world: &mut PaddlerWorld,
    completion_response_delay: i64,
) -> Result<()> {
    world.llamas.completion_response_delay = Some(completion_response_delay);

    Ok(())
}
