use iced::Background;
use iced::Border;
use iced::Theme;
use iced::widget::button;

use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BORDER;

pub fn style_button_primary(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::primary(theme, status);

    button::Style {
        background: Some(Background::Color(COLOR_BORDER)),
        text_color: COLOR_BODY_BACKGROUND,
        border: Border {
            color: COLOR_BORDER,
            width: 0.0,
            radius: 0.into(),
        },
        ..base
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use iced::Background;
    use iced::Theme;
    use iced::widget::button;

    use super::COLOR_BODY_BACKGROUND;
    use super::COLOR_BORDER;
    use super::style_button_primary;

    #[test]
    fn primary_button_paints_border_color_background_with_body_background_text() -> Result<()> {
        let style = style_button_primary(&Theme::Light, button::Status::Active);

        assert!(matches!(
            style.background,
            Some(Background::Color(color)) if color == COLOR_BORDER
        ));
        assert_eq!(
            style.text_color, COLOR_BODY_BACKGROUND,
            "expected text_color == COLOR_BODY_BACKGROUND"
        );

        Ok(())
    }
}
