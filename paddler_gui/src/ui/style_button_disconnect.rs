use iced::Background;
use iced::Border;
use iced::Color;
use iced::Theme;
use iced::widget::button;

use super::variables::COLOR_ERROR;

pub fn style_button_disconnect(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::primary(theme, status);

    button::Style {
        background: Some(Background::Color(COLOR_ERROR)),
        text_color: Color::WHITE,
        border: Border {
            color: COLOR_ERROR,
            width: 0.0,
            radius: 0.into(),
        },
        ..base
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::bail;
    use iced::Background;
    use iced::Theme;
    use iced::widget::button;

    use super::COLOR_ERROR;
    use super::style_button_disconnect;

    #[test]
    fn disconnect_button_paints_error_red_background() -> Result<()> {
        let style = style_button_disconnect(&Theme::Light, button::Status::Active);

        match style.background {
            Some(Background::Color(color)) if color == COLOR_ERROR => Ok(()),
            other => bail!("expected COLOR_ERROR background, got {other:?}"),
        }
    }
}
