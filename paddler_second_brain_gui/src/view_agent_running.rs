use iced::Element;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::progress_bar;
use iced::widget::row;
use iced::widget::svg;
use iced::widget::text;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

use crate::agent_running_data::AgentRunningData;
use crate::font::BOLD;
use crate::font::REGULAR;
use crate::message::Message;
use crate::style_agent_container::style_agent_container;
use crate::style_button_danger::style_button_danger;
use crate::style_download_progress_bar::style_download_progress_bar;
use crate::variables::FONT_SIZE_L2;
use crate::variables::SPACING_2X;
use crate::variables::SPACING_BASE;
use crate::variables::SPACING_HALF;

fn display_last_path_part(path: &str) -> String {
    path.split('/').last().unwrap_or(path).to_string()
}

pub fn view_agent_running<'content>(
    data: &'content AgentRunningData,
) -> Element<'content, Message> {
    let stop_icon = svg(svg::Handle::from_memory(
        include_bytes!("../../resources/icons/stop.svg").as_slice(),
    ))
    .width(16)
    .height(16);

    let disconnect_button = button(
        row![stop_icon, text("Disconnect").font(BOLD)]
            .spacing(SPACING_HALF)
            .align_y(iced::Center),
    )
    .padding([SPACING_HALF, SPACING_BASE])
    .style(style_button_danger)
    .on_press(Message::Disconnect);

    let mut name_row = row![container(text(data.agent_name.clone()).font(BOLD)).width(iced::Fill),];

    let mut status_row_left = column![].spacing(SPACING_HALF);

    let mut slots_label: Option<String> = None;
    match &data.status {
        None => {}
        Some(status) => {
            let is_downloading =
                status.download_total > 0 && status.download_current < status.download_total;

            if is_downloading {
                name_row = name_row.push(
                    progress_bar(
                        0.0..=status.download_total as f32,
                        status.download_current as f32,
                    )
                    .girth(12)
                    .style(style_download_progress_bar),
                );
            } else {
                let model_label = match &status.model_path {
                    Some(path) => display_last_path_part(path),
                    None => "No model loaded".to_string(),
                };

                name_row = name_row.push(text(model_label).font(REGULAR));
            }

            let status_label = if is_downloading {
                let percentage =
                    (status.download_current as f32 / status.download_total as f32) * 100.0;

                format!("Downloading ({percentage:.0}%)")
            } else if status.model_path.is_none() {
                "Waiting for model...".to_string()
            } else {
                match &status.state_application_status {
                    AgentStateApplicationStatus::Applied => "OK".to_string(),
                    AgentStateApplicationStatus::Fresh => "Pending".to_string(),
                    AgentStateApplicationStatus::AttemptedAndRetrying => "Retrying".to_string(),
                    AgentStateApplicationStatus::Stuck => "Retrying, but seems stuck?".to_string(),
                    AgentStateApplicationStatus::AttemptedAndNotAppliable => {
                        "Needs your help".to_string()
                    }
                }
            };

            status_row_left =
                status_row_left.push(text(format!("Status: {status_label}")).font(REGULAR));

            if !status.issues.is_empty() {
                status_row_left = status_row_left
                    .push(text(format!("{} issues", status.issues.len())).font(REGULAR));
            }

            slots_label = Some(format!(
                "{}/{}/{}",
                status.slots_processing, status.slots_total, status.desired_slots_total,
            ));
        }
    }

    let mut status_row_content = row![container(status_row_left).width(iced::Fill),];

    if let Some(label) = slots_label {
        status_row_content = status_row_content.push(text(format!("Slots: {label}")).font(REGULAR));
    }

    let card_content = column![name_row, status_row_content,].spacing(SPACING_BASE);

    let agent_card = container(card_content)
        .width(iced::Fill)
        .padding(SPACING_BASE)
        .style(style_agent_container);

    let connection_status = match &data.status {
        None => text("Connecting to the balancer...").font(REGULAR),
        Some(_) => text(format!(
            "Connected to the balancer at {}",
            data.cluster_address
        ))
        .font(REGULAR),
    };

    let status_row = container(
        row![
            container(connection_status).width(iced::Fill),
            disconnect_button,
        ]
        .align_y(iced::Center),
    )
    .padding([0.0, SPACING_BASE]);

    column![
        container(text("Your agent").size(FONT_SIZE_L2).font(BOLD)).padding([0.0, SPACING_BASE]),
        container(text("Agent details").font(BOLD)).padding([0.0, SPACING_BASE]),
        agent_card,
        status_row,
    ]
    .spacing(SPACING_2X)
    .into()
}
