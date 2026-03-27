use iced::Background;
use iced::Border;
use iced::Center;
use iced::Color;
use iced::Element;
use iced::Fill;
use iced::Theme;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::svg;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::text;

use super::font::BOLD;
use super::font::REGULAR;
use super::style_button_disconnect::style_button_disconnect;
use super::style_card_container::style_card_container;
use super::variables::FONT_SIZE_L2;
use super::variables::SPACING_2X;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use super::view_agent_card::view_agent_card;
use crate::running_cluster_data::RunningClusterData;
use crate::running_cluster_handler::Message;

pub fn view_running_cluster(data: &RunningClusterData) -> Element<'_, Message> {
    let copy_icon = svg(SvgHandle::from_memory(
        include_bytes!("../../../resources/icons/copy.svg").as_slice(),
    ))
    .width(16)
    .height(16);

    let address_row = container(
        row![
            container(text(format!("Cluster address: {}", data.cluster_address)).font(REGULAR))
                .width(Fill),
            button(
                row![copy_icon, text("Copy address").font(BOLD)]
                    .spacing(SPACING_BASE / 2.0)
                    .align_y(Center),
            )
            .style(button::text)
            .on_press(Message::CopyToClipboard(data.cluster_address.clone())),
        ]
        .align_y(Center)
        .padding(SPACING_BASE),
    )
    .style(style_card_container);

    let stop_icon = svg(SvgHandle::from_memory(
        include_bytes!("../../../resources/icons/stop.svg").as_slice(),
    ))
    .width(16)
    .height(16);

    let status_indicator = container("").width(16).height(16).style(|theme: &Theme| {
        let base = container::transparent(theme);

        container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xEE, 0xFF, 0xEE))),
            border: Border {
                color: Color::from_rgb8(0xCC, 0xDD, 0xCC),
                width: 2.0,
                radius: 8.into(),
            },
            ..base
        }
    });

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
        status_row,
        container(text("Connected agents").font(BOLD)).padding([0.0, SPACING_BASE]),
    ]
    .spacing(SPACING_2X);

    if data.agent_snapshots.is_empty() {
        content = content.push(
            container(text("Waiting for agents to connect...").font(REGULAR))
                .padding([0.0, SPACING_BASE]),
        );
    } else {
        for agent_snapshot in &data.agent_snapshots {
            content = content.push(view_agent_card(agent_snapshot));
        }
    }

    content.into()
}
