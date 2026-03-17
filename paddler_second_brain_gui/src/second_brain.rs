use iced::Center;
use iced::Element;
use iced::Task;
use iced::widget::button;
use iced::widget::column;
use iced::widget::text;
use tokio::sync::oneshot;

use crate::cluster_status::ClusterStatus;
use crate::message::Message;
use crate::start_balancer::start_balancer;

pub struct SecondBrain {
    cluster_status: ClusterStatus,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Drop for SecondBrain {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            if let Err(unsent_signal) = shutdown_tx.send(()) {
                log::error!("Failed to send cluster shutdown signal: {unsent_signal:?}");
            }
        }
    }
}

impl SecondBrain {
    pub fn new() -> (Self, Task<Message>) {
        let second_brain = Self {
            cluster_status: ClusterStatus::Stopped,
            shutdown_tx: None,
        };

        (second_brain, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::StartCluster => {
                let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
                self.shutdown_tx = Some(shutdown_tx);
                self.cluster_status = ClusterStatus::Running;

                Task::perform(
                    start_balancer(shutdown_rx),
                    |result: Result<(), anyhow::Error>| match result {
                        Ok(()) => Message::ClusterStopped,
                        Err(error) => Message::ClusterFailed(error.to_string()),
                    },
                )
            }
            Message::StopCluster => {
                if let Some(shutdown_tx) = self.shutdown_tx.take() {
                    if let Err(unsent_signal) = shutdown_tx.send(()) {
                        log::error!("Failed to send cluster shutdown signal: {unsent_signal:?}");
                    }
                }

                self.cluster_status = ClusterStatus::Stopped;

                Task::none()
            }
            Message::ClusterStopped => {
                self.cluster_status = ClusterStatus::Stopped;

                Task::none()
            }
            Message::ClusterFailed(error) => {
                self.cluster_status = ClusterStatus::Failed(error);

                Task::none()
            }
        }
    }

    pub fn view<'view>(&'view self) -> Element<'view, Message> {
        let status_text = match &self.cluster_status {
            ClusterStatus::Stopped => "Cluster is stopped",
            ClusterStatus::Running => "Cluster is running",
            ClusterStatus::Failed(error) => error.as_str(),
        };

        let action_button = match &self.cluster_status {
            ClusterStatus::Stopped => button("Start a cluster").on_press(Message::StartCluster),
            ClusterStatus::Running => button("Stop cluster").on_press(Message::StopCluster),
            ClusterStatus::Failed(_) => button("Start a cluster").on_press(Message::StartCluster),
        };

        column![action_button, text(status_text),]
            .padding(20)
            .spacing(10)
            .align_x(Center)
            .into()
    }
}
