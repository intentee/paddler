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

use super::font::BOLD;
use super::style_button_primary::style_button_primary;
use super::variables::FONT_SIZE_L2;
use super::variables::SPACING_2X;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use super::view_error_message::view_error_message;
use crate::home_data::HomeData;
use crate::home_message::HomeMessage;

static CREATE_CLUSTER_IMAGE: LazyLock<ImageHandle> = LazyLock::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../resources/images/create_a_cluster.png").as_slice(),
    )
});

static JOIN_CLUSTER_IMAGE: LazyLock<ImageHandle> = LazyLock::new(|| {
    ImageHandle::from_bytes(
        include_bytes!("../../../resources/images/join_a_cluster.png").as_slice(),
    )
});

impl HomeData {
    pub fn view(&self) -> Element<'_, HomeMessage> {
        let create_image = image(CREATE_CLUSTER_IMAGE.clone()).width(200).height(200);

        let join_image = image(JOIN_CLUSTER_IMAGE.clone()).width(200).height(200);

        let start_button = button(text("Start a cluster").font(BOLD))
            .padding([SPACING_HALF, SPACING_BASE])
            .style(style_button_primary)
            .on_press(HomeMessage::StartBalancer);

        let join_button = button(text("Join a cluster").font(BOLD))
            .padding([SPACING_HALF, SPACING_BASE])
            .style(style_button_primary)
            .on_press(HomeMessage::JoinBalancer);

        let start_column = column![create_image, start_button]
            .spacing(SPACING_BASE)
            .align_x(Center);

        let join_column = column![join_image, join_button]
            .spacing(SPACING_BASE)
            .align_x(Center);

        let options_row = row![start_column, join_column].spacing(SPACING_2X);

        let mut content = column![
            container(text("Paddler App").size(FONT_SIZE_L2).font(BOLD))
                .padding([0.0, SPACING_BASE]),
            container(options_row).align_x(Center),
        ]
        .spacing(SPACING_2X);

        if let Some(error) = &self.error {
            content = content.push(view_error_message(&error.to_string()));
        }

        content.into()
    }
}
