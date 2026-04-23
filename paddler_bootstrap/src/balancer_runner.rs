use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler::balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::bootstrapped_balancer_handle::BalancerBootstrapConfig;
use crate::bootstrapped_balancer_handle::BootstrappedBalancerHandle;
use crate::bootstrapped_balancer_handle::bootstrap_balancer;
use crate::service_thread::ServiceThread;

pub struct BalancerRunnerParams {
    pub buffered_request_timeout: Duration,
    pub inference_service_configuration: InferenceServiceConfiguration,
    pub management_service_configuration: ManagementServiceConfiguration,
    pub max_buffered_requests: i32,
    pub openai_service_configuration: Option<OpenAIServiceConfiguration>,
    pub parent_shutdown: Option<CancellationToken>,
    pub state_database_type: StateDatabaseType,
    pub statsd_prefix: String,
    pub statsd_service_configuration: Option<StatsdServiceConfiguration>,
    #[cfg(feature = "web_admin_panel")]
    pub web_admin_panel_service_configuration: Option<WebAdminPanelServiceConfiguration>,
}

pub struct BalancerRunner {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_tx: broadcast::Sender<BalancerDesiredState>,
    pub initial_desired_state: BalancerDesiredState,
    thread: ServiceThread,
}

impl BalancerRunner {
    pub async fn start(
        BalancerRunnerParams {
            buffered_request_timeout,
            inference_service_configuration,
            management_service_configuration,
            max_buffered_requests,
            openai_service_configuration,
            parent_shutdown,
            state_database_type,
            statsd_prefix,
            statsd_service_configuration,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        }: BalancerRunnerParams,
    ) -> Result<Self> {
        let BootstrappedBalancerHandle {
            agent_controller_pool,
            balancer_applicable_state_holder,
            balancer_desired_state_tx,
            service_manager,
            state_database,
        } = bootstrap_balancer(BalancerBootstrapConfig {
            buffered_request_timeout,
            inference_service_configuration,
            management_service_configuration,
            max_buffered_requests,
            openai_service_configuration,
            state_database_type,
            statsd_prefix,
            statsd_service_configuration,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        })
        .await?;

        let initial_desired_state = state_database.read_balancer_desired_state().await?;

        let thread = ServiceThread::spawn(parent_shutdown, move |task_shutdown| async move {
            service_manager.run_forever(task_shutdown).await
        });

        Ok(Self {
            agent_controller_pool,
            balancer_applicable_state_holder,
            balancer_desired_state_tx,
            initial_desired_state,
            thread,
        })
    }

    pub fn wait_for_completion(&mut self) -> impl Future<Output = Result<()>> + Send + 'static {
        self.thread.wait_for_completion()
    }

    pub fn cancel(&self) {
        self.thread.cancel();
    }
}
