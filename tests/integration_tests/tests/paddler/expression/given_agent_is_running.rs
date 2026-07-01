use std::process::Stdio;

use anyhow::Result;
use anyhow::anyhow;
use cucumber::given;
use tokio::process::Command;

use crate::paddler_world::PaddlerWorld;

#[given(expr = "agent {string} is running \\(observes {string}\\)")]
pub async fn given_agent_is_attached(
    world: &mut PaddlerWorld,
    agent_name: String,
    llamacpp_name: String,
) -> Result<()> {
    if world.agents.instances.contains_key(&agent_name) {
        return Err(anyhow!("Agent {} is already running", agent_name));
    }

    let local_llamacpp_port = world.llamas.llamacpp_port(&llamacpp_name)?;

    world.agents.instances.insert(
        agent_name.clone(),
        Command::new("../target/debug/paddler")
            .arg("agent")
            .arg(format!("--name={agent_name}"))
            .arg(format!(
                "--local-llamacpp-addr=127.0.0.1:{local_llamacpp_port}"
            ))
            .arg("--management-addr=127.0.0.1:8095")
            .arg(format!(
                "--monitoring-interval={}",
                world.agents.monitoring_interval.unwrap_or(500)
            ))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    );

    Ok(())
}
