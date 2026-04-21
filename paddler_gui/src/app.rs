use std::mem;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;

use iced::Bottom;
use iced::Center;
use iced::Element;
use iced::Fill;
use iced::Right;
use iced::Subscription;
use iced::Task;
use iced::futures::SinkExt;
use iced::keyboard;
use iced::widget::column;
use iced::widget::container;
use iced::widget::image;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::operation;
use iced::widget::stack;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler::produces_snapshot::ProducesSnapshot;
use paddler::slot_aggregated_status::SlotAggregatedStatus;
use paddler_bootstrap::bootstrap_agent_params::BootstrapAgentParams;
use paddler_bootstrap::bootstrap_balancer_params::BootstrapBalancerParams;
use paddler_bootstrap::bootstrapped_agent_handle::bootstrap_agent;
use paddler_bootstrap::bootstrapped_balancer_handle::bootstrap_balancer;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::agent_running_handler;
use crate::current_screen::CurrentScreen;
use crate::home_data::HomeData;
use crate::home_handler;
use crate::join_cluster_config_handler;
use crate::message::Message;
use crate::running_cluster_handler;
use crate::screen::AgentRunning;
use crate::screen::Screen;
use crate::start_cluster_config_handler;
use crate::ui::variables::SPACING_2X;
use crate::ui::variables::SPACING_BASE;
use crate::ui::view_agent_running::view_agent_running;
use crate::ui::view_home::view_home;
use crate::ui::view_join_cluster_config::view_join_cluster_config;
use crate::ui::view_running_cluster::view_running_cluster;
use crate::ui::view_start_cluster_config::view_start_cluster_config;
use crate::wait_for_bootstrapped_agent_controller_pool::wait_for_bootstrapped_agent_controller_pool;

static BETA_IMAGE: LazyLock<ImageHandle> = LazyLock::new(|| {
    ImageHandle::from_bytes(include_bytes!("../../resources/images/beta.png").as_slice())
});

fn send_shutdown(sender: &mut Option<oneshot::Sender<()>>, label: &str) {
    if let Some(shutdown_tx) = sender.take()
        && let Err(unsent_signal) = shutdown_tx.send(())
    {
        log::error!("Failed to send {label} shutdown signal: {unsent_signal:?}");
    }
}

fn collect_sorted_agent_snapshots(
    pool: &AgentControllerPool,
) -> anyhow::Result<Vec<paddler_types::agent_controller_snapshot::AgentControllerSnapshot>> {
    let pool_snapshot = pool.make_snapshot()?;
    let mut agents = pool_snapshot.agents;

    agents.sort_by(|current_agent, other_agent| {
        let current_name = current_agent.name.as_deref().unwrap_or(&current_agent.id);
        let other_name = other_agent.name.as_deref().unwrap_or(&other_agent.id);

        current_name.cmp(other_name)
    });

    Ok(agents)
}

