use iced::Background;
use iced::Theme;
use iced::widget::checkbox;

use super::variables::BORDER_OUTLINE;
use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BODY_FONT;
use super::variables::COLOR_BORDER;

pub const fn style_field_checkbox(_theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let is_checked = matches!(
        status,
        checkbox::Status::Active { is_checked: true }
            | checkbox::Status::Hovered { is_checked: true }
            | checkbox::Status::Disabled { is_checked: true }
    );

    let background = if is_checked {
        COLOR_BORDER
    } else {
        COLOR_BODY_BACKGROUND
    };
    let icon_color = if is_checked {
        COLOR_BODY_BACKGROUND
    } else {
        COLOR_BODY_FONT
    };

    checkbox::Style {
        background: Background::Color(background),
        border: BORDER_OUTLINE,
        icon_color,
        text_color: Some(COLOR_BODY_FONT),
    }
}

#[cfg(test)]
mod tests {
    use iced::Background;
    use iced::Theme;
    use iced::widget::checkbox::Status;

    use super::style_field_checkbox;
    use crate::ui::variables::COLOR_BODY_BACKGROUND;
    use crate::ui::variables::COLOR_BODY_FONT;
    use crate::ui::variables::COLOR_BORDER;

    #[test]
    fn fills_a_checked_box_and_inverts_its_mark() {
        let checked_style =
            style_field_checkbox(&Theme::Light, Status::Hovered { is_checked: true });

        assert_eq!(checked_style.background, Background::Color(COLOR_BORDER));
        assert_eq!(checked_style.icon_color, COLOR_BODY_BACKGROUND);
    }

    #[test]
    fn leaves_an_unchecked_box_blank() {
        let unchecked_style =
            style_field_checkbox(&Theme::Light, Status::Active { is_checked: false });

        assert_eq!(
            unchecked_style.background,
            Background::Color(COLOR_BODY_BACKGROUND)
        );
        assert_eq!(unchecked_style.icon_color, COLOR_BODY_FONT);
    }
}
