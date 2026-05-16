use iced::Background;
use iced::Border;
use iced::Theme;
use iced::widget::text_input;

use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BODY_FONT;
use super::variables::COLOR_BORDER;

pub fn style_field_text_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let base = text_input::default(theme, status);

    text_input::Style {
        background: Background::Color(COLOR_BODY_BACKGROUND),
        border: Border {
            color: COLOR_BORDER,
            width: 2.0,
            radius: 0.into(),
        },
        icon: COLOR_BODY_FONT,
        placeholder: base.placeholder,
        value: COLOR_BODY_FONT,
        ..base
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use iced::Theme;
    use iced::widget::text_input;

    use super::COLOR_BORDER;
    use super::style_field_text_input;

    #[test]
    fn text_input_outlines_with_border_color_at_two_pixels() -> Result<()> {
        let style = style_field_text_input(&Theme::Light, text_input::Status::Active);

        assert_eq!(
            style.border.color, COLOR_BORDER,
            "expected border in COLOR_BORDER"
        );
        assert!(
            (style.border.width - 2.0).abs() <= f32::EPSILON,
            "expected border width of 2.0"
        );

        Ok(())
    }
}
