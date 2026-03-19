use std::sync::LazyLock;

use iced::Center;
use iced::Element;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::image;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::row;
use iced::widget::text;

use crate::font::BOLD;
use crate::message::Message;
use crate::style_button_primary::style_button_primary;
use crate::variables::FONT_SIZE_L2;
use crate::variables::SPACING_2X;
use crate::variables::SPACING_BASE;
use crate::variables::SPACING_HALF;

static CREATE_CLUSTER_IMAGE: LazyLock<ImageHandle> = LazyLock::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../resources/images/create_a_cluster.png").as_slice(),
    )
});

static JOIN_CLUSTER_IMAGE: LazyLock<ImageHandle> = LazyLock::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../resources/images/join_a_cluster.png").as_slice(),
    )
});

pub fn view_home() -> Element<'static, Message> {
    let create_image = image(CREATE_CLUSTER_IMAGE.clone())
        .width(200)
        .height(200);

    let join_image = image(JOIN_CLUSTER_IMAGE.clone())
        .width(200)
        .height(200);

    let start_button = button(text("Start a cluster").font(BOLD))
        .padding([SPACING_HALF, SPACING_BASE])
        .style(style_button_primary)
        .on_press(Message::StartCluster);

    let join_button = button(text("Join a cluster").font(BOLD))
        .padding([SPACING_HALF, SPACING_BASE])
        .style(style_button_primary)
        .on_press(Message::JoinCluster);

    let start_column = column![create_image, start_button]
        .spacing(SPACING_BASE)
        .align_x(Center);

    let join_column = column![join_image, join_button]
        .spacing(SPACING_BASE)
        .align_x(Center);

    let options_row = row![start_column, join_column].spacing(SPACING_2X);

    column![
        container(text("Paddler second brain").size(FONT_SIZE_L2).font(BOLD))
            .padding([0.0, SPACING_BASE]),
        container(options_row).align_x(Center),
    ]
    .spacing(SPACING_2X)
    .into()
}
