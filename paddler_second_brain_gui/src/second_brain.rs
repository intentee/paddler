use iced::Center;
use iced::Element;
use iced::Task;
use iced::widget::button;
use iced::widget::column;
use iced::widget::text;
use tokio::sync::oneshot;

use crate::balancer_status::BalancerStatus;
use crate::message::Message;
use crate::start_balancer::start_balancer;

pub struct SecondBrain {
    balancer_status: BalancerStatus,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Drop for SecondBrain {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            if let Err(unsent_signal) = shutdown_tx.send(()) {
                log::error!("Failed to send balancer shutdown signal: {unsent_signal:?}");
            }
        }
    }
}

impl SecondBrain {
    pub fn new() -> (Self, Task<Message>) {
        let second_brain = Self {
            balancer_status: BalancerStatus::Stopped,
            shutdown_tx: None,
        };

        (second_brain, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::StartBalancer => {
                let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
                self.shutdown_tx = Some(shutdown_tx);
                self.balancer_status = BalancerStatus::Running;

                Task::perform(
                    start_balancer(shutdown_rx),
                    |result: Result<(), anyhow::Error>| match result {
                        Ok(()) => Message::BalancerStopped,
                        Err(error) => Message::BalancerFailed(error.to_string()),
                    },
                )
            }
            Message::BalancerStopped => {
                self.balancer_status = BalancerStatus::Stopped;

                Task::none()
            }
            Message::BalancerFailed(error) => {
                self.balancer_status = BalancerStatus::Failed(error);

                Task::none()
            }
        }
    }

    pub fn view<'view>(&'view self) -> Element<'view, Message> {
        let status_text = match &self.balancer_status {
            BalancerStatus::Stopped => "Balancer is stopped",
            BalancerStatus::Running => "Balancer is running",
            BalancerStatus::Failed(error) => error.as_str(),
        };

        let start_button = match &self.balancer_status {
            BalancerStatus::Stopped => button("Start Balancer").on_press(Message::StartBalancer),
            BalancerStatus::Running => button("Start Balancer"),
            BalancerStatus::Failed(_) => button("Start Balancer").on_press(Message::StartBalancer),
        };

        column![
            start_button,
            text(status_text),
        ]
        .padding(20)
        .spacing(10)
        .align_x(Center)
        .into()
    }
}
