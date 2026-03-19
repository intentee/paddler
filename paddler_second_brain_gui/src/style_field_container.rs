use iced::Shadow;
use iced::Theme;
use iced::Vector;
use iced::widget::container;

use crate::variables::COLOR_BORDER;

pub fn style_field_container(theme: &Theme) -> container::Style {
    let base = container::transparent(theme);

    container::Style {
        shadow: Shadow {
            color: COLOR_BORDER,
            offset: Vector::new(4.0, 4.0),
            blur_radius: 0.0,
        },
        ..base
    }
}
