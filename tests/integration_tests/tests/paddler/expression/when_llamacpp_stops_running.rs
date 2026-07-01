use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use cucumber::when;
use tokio::time::sleep;

use crate::paddler_world::PaddlerWorld;

const MAX_ATTEMPTS: usize = 30;

async fn do_check(llamacpp_port: u16) -> Result<()> {
    let response = reqwest::get(format!("http://127.0.0.1:{llamacpp_port}/health")).await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Health check failed: Expected status 200, got {}",
            response.status()
        ));
    }

    let body = response.text().await?;

    if body.trim() != "OK" {
        return Err(anyhow!(
            "Health check failed: Expected 'OK', got '{}'",
            body
        ));
    }

    Ok(())
}

#[when(expr = "llama.cpp server {string} stops running")]
pub async fn when_llamacpp_stops_running(
    world: &mut PaddlerWorld,
    llamacpp_name: String,
) -> Result<()> {
    if !world.llamas.instances.contains_key(&llamacpp_name) {
        return Err(anyhow!("Llama.cpp server {} is not running", llamacpp_name));
    }

    let llamacpp_port = world.llamas.llamacpp_port(&llamacpp_name)?;

    world.llamas.kill(&llamacpp_name).await?;

    let mut attempts = 0;

    while attempts < MAX_ATTEMPTS {
        sleep(Duration::from_millis(100)).await;

        if do_check(llamacpp_port).await.is_err() {
            return Ok(());
        } else {
            eprintln!(
                "Attempt {}: llama.cpp at port {} is still alive.",
                attempts + 1,
                llamacpp_port
            );
        }

        attempts += 1;
    }

    Err(anyhow!(
        "Llama.cpp server at port {} is still running after {} attempts",
        llamacpp_port,
        MAX_ATTEMPTS
    ))
}
