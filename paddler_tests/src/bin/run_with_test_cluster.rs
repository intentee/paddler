use std::net::SocketAddr;
use std::process::ExitCode;
use std::process::ExitStatus;

use anyhow::Error;
use anyhow::Result;
use clap::Parser;
use futures_util::TryFutureExt as _;
use tokio::process::Command;

use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_tests::start_cluster::start_cluster;
use paddler_tests::test_cluster_preset::TestClusterPreset;

fn http_url(addr: SocketAddr) -> String {
    format!("http://{addr}")
}

fn command_against_cluster(
    program: &str,
    arguments: &[String],
    BalancerAddresses {
        compat_openai,
        compat_typesafe,
        inference,
        management,
        ..
    }: &BalancerAddresses,
) -> Command {
    let mut command = Command::new(program);

    command
        .args(arguments)
        .env("PADDLER_INFERENCE_URL", http_url(*inference))
        .env("PADDLER_MANAGEMENT_URL", http_url(*management));

    if let Some(compat_openai) = compat_openai {
        command.env("PADDLER_COMPAT_OPENAI_URL", http_url(*compat_openai));
    }

    if let Some(compat_typesafe) = compat_typesafe {
        command.env("PADDLER_COMPAT_TYPESAFE_URL", http_url(*compat_typesafe));
    }

    command
}

async fn run_against_cluster(
    cluster: Cluster,
    program: String,
    arguments: Vec<String>,
) -> Result<ExitStatus> {
    let command_status = command_against_cluster(&program, &arguments, &cluster.balancer.addresses)
        .status()
        .await
        .map_err(Error::from);

    cluster.shutdown().await.and(command_status)
}

fn exit_code(command_status: ExitStatus) -> ExitCode {
    if command_status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[derive(Parser)]
struct RunWithTestClusterArguments {
    preset: TestClusterPreset,
    program: String,
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    arguments: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let RunWithTestClusterArguments {
        preset,
        program,
        arguments,
    } = RunWithTestClusterArguments::parse();

    start_cluster(preset.cluster_params())
        .and_then(|cluster| run_against_cluster(cluster, program, arguments))
        .await
        .map(exit_code)
}
