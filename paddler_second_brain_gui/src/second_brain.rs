use std::mem;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::time::Duration;

use iced::Center;
use iced::Element;
use iced::Fill;
use iced::Subscription;
use iced::Task;
use iced::time;
use iced::widget::column;
use iced::widget::container;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::message::Message;
use crate::screen_current::CurrentScreen;
use crate::start_agent::start_agent;
use crate::start_balancer::start_balancer;
use crate::ui::variables::SPACING_2X;
use crate::ui::variables::SPACING_BASE;
use crate::ui::view_agent_running::view_agent_running;
use crate::ui::view_home::view_home;
use crate::ui::view_join_cluster_config::view_join_cluster_config;
use crate::ui::view_running_cluster::view_running_cluster;
use crate::ui::view_start_cluster_config::view_start_cluster_config;

fn is_port_in_use(address: &SocketAddr) -> bool {
    TcpStream::connect_timeout(address, Duration::from_millis(100)).is_ok()
}

fn drain_latest<TValue>(receiver: &mut mpsc::UnboundedReceiver<TValue>) -> Option<TValue> {
    let mut latest = None;

    while let Ok(value) = receiver.try_recv() {
        latest = Some(value);
    }

    latest
}

pub struct SecondBrain {
    agent_snapshots_rx: Option<mpsc::UnboundedReceiver<Vec<AgentControllerSnapshot>>>,
    agent_shutdown_tx: Option<oneshot::Sender<()>>,
    agent_status_rx: Option<mpsc::UnboundedReceiver<SlotAggregatedStatusSnapshot>>,
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
            agent_snapshots_rx: None,
            agent_shutdown_tx: None,
            agent_status_rx: None,
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
                config.state_data.error = None;
                self.screen = CurrentScreen::JoinClusterConfig(config);

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

                let agent_name = if config.state_data.agent_name.is_empty() {
                    None
                } else {
                    Some(config.state_data.agent_name.clone())
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
                        agent_name,
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
                if let Some(shutdown_tx) = self.shutdown_tx.take()
                    && let Err(unsent_signal) = shutdown_tx.send(())
                {
                    log::error!("Failed to send cluster shutdown signal: {unsent_signal:?}");
                }
                self.agent_snapshots_rx = None;
                self.screen = CurrentScreen::Home(config.cancel());

                Task::none()
            }
            (CurrentScreen::StartClusterConfig(mut config), Message::SelectModel(preset)) => {
                config.state_data.selected_model = Some(preset);
                config.state_data.balancer_address_error = None;
                config.state_data.inference_address_error = None;
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

                let management_addr = match config.state_data.balancer_address.parse::<SocketAddr>()
                {
                    Ok(addr) => Some(addr),
                    Err(parse_error) => {
                        config.state_data.balancer_address_error =
                            Some(format!("Invalid address: {parse_error}"));
                        None
                    }
                };

                let inference_addr = match config.state_data.inference_address.parse::<SocketAddr>()
                {
                    Ok(addr) => Some(addr),
                    Err(parse_error) => {
                        config.state_data.inference_address_error =
                            Some(format!("Invalid address: {parse_error}"));
                        None
                    }
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

                let (management_addr, inference_addr) = match (management_addr, inference_addr) {
                    (Some(management), Some(inference)) => (management, inference),
                    _ => {
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
                let (agent_snapshots_tx, agent_snapshots_rx) =
                    mpsc::unbounded_channel::<Vec<AgentControllerSnapshot>>();

                self.agent_snapshots_rx = Some(agent_snapshots_rx);
                self.shutdown_tx = Some(shutdown_tx);
                config.state_data.starting = true;
                self.screen = CurrentScreen::StartClusterConfig(config);

                Task::batch([
                    Task::perform(
                        start_balancer(
                            management_addr,
                            inference_addr,
                            desired_state,
                            agent_snapshots_tx,
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
            (CurrentScreen::RunningCluster(mut running), Message::RefreshAgentCount) => {
                if let Some(snapshots) = self.agent_snapshots_rx.as_mut().and_then(drain_latest) {
                    running.state_data.agent_snapshots = snapshots;
                }
                self.screen = CurrentScreen::RunningCluster(running);

                Task::none()
            }
            (CurrentScreen::RunningCluster(mut running), Message::Stop) => {
                if let Some(shutdown_tx) = self.shutdown_tx.take()
                    && let Err(unsent_signal) = shutdown_tx.send(())
                {
                    log::error!("Failed to send cluster shutdown signal: {unsent_signal:?}");
                }
                self.agent_snapshots_rx = None;
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
                self.agent_snapshots_rx = None;
                self.shutdown_tx = None;
                self.screen = CurrentScreen::Home(running.cluster_failed(error));

                Task::none()
            }
            (CurrentScreen::AgentRunning(mut running), Message::RefreshAgentStatus) => {
                if let Some(status) = self.agent_status_rx.as_mut().and_then(drain_latest) {
                    running.state_data.apply_status(status);
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

    pub fn subscription(&self) -> Subscription<Message> {
        match &self.screen {
            CurrentScreen::AgentRunning(_) => {
                time::every(Duration::from_secs(1)).map(|_| Message::RefreshAgentStatus)
            }
            CurrentScreen::RunningCluster(_) => {
                time::every(Duration::from_secs(1)).map(|_| Message::RefreshAgentCount)
            }
            _ => Subscription::none(),
        }
    }

    pub fn view<'view>(&'view self) -> Element<'view, Message> {
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
