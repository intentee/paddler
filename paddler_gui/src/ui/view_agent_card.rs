use iced::Element;
use iced::Fill;
use iced::widget::column;
use iced::widget::container;
use iced::widget::progress_bar;
use iced::widget::row;
use iced::widget::text;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;

use super::agent_status_label::agent_status_label;
use super::display_last_path_part::display_last_path_part;
use super::font::BOLD;
use super::font::REGULAR;
use super::style_agent_container::style_agent_container;
use super::style_download_progress_bar::style_download_progress_bar;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;

pub fn view_agent_card<TMessage: 'static>(
    snapshot: &AgentControllerSnapshot,
) -> Element<'_, TMessage> {
    let is_downloading =
        snapshot.download_total > 0 && snapshot.download_current < snapshot.download_total;

    let mut name_row = row![];

    match &snapshot.name {
        Some(agent_name) => {
            name_row = name_row.push(container(text(agent_name.clone()).font(BOLD)).width(Fill));
        }
        None => {
            name_row = name_row.push(container("").width(Fill));
        }
    }

    if is_downloading {
        name_row = name_row.push(
            #[expect(
                clippy::cast_precision_loss,
                reason = "download sizes fit in f32 mantissa"
            )]
            progress_bar(
                0.0..=snapshot.download_total as f32,
                snapshot.download_current as f32,
            )
            .girth(12)
            .style(style_download_progress_bar),
        );
    } else {
        let model_label = snapshot.model_path.as_ref().map_or_else(
            || "No model loaded".to_owned(),
            |path| display_last_path_part(path),
        );

        name_row = name_row.push(text(model_label).font(REGULAR));
    }

    let status_label = agent_status_label(snapshot);

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
