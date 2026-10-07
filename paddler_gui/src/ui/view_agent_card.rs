use std::path::Path;

use iced::Element;
use iced::Fill;
use iced::widget::column;
use iced::widget::container;
use iced::widget::progress_bar;
use iced::widget::row;
use iced::widget::text;

use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::model_download_status::ModelDownloadStatus;

use super::font::BOLD;
use super::font::REGULAR;
use super::style_agent_container::style_agent_container;
use super::style_download_progress_bar::style_download_progress_bar;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;

fn display_last_path_part(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_owned()
}

fn view_download_progress_bar<'card, TMessage: 'static>(
    downloaded_bytes: u64,
    total_bytes: u64,
) -> Element<'card, TMessage> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "download sizes fit in f32 mantissa"
    )]
    progress_bar(0.0..=total_bytes as f32, downloaded_bytes as f32)
        .girth(12)
        .style(style_download_progress_bar)
        .into()
}

fn agent_status_label(status: &AgentStatus) -> String {
    match &status.download_status {
        ModelDownloadStatus::Downloading {
            downloaded_bytes,
            total_bytes,
            ..
        } => {
            #[expect(
                clippy::cast_precision_loss,
                reason = "download sizes fit in f32 mantissa"
            )]
            let percentage = (*downloaded_bytes as f32 / *total_bytes as f32) * 100.0;

            format!("Downloading ({percentage:.0}%)")
        }
        ModelDownloadStatus::DownloadingWithUnknownSize { .. } => "Downloading...".to_owned(),
        ModelDownloadStatus::NotDownloading if status.model_path.is_none() => {
            "Waiting for model...".to_owned()
        }
        ModelDownloadStatus::NotDownloading => match &status.state_application_status {
            AgentStateApplicationStatus::Applied => "OK".to_owned(),
            AgentStateApplicationStatus::Fresh => "Pending".to_owned(),
            AgentStateApplicationStatus::AttemptedAndRetrying => "Retrying".to_owned(),
            AgentStateApplicationStatus::Stuck => "Retrying, but seems stuck?".to_owned(),
            AgentStateApplicationStatus::AttemptedAndNotAppliable => "Needs your help".to_owned(),
        },
    }
}

fn model_label(status: &AgentStatus) -> String {
    status.model_path.as_ref().map_or_else(
        || "No model loaded".to_owned(),
        |path| display_last_path_part(path),
    )
}

pub fn view_agent_card<'card, TMessage: 'static>(
    name: Option<&'card str>,
    slots_processing: u64,
    status: &'card AgentStatus,
) -> Element<'card, TMessage> {
    let name_cell =
        container(name.map_or_else(|| text(""), |agent_name| text(agent_name).font(BOLD)))
            .width(Fill);

    let download_cell = match &status.download_status {
        ModelDownloadStatus::Downloading {
            downloaded_bytes,
            total_bytes,
            ..
        } => view_download_progress_bar(*downloaded_bytes, *total_bytes),
        ModelDownloadStatus::DownloadingWithUnknownSize { .. }
        | ModelDownloadStatus::NotDownloading => text(model_label(status)).font(REGULAR).into(),
    };
    let status_label = agent_status_label(status);

    let name_row = row![name_cell, download_cell];

    let mut status_row_left = column![].spacing(SPACING_HALF);

    status_row_left = status_row_left.push(text(format!("Status: {status_label}")).font(REGULAR));

    if !status.issues.is_empty() {
        status_row_left =
            status_row_left.push(text(format!("{} issues", status.issues.len())).font(REGULAR));
    }

    let slots_label = format!(
        "{slots_processing}/{}/{}",
        status.runtime.slots_total(),
        status.desired_slots_total,
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

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::agent_status::AgentStatus;
    use paddler_messaging::model_download_status::ModelDownloadStatus;

    use super::agent_status_label;
    use super::model_label;

    fn status_with_model(state_application_status: AgentStateApplicationStatus) -> AgentStatus {
        AgentStatus {
            model_path: Some("/models/qwen3.gguf".to_owned()),
            state_application_status,
            ..AgentStatus::default()
        }
    }

    #[test]
    fn labels_a_download_of_known_size_with_its_progress() {
        let status = AgentStatus {
            download_status: ModelDownloadStatus::Downloading {
                downloaded_bytes: 250,
                model_path: "owner/repo/main/model.gguf".to_owned(),
                total_bytes: 1000,
            },
            ..AgentStatus::default()
        };

        assert_eq!(agent_status_label(&status), "Downloading (25%)");
    }

    #[test]
    fn labels_a_download_of_unknown_size_without_progress() {
        let status = AgentStatus {
            download_status: ModelDownloadStatus::DownloadingWithUnknownSize {
                downloaded_bytes: 250,
                model_path: "https://example.com/model.gguf".to_owned(),
            },
            ..AgentStatus::default()
        };

        assert_eq!(agent_status_label(&status), "Downloading...");
    }

    #[test]
    fn labels_an_agent_without_a_model_as_waiting() {
        assert_eq!(
            agent_status_label(&AgentStatus::default()),
            "Waiting for model..."
        );
    }

    #[test]
    fn labels_each_state_application_status_of_a_loaded_agent() {
        assert_eq!(
            [
                AgentStateApplicationStatus::Applied,
                AgentStateApplicationStatus::Fresh,
                AgentStateApplicationStatus::AttemptedAndRetrying,
                AgentStateApplicationStatus::Stuck,
                AgentStateApplicationStatus::AttemptedAndNotAppliable,
            ]
            .map(|state_application_status| {
                agent_status_label(&status_with_model(state_application_status))
            }),
            [
                "OK",
                "Pending",
                "Retrying",
                "Retrying, but seems stuck?",
                "Needs your help"
            ]
        );
    }

    #[test]
    fn names_the_loaded_model_by_its_file_name() {
        assert_eq!(
            model_label(&status_with_model(AgentStateApplicationStatus::Applied)),
            "qwen3.gguf"
        );
    }

    #[test]
    fn names_a_missing_model() {
        assert_eq!(model_label(&AgentStatus::default()), "No model loaded");
    }
}
