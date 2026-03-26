use std::mem;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use iced::Center;
use iced::Element;
use iced::Fill;
use iced::Subscription;
use iced::Task;
use iced::futures::SinkExt;
use iced::widget::column;
use iced::widget::container;
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
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::message::Message;
use crate::model_preset::ModelPreset;
use crate::screen_current::CurrentScreen;
use crate::ui::variables::SPACING_2X;
use crate::ui::variables::SPACING_BASE;
use crate::ui::view_agent_running::view_agent_running;
use crate::ui::view_home::view_home;
use crate::ui::view_join_cluster_config::view_join_cluster_config;
use crate::ui::view_running_cluster::view_running_cluster;
use crate::ui::view_start_cluster_config::view_start_cluster_config;

fn is_port_in_use(address: &SocketAddr) -> bool {
    TcpListener::bind(address).is_err()
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

pub struct SecondBrain {
    agent_shutdown_tx: Option<oneshot::Sender<()>>,
    screen: CurrentScreen,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Drop for SecondBrain {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take()
            && let Err(unsent_signal) = shutdown_tx.send(())
        {
            log::error!("Failed to send cluster shutdown signal: {unsent_signal:?}");
        }

        if let Some(agent_shutdown_tx) = self.agent_shutdown_tx.take()
            && let Err(unsent_signal) = agent_shutdown_tx.send(())
        {
            log::error!("Failed to send agent shutdown signal: {unsent_signal:?}");
        }
    }
}

impl SecondBrain {
    pub fn new() -> (Self, Task<Message>) {
        let second_brain = Self {
            agent_shutdown_tx: None,
            screen: CurrentScreen::default(),
            shutdown_tx: None,
        };

        (second_brain, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let screen = mem::take(&mut self.screen);

        match (screen, message) {
            (CurrentScreen::Home(home), Message::JoinCluster) => {
                self.screen = CurrentScreen::JoinClusterConfig(home.join_cluster());

                Task::none()
            }
            (CurrentScreen::Home(home), Message::StartCluster) => {
                self.screen = CurrentScreen::StartClusterConfig(home.start_cluster());

                Task::none()
            }
            (CurrentScreen::JoinClusterConfig(config), Message::Cancel) => {
                self.screen = CurrentScreen::Home(config.cancel());

                Task::none()
            }
            (CurrentScreen::JoinClusterConfig(mut config), Message::SetAgentName(name)) => {
                config.state_data.agent_name = name;
                self.screen = CurrentScreen::JoinClusterConfig(config);

                Task::none()
            }
            (CurrentScreen::JoinClusterConfig(mut config), Message::SetClusterAddress(address)) => {
                config.state_data.cluster_address = address;
                config.state_data.cluster_address_error = None;
                self.screen = CurrentScreen::JoinClusterConfig(config);

                Task::none()
            }
            (CurrentScreen::JoinClusterConfig(mut config), Message::SetSlotsCount(slots)) => {
                if slots.is_empty() || slots.chars().all(|character| character.is_ascii_digit()) {
                    config.state_data.slots_count = slots;
                    config.state_data.slots_error = None;
                }
                self.screen = CurrentScreen::JoinClusterConfig(config);

                Task::none()
            }
            (CurrentScreen::JoinClusterConfig(mut config), Message::Connect) => {
                config.state_data.cluster_address_error = None;
                config.state_data.slots_error = None;

                if config.state_data.cluster_address.is_empty() {
                    config.state_data.cluster_address_error =
                        Some("Cluster address is required.".to_owned());
                } else if config
                    .state_data
                    .cluster_address
                    .parse::<SocketAddr>()
                    .is_err()
                {
                    config.state_data.cluster_address_error =
                        Some("Invalid address, expected format: IP:port".to_owned());
                }

                let slots = if config.state_data.slots_count.is_empty() {
                    config.state_data.slots_error = Some("Number of slots is required.".to_owned());
                    None
                } else {
                    match config.state_data.slots_count.parse::<i32>() {
                        Ok(slots) if slots > 0 => Some(slots),
                        Ok(non_positive_slots) => {
                            log::debug!(
                                "User entered non-positive slot count: {non_positive_slots}"
                            );
                            config.state_data.slots_error = Some(
                                "Invalid number of slots (the number should be greater than zero)."
                                    .to_owned(),
                            );
                            None
                        }
                        Err(error) => {
                            let message = match error.kind() {
                                std::num::IntErrorKind::PosOverflow => {
                                    "Number of slots is too large."
                                }
                                unexpected_kind => {
                                    log::error!(
                                        "Unexpected slots parse error: {unexpected_kind:?}"
                                    );
                                    "Invalid number of slots."
                                }
                            };
                            config.state_data.slots_error = Some(message.to_owned());
                            None
                        }
                    }
                };

                if config.state_data.cluster_address_error.is_some()
                    || config.state_data.slots_error.is_some()
                {
                    self.screen = CurrentScreen::JoinClusterConfig(config);

                    return Task::none();
                }

                let Some(slots) = slots else {
                    self.screen = CurrentScreen::JoinClusterConfig(config);

                    return Task::none();
                };

                let agent_name = if config.state_data.agent_name.is_empty() {
                    None
                } else {
                    Some(config.state_data.agent_name.clone())
                };
                let management_address = config.state_data.cluster_address.clone();

                let (agent_shutdown_tx, agent_shutdown_rx) = oneshot::channel::<()>();
                let (status_watch_tx, status_watch_rx) =
                    watch::channel::<Option<Arc<SlotAggregatedStatus>>>(None);

                self.agent_shutdown_tx = Some(agent_shutdown_tx);
                self.screen = CurrentScreen::AgentRunning(config.connect());

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
                                        .send(Message::AgentStatusUpdated(snapshot))
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
            (CurrentScreen::StartClusterConfig(config), Message::Cancel) => {
                if let Some(shutdown_tx) = self.shutdown_tx.take()
                    && let Err(unsent_signal) = shutdown_tx.send(())
                {
                    log::error!("Failed to send cluster shutdown signal: {unsent_signal:?}");
                }
                self.screen = CurrentScreen::Home(config.cancel());

                Task::none()
            }
            (CurrentScreen::StartClusterConfig(mut config), Message::SelectModel(preset)) => {
                config.state_data.selected_model = Some(preset);
                config.state_data.model_error = None;
                self.screen = CurrentScreen::StartClusterConfig(config);

                Task::none()
            }
            (
                CurrentScreen::StartClusterConfig(mut config),
                Message::SetBalancerAddress(address),
            ) => {
                config.state_data.balancer_address = address;
                config.state_data.balancer_address_error = None;
                self.screen = CurrentScreen::StartClusterConfig(config);

                Task::none()
            }
            (
                CurrentScreen::StartClusterConfig(mut config),
                Message::SetInferenceAddress(address),
            ) => {
                config.state_data.inference_address = address;
                config.state_data.inference_address_error = None;
                self.screen = CurrentScreen::StartClusterConfig(config);

                Task::none()
            }
            (CurrentScreen::StartClusterConfig(mut config), Message::Confirm) => {
                config.state_data.balancer_address_error = None;
                config.state_data.inference_address_error = None;
                config.state_data.model_error = None;

                if config.state_data.selected_model.is_none() {
                    config.state_data.model_error = Some("Please select a model.".to_owned());
                }

                let management_addr = if config.state_data.balancer_address.is_empty() {
                    config.state_data.balancer_address_error =
                        Some("Balancer address is required.".to_owned());
                    None
                } else if let Ok(addr) = config.state_data.balancer_address.parse::<SocketAddr>() {
                    Some(addr)
                } else {
                    config.state_data.balancer_address_error =
                        Some("Invalid address, expected format: IP:port".to_owned());
                    None
                };

                let inference_addr = if config.state_data.inference_address.is_empty() {
                    config.state_data.inference_address_error =
                        Some("Inference address is required.".to_owned());
                    None
                } else if let Ok(addr) = config.state_data.inference_address.parse::<SocketAddr>() {
                    Some(addr)
                } else {
                    config.state_data.inference_address_error =
                        Some("Invalid address, expected format: IP:port".to_owned());
                    None
                };

                let management_addr = match management_addr {
                    Some(addr) if is_port_in_use(&addr) => {
                        config.state_data.balancer_address_error =
                            Some(format!("Port {} is already in use", addr.port()));
                        None
                    }
                    other => other,
                };

                let inference_addr = match inference_addr {
                    Some(addr) if is_port_in_use(&addr) => {
                        config.state_data.inference_address_error =
                            Some(format!("Port {} is already in use", addr.port()));
                        None
                    }
                    other => other,
                };

                if config.state_data.model_error.is_some()
                    || config.state_data.balancer_address_error.is_some()
                    || config.state_data.inference_address_error.is_some()
                {
                    self.screen = CurrentScreen::StartClusterConfig(config);

                    return Task::none();
                }

                let (Some(management_addr), Some(inference_addr)) =
                    (management_addr, inference_addr)
                else {
                    self.screen = CurrentScreen::StartClusterConfig(config);

                    return Task::none();
                };

                let desired_state = config
                    .state_data
                    .selected_model
                    .as_ref()
                    .map(ModelPreset::to_balancer_desired_state)
                    .unwrap_or_default();

                let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
                let (pool_watch_tx, pool_watch_rx) =
                    watch::channel::<Option<Arc<AgentControllerPool>>>(None);

                self.shutdown_tx = Some(shutdown_tx);
                config.state_data.starting = true;
                self.screen = CurrentScreen::StartClusterConfig(config);

                Task::batch([
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                actix_web::rt::System::new().block_on(async {
                                    let bootstrapped =
                                        bootstrap_balancer(BootstrapBalancerParams {
                                            buffered_request_timeout: Duration::from_secs(10),
                                            inference_service_configuration:
                                                InferenceServiceConfiguration {
                                                    addr: inference_addr,
                                                    cors_allowed_hosts: vec![],
                                                    inference_item_timeout: Duration::from_secs(30),
                                                },
                                            management_service_configuration:
                                                ManagementServiceConfiguration {
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

                                    bootstrapped
                                        .state_database
                                        .store_balancer_desired_state(&desired_state)
                                        .await?;

                                    if pool_watch_tx
                                        .send(Some(bootstrapped.agent_controller_pool.clone()))
                                        .is_err()
                                    {
                                        return Err(anyhow::anyhow!(
                                            "Monitor stream was dropped before receiving pool"
                                        ));
                                    }

                                    bootstrapped.service_manager.run_forever(shutdown_rx).await
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
                    Task::done(Message::ClusterStarted),
                    Task::stream(iced::stream::channel(1, async move |mut output| {
                        let mut watch_rx = pool_watch_rx;

                        let agent_controller_pool = loop {
                            if watch_rx.changed().await.is_err() {
                                return;
                            }
                            let borrowed = watch_rx.borrow_and_update().clone();
                            if let Some(pool) = borrowed {
                                break pool;
                            }
                        };

                        loop {
                            match collect_sorted_agent_snapshots(&agent_controller_pool) {
                                Ok(snapshots) => {
                                    if output
                                        .send(Message::AgentSnapshotsUpdated(snapshots))
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
            (
                CurrentScreen::RunningCluster(mut running),
                Message::AgentSnapshotsUpdated(snapshots),
            ) => {
                running.state_data.agent_snapshots = snapshots;
                self.screen = CurrentScreen::RunningCluster(running);

                Task::none()
            }
            (CurrentScreen::RunningCluster(mut running), Message::Stop) => {
                if let Some(shutdown_tx) = self.shutdown_tx.take()
                    && let Err(unsent_signal) = shutdown_tx.send(())
                {
                    log::error!("Failed to send cluster shutdown signal: {unsent_signal:?}");
                }
                running.state_data.stopping = true;
                self.screen = CurrentScreen::RunningCluster(running);

                Task::none()
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
            (CurrentScreen::AgentRunning(mut running), Message::AgentStatusUpdated(status)) => {
                running.state_data.apply_status(status);
                self.screen = CurrentScreen::AgentRunning(running);

                Task::none()
            }
            (CurrentScreen::AgentRunning(running), Message::Disconnect) => {
                if let Some(agent_shutdown_tx) = self.agent_shutdown_tx.take()
                    && let Err(unsent_signal) = agent_shutdown_tx.send(())
                {
                    log::error!("Failed to send agent shutdown signal: {unsent_signal:?}");
                }
                self.screen = CurrentScreen::Home(running.disconnect());

                Task::none()
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
            (screen, Message::CopyToClipboard(content)) => {
                self.screen = screen;

                iced::clipboard::write::<Message>(content).discard()
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
        Subscription::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let screen_content = match &self.screen {
            CurrentScreen::AgentRunning(screen) => view_agent_running(&screen.state_data),
            CurrentScreen::Home(screen) => view_home(&screen.state_data),
            CurrentScreen::JoinClusterConfig(screen) => {
                view_join_cluster_config(&screen.state_data)
            }
            CurrentScreen::StartClusterConfig(screen) => {
                view_start_cluster_config(&screen.state_data)
            }
            CurrentScreen::RunningCluster(screen) => view_running_cluster(&screen.state_data),
        };

        let content_column = column![screen_content]
            .max_width(700)
            .padding([SPACING_2X * 2.0, SPACING_BASE])
            .spacing(SPACING_BASE)
            .align_x(Center);

        container(content_column).center_x(Fill).into()
    }
}
