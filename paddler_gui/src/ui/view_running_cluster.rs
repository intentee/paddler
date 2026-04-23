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
use paddler_types::agent_desired_model::AgentDesiredModel;

use super::font::BOLD;
use super::font::REGULAR;
use super::style_button_disconnect::style_button_disconnect;
use super::style_card_container::style_card_container;
use super::style_status_indicator::style_status_indicator;
use super::variables::FONT_SIZE_L2;
use super::variables::SPACING_2X;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use super::view_agent_card::view_agent_card;
use crate::running_cluster_data::RunningClusterData;
use crate::running_cluster_handler::Message;

fn format_desired_model(desired_model: &AgentDesiredModel) -> String {
    match desired_model {
        AgentDesiredModel::HuggingFace(reference) => {
            format!(
                "HuggingFace {}/{} ({})",
                reference.repo_id, reference.filename, reference.revision,
            )
        }
        AgentDesiredModel::LocalToAgent(path) => format!("Local: {path}"),
        AgentDesiredModel::None => "(not set)".to_owned(),
    }
}

pub fn view_running_cluster(data: &RunningClusterData) -> Element<'_, Message> {
    let copy_icon = svg(SvgHandle::from_memory(
        include_bytes!("../../../resources/icons/copy.svg").as_slice(),
    ))
    .width(16)
    .height(16);

    let desired_model_label = format_desired_model(&data.snapshot.balancer_desired_state.model);
    let applied_model_label = data
        .snapshot
        .balancer_applicable_state
        .as_ref()
        .map_or_else(
            || "(reconciling...)".to_owned(),
            |applicable| format_desired_model(&applicable.agent_desired_state.model),
        );

    let address_row = container(
        column![
            row![
                container(text(format!("Cluster address: {}", data.cluster_address)).font(REGULAR))
                    .width(Fill),
                button(
                    row![copy_icon, text("Copy address").font(BOLD)]
                        .spacing(SPACING_HALF)
                        .align_y(Center),
                )
                .style(button::text)
                .on_press(Message::CopyToClipboard(data.cluster_address.clone())),
            ]
            .align_y(Center),
            text(format!("Configured model: {desired_model_label}")).font(REGULAR),
            text(format!("Applied model: {applied_model_label}")).font(REGULAR),
        ]
        .spacing(SPACING_HALF)
        .padding(SPACING_BASE),
    )
    .style(style_card_container);

    let stop_icon = svg(SvgHandle::from_memory(
        include_bytes!("../../../resources/icons/stop.svg").as_slice(),
    ))
    .width(16)
    .height(16);

    let status_indicator = container("")
        .width(16)
        .height(16)
        .style(style_status_indicator);

    let stop_button = if data.stopping {
        button(
            row![stop_icon, text("Stopping...").font(BOLD)]
                .spacing(SPACING_HALF)
                .align_y(Center),
        )
        .padding([SPACING_HALF, SPACING_BASE])
        .style(style_button_disconnect)
    } else {
        button(
            row![stop_icon, text("Stop cluster").font(BOLD)]
                .spacing(SPACING_HALF)
                .align_y(Center),
        )
        .padding([SPACING_HALF, SPACING_BASE])
        .style(style_button_disconnect)
        .on_press(Message::Stop)
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
        container(text("Your cluster").size(FONT_SIZE_L2).font(BOLD)).padding([0.0, SPACING_BASE]),
        address_row,
    ]
    .spacing(SPACING_2X);

    if let Some(address) = &data.web_admin_panel_address {
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
                    .on_press(Message::OpenUrl(format!("http://{address}"))),
                ]
                .align_y(Center)
                .padding(SPACING_BASE),
            )
            .style(style_card_container),
        );
    }

    content = content.push(status_row);
    content =
        content.push(container(text("Connected agents").font(BOLD)).padding([0.0, SPACING_BASE]));

    if data.snapshot.agent_snapshots.is_empty() {
        content = content.push(
            container(text("Waiting for agents to connect...").font(REGULAR))
                .padding([0.0, SPACING_BASE]),
        );
    } else {
        for agent_snapshot in &data.snapshot.agent_snapshots {
            content = content.push(view_agent_card(agent_snapshot));
        }
    }

    content.into()
}
