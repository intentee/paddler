use iced::Background;
use iced::Border;
use iced::Theme;
use iced::widget::container;

use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BORDER;

pub fn style_card_container(theme: &Theme) -> container::Style {
    let base = container::transparent(theme);

    container::Style {
        background: Some(Background::Color(COLOR_BODY_BACKGROUND)),
        border: Border {
            color: COLOR_BORDER,
            width: 2.0,
            radius: 0.into(),
        },
        ..base
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::bail;
    use iced::Theme;

    use super::COLOR_BORDER;
    use super::style_card_container;

    #[test]
    fn card_container_has_border_in_outline_color() -> Result<()> {
        let style = style_card_container(&Theme::Light);

        if style.border.color != COLOR_BORDER {
            bail!("expected COLOR_BORDER border, got {:?}", style.border.color);
        }
        Ok(())
    }
}