pub struct App {
    agent_shutdown_tx: Option<oneshot::Sender<()>>,
    screen: CurrentScreen,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Drop for App {
    fn drop(&mut self) {
        send_shutdown(&mut self.shutdown_tx, "cluster");
        send_shutdown(&mut self.agent_shutdown_tx, "agent");
    }
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self {
            agent_shutdown_tx: None,
            screen: CurrentScreen::default(),
            shutdown_tx: None,
        };

        (app, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let screen = mem::take(&mut self.screen);

        match (screen, message) {
            (CurrentScreen::Home(home), Message::Home(msg)) => {
                let action = HomeData::update(msg);

                match action {
                    home_handler::Action::StartCluster => {
                        self.screen = CurrentScreen::StartClusterConfig(home.start_cluster());

                        Task::none()
                    }
                    home_handler::Action::JoinCluster => {
                        self.screen = CurrentScreen::JoinClusterConfig(home.join_cluster());

                        Task::none()
                    }
                }
            }
            (CurrentScreen::JoinClusterConfig(mut config), Message::JoinClusterConfig(msg)) => {
                let action = config.state_data.update(msg);

                match action {
                    join_cluster_config_handler::Action::None => {
                        self.screen = CurrentScreen::JoinClusterConfig(config);

                        Task::none()
                    }
                    join_cluster_config_handler::Action::Cancel => {
                        self.screen = CurrentScreen::Home(config.cancel());

                        Task::none()
                    }
                    join_cluster_config_handler::Action::ConnectAgent {
                        agent_name,
                        management_address,
                        slots,
                    } => self.spawn_agent(config.connect(), agent_name, management_address, slots),
                }
            }
            (CurrentScreen::StartClusterConfig(mut config), Message::StartClusterConfig(msg)) => {
                let action = config.state_data.update(msg);

                match action {
                    start_cluster_config_handler::Action::None => {
                        self.screen = CurrentScreen::StartClusterConfig(config);

                        Task::none()
                    }
                    start_cluster_config_handler::Action::Cancel => {
                        send_shutdown(&mut self.shutdown_tx, "cluster");
                        self.screen = CurrentScreen::Home(config.cancel());

                        Task::none()
                    }
                    start_cluster_config_handler::Action::StartCluster {
                        management_addr,
                        inference_addr,
                        desired_state,
                    } => {
                        self.screen = CurrentScreen::StartClusterConfig(config);

                        self.spawn_cluster(management_addr, inference_addr, desired_state)
                    }
                }
            }
            (CurrentScreen::StartClusterConfig(config), Message::ClusterStarted) => {
                self.screen = CurrentScreen::RunningCluster(config.cluster_started());

                Task::none()
            }
            (CurrentScreen::StartClusterConfig(config), Message::ClusterFailed(error)) => {
                log::error!("Cluster failed to start: {error}");
                self.shutdown_tx = None;
                self.screen = CurrentScreen::Home(config.cluster_failed(error));

                Task::none()
            }
            (CurrentScreen::RunningCluster(mut running), Message::RunningCluster(msg)) => {
                let action = running.state_data.update(msg);

                match action {
                    running_cluster_handler::Action::None => {
                        self.screen = CurrentScreen::RunningCluster(running);

                        Task::none()
                    }
                    running_cluster_handler::Action::Stop => {
                        send_shutdown(&mut self.shutdown_tx, "cluster");
                        self.screen = CurrentScreen::RunningCluster(running);

                        Task::none()
                    }
                    running_cluster_handler::Action::CopyToClipboard(content) => {
                        self.screen = CurrentScreen::RunningCluster(running);

                        iced::clipboard::write::<Message>(content).discard()
                    }
                }
            }
            (CurrentScreen::RunningCluster(running), Message::ClusterStopped) => {
                self.screen = CurrentScreen::Home(running.cluster_stopped());

                Task::none()
            }
            (CurrentScreen::RunningCluster(running), Message::ClusterFailed(error)) => {
                log::error!("Cluster failed unexpectedly: {error}");
                self.shutdown_tx = None;
                self.screen = CurrentScreen::Home(running.cluster_failed(error));

                Task::none()
            }
            (CurrentScreen::AgentRunning(mut running), Message::AgentRunning(msg)) => {
                let action = running.state_data.update(msg);

                match action {
                    agent_running_handler::Action::None => {
                        self.screen = CurrentScreen::AgentRunning(running);

                        Task::none()
                    }
                    agent_running_handler::Action::Disconnect => {
                        send_shutdown(&mut self.agent_shutdown_tx, "agent");
                        self.screen = CurrentScreen::Home(running.disconnect());

                        Task::none()
                    }
                }
            }
            (CurrentScreen::AgentRunning(running), Message::AgentStopped) => {
                log::info!("Agent stopped");
                self.agent_shutdown_tx = None;
                self.screen = CurrentScreen::Home(running.disconnect());

                Task::none()
            }
            (CurrentScreen::AgentRunning(running), Message::AgentFailed(error)) => {
                log::error!("Agent failed: {error}");
                self.agent_shutdown_tx = None;
                self.screen = CurrentScreen::Home(running.agent_failed(error));

                Task::none()
            }
            (screen, Message::TabPressed { shift }) => {
                self.screen = screen;

                if shift {
                    operation::focus_previous()
                } else {
                    operation::focus_next()
                }
            }
            (screen, message) => {
                log::warn!("Unhandled message {message:?} for current screen");
                self.screen = screen;

                Task::none()
            }
        }
    }

    #[expect(
        clippy::unused_self,
        reason = "signature required by iced application API"
    )]
    pub fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Tab),
                modifiers,
                ..
            } => Some(Message::TabPressed {
                shift: modifiers.shift(),
            }),
            _ => None,
        })
    }

    pub fn view(&self) -> Element<'_, Message> {
        let screen_content = match &self.screen {
            CurrentScreen::AgentRunning(screen) => {
                view_agent_running(&screen.state_data).map(Message::AgentRunning)
            }
            CurrentScreen::Home(screen) => view_home(&screen.state_data).map(Message::Home),
            CurrentScreen::JoinClusterConfig(screen) => {
                view_join_cluster_config(&screen.state_data).map(Message::JoinClusterConfig)
            }
            CurrentScreen::StartClusterConfig(screen) => {
                view_start_cluster_config(&screen.state_data).map(Message::StartClusterConfig)
            }
            CurrentScreen::RunningCluster(screen) => {
                view_running_cluster(&screen.state_data).map(Message::RunningCluster)
            }
        };

        let content_column = column![screen_content]
            .max_width(700)
            .padding([SPACING_2X * 2.0, SPACING_BASE])
            .spacing(SPACING_BASE)
            .align_x(Center);

        let base_view = container(content_column).center_x(Fill).height(Fill);

        if matches!(self.screen, CurrentScreen::Home(_)) {
            let beta_image = image(BETA_IMAGE.clone()).width(100).height(100);

            let beta_overlay = container(beta_image)
                .width(Fill)
                .height(Fill)
                .align_x(Right)
                .align_y(Bottom);

            stack![base_view, beta_overlay].into()
        } else {
            base_view.into()
        }
    }

    fn spawn_agent(
        &mut self,
        screen: Screen<AgentRunning>,
        agent_name: Option<String>,
        management_address: String,
        slots: i32,
    ) -> Task<Message> {
        let (agent_shutdown_tx, agent_shutdown_rx) = oneshot::channel::<()>();
        let (status_watch_tx, status_watch_rx) =
            watch::channel::<Option<Arc<SlotAggregatedStatus>>>(None);

        self.agent_shutdown_tx = Some(agent_shutdown_tx);
        self.screen = CurrentScreen::AgentRunning(screen);

        Task::batch([
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        actix_web::rt::System::new().block_on(async {
                            let bootstrapped = bootstrap_agent(BootstrapAgentParams {
                                agent_name,
                                management_address,
                                slots,
                            });

                            if status_watch_tx
                                .send(Some(bootstrapped.slot_aggregated_status.clone()))
                                .is_err()
                            {
                                return Err(anyhow::anyhow!(
                                    "Monitor stream was dropped before receiving status"
                                ));
                            }

                            bootstrapped
                                .service_manager
                                .run_forever(agent_shutdown_rx)
                                .await
                        })
                    })
                    .await
                    .map_err(|error| anyhow::anyhow!("Agent task panicked: {error}"))?
                },
                |result: Result<(), anyhow::Error>| match result {
                    Ok(()) => Message::AgentStopped,
                    Err(error) => Message::AgentFailed(error.to_string()),
                },
            ),
            Task::stream(iced::stream::channel(1, async move |mut output| {
                let mut watch_rx = status_watch_rx;

                let slot_aggregated_status = loop {
                    if watch_rx.changed().await.is_err() {
                        return;
                    }
                    let borrowed = watch_rx.borrow_and_update().clone();
                    if let Some(status) = borrowed {
                        break status;
                    }
                };

                loop {
                    match slot_aggregated_status.make_snapshot() {
                        Ok(snapshot) => {
                            if output
                                .send(Message::AgentRunning(
                                    agent_running_handler::Message::AgentStatusUpdated(snapshot),
                                ))
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                        Err(error) => {
                            log::error!("Failed to make agent status snapshot: {error}");

                            return;
                        }
                    }

                    tokio::select! {
                        () = slot_aggregated_status.update_notifier.notified() => {}
                        changed_result = watch_rx.changed() => {
                            if changed_result.is_err() {
                                return;
                            }
                        }
                    }
                }
            })),
        ])
    }

    fn spawn_cluster(
        &mut self,
        management_addr: SocketAddr,
        inference_addr: SocketAddr,
        desired_state: BalancerDesiredState,
    ) -> Task<Message> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (pool_watch_tx, pool_watch_rx) =
            watch::channel::<Option<Arc<AgentControllerPool>>>(None);

        self.shutdown_tx = Some(shutdown_tx);

        Task::batch([
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        actix_web::rt::System::new().block_on(async {
                            let bootstrapped = bootstrap_balancer(BootstrapBalancerParams {
                                buffered_request_timeout: Duration::from_secs(10),
                                inference_service_configuration: InferenceServiceConfiguration {
                                    addr: inference_addr,
                                    cors_allowed_hosts: vec![],
                                    inference_item_timeout: Duration::from_secs(30),
                                },
                                management_service_configuration: ManagementServiceConfiguration {
                                    addr: management_addr,
                                    cors_allowed_hosts: vec![],
                                },
                                max_buffered_requests: 30,
                                openai_service_configuration: None,
                                state_database_type: StateDatabaseType::Memory,
                                statsd_prefix: "paddler_".to_owned(),
                                #[cfg(feature = "web_admin_panel")]
                                web_admin_panel_service_configuration: None,
                            })
                            .await?;

                            if pool_watch_tx
                                .send(Some(bootstrapped.agent_controller_pool.clone()))
                                .is_err()
                            {
                                return Err(anyhow::anyhow!(
                                    "Monitor stream was dropped before receiving pool"
                                ));
                            }

                            let state_database = bootstrapped.state_database.clone();

                            let service_handle = actix_web::rt::spawn(
                                bootstrapped.service_manager.run_forever(shutdown_rx),
                            );

                            state_database
                                .store_balancer_desired_state(&desired_state)
                                .await?;

                            service_handle.await.map_err(|error| {
                                anyhow::anyhow!("Service manager task failed: {error}")
                            })?
                        })
                    })
                    .await
                    .map_err(|error| anyhow::anyhow!("Balancer task panicked: {error}"))?
                },
                |result: Result<(), anyhow::Error>| match result {
                    Ok(()) => Message::ClusterStopped,
                    Err(error) => Message::ClusterFailed(error.to_string()),
                },
            ),
            Task::stream(iced::stream::channel(1, async move |mut output| {
                let mut watch_rx = pool_watch_rx;

                let agent_controller_pool =
                    match wait_for_bootstrapped_agent_controller_pool(&mut watch_rx).await {
                        Ok(pool) => pool,
                        Err(error) => {
                            log::error!(
                                "Failed waiting for bootstrapped agent controller pool: {error}"
                            );

                            return;
                        }
                    };

                if output.send(Message::ClusterStarted).await.is_err() {
                    return;
                }

                loop {
                    match collect_sorted_agent_snapshots(&agent_controller_pool) {
                        Ok(snapshots) => {
                            if output
                                .send(Message::RunningCluster(
                                    running_cluster_handler::Message::AgentSnapshotsUpdated(
                                        snapshots,
                                    ),
                                ))
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                        Err(error) => {
                            log::error!("Failed to collect agent snapshots: {error}");

                            return;
                        }
                    }

                    tokio::select! {
                        () = agent_controller_pool.update_notifier.notified() => {}
                        changed_result = watch_rx.changed() => {
                            if changed_result.is_err() {
                                return;
                            }
                        }
                    }
                }
            })),
        ])
    }
}
