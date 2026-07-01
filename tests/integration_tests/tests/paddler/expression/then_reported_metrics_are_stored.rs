use core::panic;

use anyhow::Result;
use cucumber::then;

use crate::metrics::Metrics;
use crate::paddler_world::PaddlerWorld;

async fn fetch_metrics(statsd_port: u16) -> Result<Metrics> {
    let response = reqwest::get(format!("http://localhost:{statsd_port}/metrics")).await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch metrics: Expected status 200, got {}",
            response.status()
        ));
    }

    let metrics = response.json::<Metrics>().await?;

    Ok(metrics)
}

#[then(expr = "reported metrics are stored")]
pub async fn then_metrics_are_stored(world: &mut PaddlerWorld) -> Result<()> {
    let metrics = fetch_metrics(9102).await?;
    world.statsd.metrics.push(metrics);

    Ok(())
}
