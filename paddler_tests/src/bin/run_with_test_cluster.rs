use std::net::SocketAddr;
use std::process::ExitCode;
use std::process::ExitStatus;

use anyhow::Error;
use anyhow::Result;
use clap::Parser;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
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

    ready(preset.cluster_params())
        .err_into()
        .and_then(start_cluster)
        .and_then(|cluster| run_against_cluster(cluster, program, arguments))
        .await
        .map(exit_code)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsStr;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;

    use paddler_balancer::balancer_addresses::BalancerAddresses;

    use super::command_against_cluster;

    #[test]
    fn exports_no_url_for_a_compatibility_service_the_cluster_does_not_serve() {
        let command = command_against_cluster(
            "printenv",
            &[],
            &BalancerAddresses {
                compat_openai: None,
                compat_typesafe: None,
                inference: SocketAddr::from((Ipv4Addr::LOCALHOST, 8061)),
                management: SocketAddr::from((Ipv4Addr::LOCALHOST, 8060)),
                web_admin_panel: None,
            },
        );

        assert_eq!(
            command.as_std().get_envs().collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                (
                    OsStr::new("PADDLER_INFERENCE_URL"),
                    Some(OsStr::new("http://127.0.0.1:8061"))
                ),
                (
                    OsStr::new("PADDLER_MANAGEMENT_URL"),
                    Some(OsStr::new("http://127.0.0.1:8060"))
                ),
            ])
        );
    }
}
