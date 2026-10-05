use iced::Center;
use iced::Element;
use iced::Fill;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::text;

use paddler_messaging::balancer_connection::BalancerConnection;

use super::font::BOLD;
use super::font::REGULAR;
use super::variables::FONT_SIZE_L2;
use super::variables::SPACING_2X;
use super::variables::SPACING_BASE;
use super::view_agent_card::view_agent_card;
use super::view_stop_button::view_stop_button;
use crate::agent_running_data::AgentRunningData;
use crate::agent_running_message::AgentRunningMessage;

impl AgentRunningData {
    #[must_use]
    pub fn view(&self) -> Element<'_, AgentRunningMessage> {
        let disconnect_button =
            view_stop_button("Disconnect", Some(AgentRunningMessage::Disconnect));

        let connection_status = match self.balancer_connection {
            BalancerConnection::Connected => text(format!(
                "Connected to the cluster at {}",
                self.balancer_address
            ))
            .font(REGULAR),
            BalancerConnection::Connecting => text("Connecting to the cluster...").font(REGULAR),
        };

        let status_row = container(
            row![container(connection_status).width(Fill), disconnect_button,].align_y(Center),
        )
        .padding([0.0, SPACING_BASE]);

        column![
            container(text("Your agent").size(FONT_SIZE_L2).font(BOLD))
                .padding([0.0, SPACING_BASE]),
            container(text("Agent details").font(BOLD)).padding([0.0, SPACING_BASE]),
            view_agent_card(self.name.as_deref(), self.slots_processing, &self.status),
            status_row,
        ]
        .spacing(SPACING_2X)
        .into()
    }
}
