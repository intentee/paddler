use std::mem;
use std::time::Duration;

use iced::Center;
use iced::Element;
use iced::Subscription;
use iced::Task;
use iced::time;
use iced::widget::column;
use iced::widget::text;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::detect_network_interfaces::detect_network_interfaces;
use crate::message::Message;
use crate::network_interface_address::NetworkInterfaceAddress;
use crate::screen_current::CurrentScreen;
use crate::start_agent::start_agent;
use crate::start_balancer::start_balancer;
use crate::view_agent_running::view_agent_running;
use crate::view_home::view_home;
use crate::view_join_cluster_config::view_join_cluster_config;
use crate::view_running_cluster::view_running_cluster;
use crate::view_start_cluster_config::view_start_cluster_config;
use crate::view_starting_cluster::view_starting_cluster;
use crate::view_stopping_cluster::view_stopping_cluster;

pub struct SecondBrain {
    agent_count_rx: Option<mpsc::UnboundedReceiver<usize>>,
    agent_shutdown_tx: Option<oneshot::Sender<()>>,
    agent_status_rx: Option<mpsc::UnboundedReceiver<SlotAggregatedStatusSnapshot>>,
    network_interfaces_rx: Option<mpsc::UnboundedReceiver<Vec<NetworkInterfaceAddress>>>,
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
            agent_count_rx: None,
            agent_shutdown_tx: None,
            agent_status_rx: None,
            network_interfaces_rx: None,
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
            (CurrentScreen::JoinClusterConfig(mut config), Message::SetClusterAddress(address)) => {
                config.state_data.cluster_address = address;
                config.state_data.error = None;
                self.screen = CurrentScreen::JoinClusterConfig(config);

                Task::none()
            }
            (CurrentScreen::JoinClusterConfig(mut config), Message::SetSlotsCount(slots)) => {
                config.state_data.slots_count = slots;
                config.state_data.error = None;
                self.screen = CurrentScreen::JoinClusterConfig(config);

                Task::none()
            }
            (CurrentScreen::JoinClusterConfig(config), Message::Connect) => {
                let slots = match config.state_data.slots_count.parse::<i32>() {
                    Ok(slots) if slots > 0 => slots,
                    _ => {
                        let mut config = config;
                        config.state_data.error =
                            Some("Enter a valid number of slots.".to_string());
                        self.screen = CurrentScreen::JoinClusterConfig(config);

                        return Task::none();
                    }
                };

                let management_address = config.state_data.cluster_address.clone();

                let (agent_shutdown_tx, agent_shutdown_rx) = oneshot::channel::<()>();
                let (agent_status_tx, agent_status_rx) =
                    mpsc::unbounded_channel::<SlotAggregatedStatusSnapshot>();

                self.agent_shutdown_tx = Some(agent_shutdown_tx);
                self.agent_status_rx = Some(agent_status_rx);
                self.screen = CurrentScreen::AgentRunning(config.connect());

                Task::perform(
                    start_agent(
                        management_address,
                        slots,
                        agent_status_tx,
                        agent_shutdown_rx,
                    ),
                    |result: Result<(), anyhow::Error>| match result {
                        Ok(()) => Message::AgentStopped,
                        Err(error) => Message::AgentFailed(error.to_string()),
                    },
                )
            }
            (CurrentScreen::StartClusterConfig(config), Message::Cancel) => {
                self.screen = CurrentScreen::Home(config.cancel());

                Task::none()
            }
            (CurrentScreen::StartClusterConfig(mut config), Message::SelectModel(preset)) => {
                config.state_data.selected_model = Some(preset);
                config.state_data.error = None;
                self.screen = CurrentScreen::StartClusterConfig(config);

                Task::none()
            }
            (
                CurrentScreen::StartClusterConfig(mut config),
                Message::ToggleRunAgentLocally(enabled),
            ) => {
                config.state_data.run_agent_locally = enabled;
                config.state_data.error = None;
                self.screen = CurrentScreen::StartClusterConfig(config);

                Task::none()
            }
            (CurrentScreen::StartClusterConfig(config), Message::Confirm) => {
                let network_interfaces = detect_network_interfaces();
                let management_port = 8060;

                let bind_ip = match network_interfaces.first() {
                    Some(interface) => interface.ip_address,
                    None => {
                        let mut config = config;
                        config.state_data.error = Some(
                            "No local network found. Connect to internet to start a cluster."
                                .to_string(),
                        );
                        self.screen = CurrentScreen::StartClusterConfig(config);

                        return Task::none();
                    }
                };

                let desired_state = config
                    .state_data
                    .selected_model
                    .as_ref()
                    .map(|preset| preset.to_balancer_desired_state())
                    .unwrap_or_default();

                let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
                let (agent_count_tx, agent_count_rx) = mpsc::unbounded_channel::<usize>();
                let (network_interfaces_tx, network_interfaces_rx) =
                    mpsc::unbounded_channel::<Vec<NetworkInterfaceAddress>>();

                self.agent_count_rx = Some(agent_count_rx);
                self.network_interfaces_rx = Some(network_interfaces_rx);
                self.shutdown_tx = Some(shutdown_tx);
                self.screen = CurrentScreen::StartingCluster(
                    config.confirm(network_interfaces, management_port),
                );

                Task::batch([
                    Task::perform(
                        start_balancer(
                            bind_ip,
                            desired_state,
                            agent_count_tx,
                            network_interfaces_tx,
                            shutdown_rx,
                        ),
                        |result: Result<(), anyhow::Error>| match result {
                            Ok(()) => Message::ClusterStopped,
                            Err(error) => Message::ClusterFailed(error.to_string()),
                        },
                    ),
                    Task::done(Message::ClusterStarted),
                ])
            }
            (CurrentScreen::StartingCluster(starting), Message::ClusterStarted) => {
                self.screen = CurrentScreen::RunningCluster(starting.cluster_started());

                Task::none()
            }
            (CurrentScreen::StartingCluster(starting), Message::ClusterFailed(error)) => {
                log::error!("Cluster failed to start: {error}");
                self.shutdown_tx = None;
                self.screen = CurrentScreen::Home(starting.cluster_failed());

                Task::none()
            }
            (CurrentScreen::RunningCluster(running), Message::Cancel) => {
                self.screen = CurrentScreen::Home(running.dismiss());

                Task::none()
            }
            (CurrentScreen::RunningCluster(mut running), Message::RefreshAgentCount) => {
                if let Some(agent_count_rx) = &mut self.agent_count_rx {
                    let mut latest_count = None;

                    while let Ok(count) = agent_count_rx.try_recv() {
                        latest_count = Some(count);
                    }

                    if let Some(count) = latest_count {
                        running.state_data.agent_count = count;
                    }
                }
                self.screen = CurrentScreen::RunningCluster(running);

                Task::none()
            }
            (CurrentScreen::RunningCluster(mut running), Message::RefreshNetworkInterfaces) => {
                if let Some(network_interfaces_rx) = &mut self.network_interfaces_rx {
                    let mut latest_interfaces = None;

                    while let Ok(interfaces) = network_interfaces_rx.try_recv() {
                        latest_interfaces = Some(interfaces);
                    }

                    if let Some(interfaces) = latest_interfaces {
                        running.state_data.network_interfaces = interfaces;
                    }
                }
                self.screen = CurrentScreen::RunningCluster(running);

                Task::none()
            }
            (CurrentScreen::RunningCluster(running), Message::Stop) => {
                if let Some(shutdown_tx) = self.shutdown_tx.take()
                    && let Err(unsent_signal) = shutdown_tx.send(())
                {
                    log::error!("Failed to send cluster shutdown signal: {unsent_signal:?}");
                }
                self.agent_count_rx = None;
                self.network_interfaces_rx = None;
                self.screen = CurrentScreen::StoppingCluster(running.stop());

                Task::none()
            }
            (CurrentScreen::RunningCluster(running), Message::ClusterFailed(error)) => {
                log::error!("Cluster failed unexpectedly: {error}");
                self.agent_count_rx = None;
                self.network_interfaces_rx = None;
                self.shutdown_tx = None;
                self.screen = CurrentScreen::Home(running.cluster_failed());

                Task::none()
            }
            (CurrentScreen::AgentRunning(mut running), Message::RefreshAgentStatus) => {
                if let Some(agent_status_rx) = &mut self.agent_status_rx {
                    let mut latest_status = None;

                    while let Ok(status) = agent_status_rx.try_recv() {
                        latest_status = Some(status);
                    }

                    if let Some(status) = latest_status {
                        running.state_data.status = Some(status);
                    }
                }
                self.screen = CurrentScreen::AgentRunning(running);

                Task::none()
            }
            (CurrentScreen::AgentRunning(running), Message::Disconnect) => {
                if let Some(agent_shutdown_tx) = self.agent_shutdown_tx.take()
                    && let Err(unsent_signal) = agent_shutdown_tx.send(())
                {
                    log::error!("Failed to send agent shutdown signal: {unsent_signal:?}");
                }
                self.agent_status_rx = None;
                self.screen = CurrentScreen::Home(running.disconnect());

                Task::none()
            }
            (CurrentScreen::AgentRunning(running), Message::AgentStopped) => {
                log::info!("Agent stopped");
                self.agent_shutdown_tx = None;
                self.agent_status_rx = None;
                self.screen = CurrentScreen::Home(running.disconnect());

                Task::none()
            }
            (CurrentScreen::AgentRunning(running), Message::AgentFailed(error)) => {
                log::error!("Agent failed: {error}");
                self.agent_shutdown_tx = None;
                self.agent_status_rx = None;
                self.screen = CurrentScreen::Home(running.agent_failed());

                Task::none()
            }
            (CurrentScreen::StoppingCluster(stopping), Message::ClusterStopped) => {
                self.screen = CurrentScreen::Home(stopping.cluster_stopped());

                Task::none()
            }
            (CurrentScreen::StoppingCluster(stopping), Message::ClusterFailed(error)) => {
                log::error!("Cluster failed during shutdown: {error}");
                self.screen = CurrentScreen::Home(stopping.cluster_failed());

                Task::none()
            }
            (screen, message) => {
                log::warn!("Unhandled message {message:?} for current screen");
                self.screen = screen;

                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match &self.screen {
            CurrentScreen::AgentRunning(_) => {
                time::every(Duration::from_secs(1)).map(|_| Message::RefreshAgentStatus)
            }
            CurrentScreen::RunningCluster(_) => Subscription::batch([
                time::every(Duration::from_secs(1)).map(|_| Message::RefreshAgentCount),
                time::every(Duration::from_secs(1)).map(|_| Message::RefreshNetworkInterfaces),
            ]),
            _ => Subscription::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let screen_content = match &self.screen {
            CurrentScreen::AgentRunning(screen) => view_agent_running(&screen.state_data),
            CurrentScreen::Home(_) => view_home(),
            CurrentScreen::JoinClusterConfig(screen) => {
                view_join_cluster_config(&screen.state_data)
            }
            CurrentScreen::StartClusterConfig(screen) => {
                view_start_cluster_config(&screen.state_data)
            }
            CurrentScreen::StartingCluster(_) => view_starting_cluster(),
            CurrentScreen::RunningCluster(screen) => view_running_cluster(&screen.state_data),
            CurrentScreen::StoppingCluster(_) => view_stopping_cluster(),
        };

        column![text("Paddler second brain").size(24), screen_content]
            .padding(20)
            .spacing(20)
            .align_x(Center)
            .into()
    }
}
