use anyhow::Context as _;
use anyhow::Result;
use log::LevelFilter;
use log::set_max_level;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_cache_dir::cache_dir::CacheDir;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
use paddler_model_source::huggingface_hub::HuggingFaceHub;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::running_balancer::RunningBalancer;

use crate::in_process_agent_spawner::InProcessAgentSpawner;
use crate::in_process_balancer::InProcessBalancer;
use crate::in_process_balancer_runner_params::in_process_balancer_runner_params;

pub async fn start_cluster(cluster_params: ClusterParams) -> Result<Cluster> {
    set_max_level(LevelFilter::Trace);

    let running_balancer =
        BalancerRunner::start(in_process_balancer_runner_params(&cluster_params)?)
            .await
            .context("failed to start in-process BalancerRunner")
            .and_then(|balancer_runner| {
                RunningBalancer::new(
                    balancer_runner.addresses,
                    Box::new(InProcessBalancer::new(balancer_runner)),
                )
                .context("failed to address the in-process balancer")
            })?;
    let management_address = running_balancer.addresses.management.to_string();
    let downloaded_models = TempDir::new()?;
    let desired_model_resolver = DesiredModelResolver {
        cache_dir: CacheDir {
            explicit: Some(downloaded_models.path().to_path_buf()),
            home: None,
            xdg: None,
        },
        download_manager: DownloadManager::new()?,
        huggingface_hub: HuggingFaceHub {
            cache_location: HuggingFaceCacheLocation::HubDefault,
            endpoint: HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
        },
    };

    let mut cluster = Cluster::connect(
        CancellationToken::new(),
        running_balancer,
        Box::new(InProcessAgentSpawner {
            desired_model_resolver,
            downloaded_models,
            management_address,
        }),
        &cluster_params.desired_state,
    )
    .await?;

    cluster
        .register_agents(&cluster_params.agents, cluster_params.wait_for_slots_ready)
        .await?;

    Ok(cluster)
}
