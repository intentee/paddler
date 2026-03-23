use iced::Element;
use iced::Fill;
use iced::widget::column;
use iced::widget::container;
use iced::widget::progress_bar;
use iced::widget::row;
use iced::widget::text;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

use super::font::BOLD;
use super::font::REGULAR;
use super::style_agent_container::style_agent_container;
use super::style_download_progress_bar::style_download_progress_bar;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use crate::message::Message;

fn display_last_path_part(path: &str) -> String {
    path.split('/').next_back().unwrap_or(path).to_string()
}

pub fn view_agent_card<'content>(
    snapshot: &'content AgentControllerSnapshot,
) -> Element<'content, Message> {
    let agent_name = snapshot.name.as_deref().unwrap_or(&snapshot.id);

    let is_downloading =
        snapshot.download_total > 0 && snapshot.download_current < snapshot.download_total;

    let mut name_row = row![container(text(agent_name.to_string()).font(BOLD)).width(Fill),];

    if is_downloading {
        name_row = name_row.push(
            progress_bar(
                0.0..=snapshot.download_total as f32,
                snapshot.download_current as f32,
            )
            .girth(12)
            .style(style_download_progress_bar),
        );
    } else {
        let model_label = match &snapshot.model_path {
            Some(path) => display_last_path_part(path),
            None => "No model loaded".to_string(),
        };

        name_row = name_row.push(text(model_label).font(REGULAR));
    }

    let status_label = if is_downloading {
        let percentage =
            (snapshot.download_current as f32 / snapshot.download_total as f32) * 100.0;

        format!("Downloading ({percentage:.0}%)")
    } else if snapshot.model_path.is_none() {
        "Waiting for model...".to_string()
    } else {
        match &snapshot.state_application_status {
            AgentStateApplicationStatus::Applied => "OK".to_string(),
            AgentStateApplicationStatus::Fresh => "Pending".to_string(),
            AgentStateApplicationStatus::AttemptedAndRetrying => "Retrying".to_string(),
            AgentStateApplicationStatus::Stuck => "Retrying, but seems stuck?".to_string(),
            AgentStateApplicationStatus::AttemptedAndNotAppliable => "Needs your help".to_string(),
        }
    };

    let mut status_row_left = column![].spacing(SPACING_HALF);

    status_row_left = status_row_left.push(text(format!("Status: {status_label}")).font(REGULAR));

    if !snapshot.issues.is_empty() {
        status_row_left =
            status_row_left.push(text(format!("{} issues", snapshot.issues.len())).font(REGULAR));
    }

    let slots_label = format!(
        "{}/{}/{}",
        snapshot.slots_processing, snapshot.slots_total, snapshot.desired_slots_total,
    );

    let status_row_content = row![
        container(status_row_left).width(Fill),
        text(format!("Slots: {slots_label}")).font(REGULAR),
    ];

    let card_content = column![name_row, status_row_content,].spacing(SPACING_BASE);

    container(card_content)
        .width(Fill)
        .padding(SPACING_BASE)
        .style(style_agent_container)
        .into()
}
