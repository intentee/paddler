use iced::Element;
use iced::widget::button;
use iced::widget::column;
use iced::widget::progress_bar;
use iced::widget::text;

use crate::agent_running_data::AgentRunningData;
use crate::message::Message;

pub fn view_agent_running<'content>(
    data: &'content AgentRunningData,
) -> Element<'content, Message> {
    let mut content = column![button("Disconnect").on_press(Message::Disconnect),].spacing(10);

    match &data.status {
        None => {
            content = content.push(text("Connecting to cluster..."));
        }
        Some(status) => {
            if status.download_total > 0 && status.download_current < status.download_total {
                let percentage =
                    (status.download_current as f32 / status.download_total as f32) * 100.0;
                let download_label = match &status.download_filename {
                    Some(filename) => format!("Downloading {filename} ({percentage:.0}%)"),
                    None => format!("Downloading model ({percentage:.0}%)"),
                };

                content = content.push(text(download_label));
                content = content.push(progress_bar(
                    0.0..=status.download_total as f32,
                    status.download_current as f32,
                ));
            } else if status.model_path.is_some() {
                let status_label = format!(
                    "Ready ({}/{} slots busy)",
                    status.slots_processing, status.slots_total,
                );

                content = content.push(text(status_label));
            } else {
                content = content.push(text("Waiting for model..."));
            }

            if !status.issues.is_empty() {
                content = content.push(text(format!("{} issues", status.issues.len())));
            }
        }
    }

    content.into()
}
