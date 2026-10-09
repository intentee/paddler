use std::ffi::OsStr;
use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;

use paddler_test_cluster_harness::running_balancer::RunningBalancer;

use crate::paddler_command::paddler_command;
use crate::read_balancer_addresses::read_balancer_addresses;
use crate::spawned_balancer_subprocess::SpawnedBalancerSubprocess;
use crate::subprocess_cluster_error::SubprocessClusterError;
use crate::subprocess_process::SubprocessProcess;

pub async fn spawn_balancer_subprocess<TArguments, TArgument>(
    binary_path: &str,
    arguments: TArguments,
) -> Result<SpawnedBalancerSubprocess>
where
    TArguments: IntoIterator<Item = TArgument>,
    TArgument: AsRef<OsStr>,
{
    let mut balancer_subprocess = paddler_command(binary_path)
        .arg("balancer")
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn paddler balancer subprocess")?;
    let balancer_stdout = balancer_subprocess
        .stdout
        .take()
        .ok_or(SubprocessClusterError::StdoutNotPiped)?;
    let Some(addresses) = read_balancer_addresses(balancer_stdout).await? else {
        let exit_status = balancer_subprocess
            .wait()
            .await
            .map_err(SubprocessClusterError::ProcessExitUnobservable)?;

        return Err(SubprocessClusterError::BalancerExitedBeforeAnnouncing { exit_status }.into());
    };
    let balancer_process = SubprocessProcess::new(balancer_subprocess);
    let signals = balancer_process
        .signals()
        .ok_or(SubprocessClusterError::ProcessAlreadyReaped)?;

    Ok(SpawnedBalancerSubprocess {
        running_balancer: RunningBalancer::new(addresses, Box::new(balancer_process)),
        signals,
    })
}
