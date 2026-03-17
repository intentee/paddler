use std::mem;

use iced::Center;
use iced::Element;
use iced::Task;
use iced::widget::column;
use iced::widget::text;
use tokio::sync::oneshot;

use crate::message::Message;
use crate::screen_current::CurrentScreen;
use crate::start_balancer::start_balancer;
use crate::view_home::view_home;
use crate::view_running_cluster::view_running_cluster;
use crate::view_start_cluster_config::view_start_cluster_config;
use crate::view_starting_cluster::view_starting_cluster;
use crate::view_stopping_cluster::view_stopping_cluster;

pub struct SecondBrain {
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
    }
}

impl SecondBrain {
    pub fn new() -> (Self, Task<Message>) {
        let second_brain = Self {
            screen: CurrentScreen::default(),
            shutdown_tx: None,
        };

        (second_brain, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let screen = mem::take(&mut self.screen);

        match (screen, message) {
            (CurrentScreen::Home(home), Message::StartCluster) => {
                self.screen = CurrentScreen::StartClusterConfig(home.start_cluster());

                Task::none()
            }
            (CurrentScreen::StartClusterConfig(config), Message::Cancel) => {
                self.screen = CurrentScreen::Home(config.cancel());

                Task::none()
            }
            (CurrentScreen::StartClusterConfig(mut config), Message::SelectModel(model)) => {
                config.state_data.selected_model = Some(model);
                self.screen = CurrentScreen::StartClusterConfig(config);

                Task::none()
            }
            (
                CurrentScreen::StartClusterConfig(mut config),
                Message::ToggleRunAgentLocally(enabled),
            ) => {
                config.state_data.run_agent_locally = enabled;
                self.screen = CurrentScreen::StartClusterConfig(config);

                Task::none()
            }
            (CurrentScreen::StartClusterConfig(config), Message::Confirm) => {
                let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
                self.shutdown_tx = Some(shutdown_tx);
                self.screen = CurrentScreen::StartingCluster(config.confirm());

                Task::batch([
                    Task::perform(
                        start_balancer(shutdown_rx),
                        |result: Result<(), anyhow::Error>| match result {
                            Ok(()) => Message::ClusterStopped,
                            Err(error) => Message::ClusterFailed(error.to_string()),
                        },
                    ),
                    Task::done(Message::ClusterStarted),
                ])
            }
            (CurrentScreen::StartingCluster(starting), Message::ClusterStarted) => {
                let cluster_address = "192.168.1.1".to_string(); // TODO: detect local IP
                self.screen =
                    CurrentScreen::RunningCluster(starting.cluster_started(cluster_address));

                Task::none()
            }
            (CurrentScreen::StartingCluster(starting), Message::ClusterFailed(error)) => {
                log::error!("Cluster failed to start: {error}");
                self.shutdown_tx = None;
                self.screen = CurrentScreen::Home(starting.cluster_failed());

                Task::none()
            }
            (CurrentScreen::RunningCluster(running), Message::Stop) => {
                if let Some(shutdown_tx) = self.shutdown_tx.take()
                    && let Err(unsent_signal) = shutdown_tx.send(())
                {
                    log::error!("Failed to send cluster shutdown signal: {unsent_signal:?}");
                }
                self.screen = CurrentScreen::StoppingCluster(running.stop());

                Task::none()
            }
            (CurrentScreen::RunningCluster(running), Message::ClusterFailed(error)) => {
                log::error!("Cluster failed unexpectedly: {error}");
                self.shutdown_tx = None;
                self.screen = CurrentScreen::Home(running.cluster_failed());

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

    pub fn view(&self) -> Element<'_, Message> {
        let screen_content = match &self.screen {
            CurrentScreen::Home(_) => view_home(),
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
