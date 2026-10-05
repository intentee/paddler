use iced::Center;
use iced::Element;
use iced::Fill;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::svg;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::text;

use paddler_messaging::agent_desired_model::AgentDesiredModel;

use super::font::BOLD;
use super::font::REGULAR;
use super::style_card_container::style_card_container;
use super::style_status_indicator::style_status_indicator;
use super::variables::FONT_SIZE_L2;
use super::variables::SPACING_2X;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use super::view_agent_card::view_agent_card;
use super::view_stop_button::view_stop_button;
use crate::running_balancer_data::RunningBalancerData;
use crate::running_balancer_message::RunningBalancerMessage;

fn format_desired_model(desired_model: &AgentDesiredModel) -> String {
    match desired_model {
        AgentDesiredModel::HuggingFace(reference) => {
            format!(
                "HuggingFace {}/{} ({})",
                reference.repo_id, reference.filename, reference.revision,
            )
        }
        AgentDesiredModel::LocalToAgent(path) => format!("Local: {path}"),
        AgentDesiredModel::Url(reference) => format!("URL: {}", reference.url),
        AgentDesiredModel::None => "(not set)".to_owned(),
    }
}

impl RunningBalancerData {
    pub fn view(&self) -> Element<'_, RunningBalancerMessage> {
        let copy_icon = svg(SvgHandle::from_memory(
            include_bytes!("../../../resources/icons/copy.svg").as_slice(),
        ))
        .width(16)
        .height(16);

        let desired_model_label = format_desired_model(&self.snapshot.balancer_desired_state.model);
        let applied_model_label = format_desired_model(
            &self
                .snapshot
                .balancer_applicable_state
                .agent_desired_state
                .model,
        );

        let address_row = container(
            column![
                row![
                    container(
                        text(format!("Cluster address: {}", self.addresses.management))
                            .font(REGULAR)
                    )
                    .width(Fill),
                    button(
                        row![copy_icon, text("Copy address").font(BOLD)]
                            .spacing(SPACING_HALF)
                            .align_y(Center),
                    )
                    .style(button::text)
                    .on_press(RunningBalancerMessage::CopyToClipboard(
                        self.addresses.management.to_string()
                    )),
                ]
                .align_y(Center),
                text(format!("Configured model: {desired_model_label}")).font(REGULAR),
                text(format!("Applied model: {applied_model_label}")).font(REGULAR),
            ]
            .spacing(SPACING_HALF)
            .padding(SPACING_BASE),
        )
        .style(style_card_container);

        let status_indicator = container("")
            .width(16)
            .height(16)
            .style(style_status_indicator);

        let stop_button = if self.stopping {
            view_stop_button("Stopping...", None)
        } else {
            view_stop_button("Stop cluster", Some(RunningBalancerMessage::Stop))
        };

        let status_row = container(
            row![
                container(
                    row![text("Cluster is running").font(REGULAR), status_indicator,]
                        .spacing(SPACING_HALF)
                        .align_y(Center),
                )
                .width(Fill),
                stop_button,
            ]
            .align_y(Center),
        )
        .padding([SPACING_HALF, SPACING_BASE]);

        let mut content = column![
            container(text("Your cluster").size(FONT_SIZE_L2).font(BOLD))
                .padding([0.0, SPACING_BASE]),
            address_row,
        ]
        .spacing(SPACING_2X);

        if let Some(address) = self.addresses.web_admin_panel {
            let open_in_new_icon = svg(SvgHandle::from_memory(
                include_bytes!("../../../resources/icons/open_in_new.svg").as_slice(),
            ))
            .width(16)
            .height(16);

            content = content.push(
                container(
                    row![
                        container(text(format!("Web admin panel: {address}")).font(REGULAR))
                            .width(Fill),
                        button(
                            row![open_in_new_icon, text("Open in browser").font(BOLD)]
                                .spacing(SPACING_HALF)
                                .align_y(Center),
                        )
                        .style(button::text)
                        .on_press(RunningBalancerMessage::OpenUrl(format!("http://{address}"))),
                    ]
                    .align_y(Center)
                    .padding(SPACING_BASE),
                )
                .style(style_card_container),
            );
        }

        content = content.push(status_row);
        content = content
            .push(container(text("Connected agents").font(BOLD)).padding([0.0, SPACING_BASE]));

        if self.snapshot.agent_snapshots.is_empty() {
            content = content.push(
                container(text("Waiting for agents to connect...").font(REGULAR))
                    .padding([0.0, SPACING_BASE]),
            );
        } else {
            for agent_snapshot in &self.snapshot.agent_snapshots {
                content = content.push(view_agent_card(
                    agent_snapshot.name.as_deref(),
                    agent_snapshot.slots_processing,
                    &agent_snapshot.status,
                ));
            }
        }

        content.into()
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_messaging::url_model_reference::UrlModelReference;

    use super::format_desired_model;

    #[test]
    fn describes_each_model_source() {
        assert_eq!(
            [
                AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
                    repo_id: "unsloth/Qwen3-0.6B-GGUF".to_owned(),
                    filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
                    revision: "main".to_owned(),
                }),
                AgentDesiredModel::LocalToAgent("/models/qwen3.gguf".to_owned()),
                AgentDesiredModel::Url(UrlModelReference {
                    url: "https://example.com/qwen3.gguf".to_owned(),
                }),
                AgentDesiredModel::None,
            ]
            .map(|desired_model| format_desired_model(&desired_model)),
            [
                "HuggingFace unsloth/Qwen3-0.6B-GGUF/Qwen3-0.6B-Q8_0.gguf (main)",
                "Local: /models/qwen3.gguf",
                "URL: https://example.com/qwen3.gguf",
                "(not set)",
            ]
        );
    }
}
