use iced::Background;
use iced::Border;
use iced::Theme;
use iced::widget::container;

use super::variables::COLOR_AGENT_BACKGROUND;
use super::variables::COLOR_BORDER;

pub fn style_agent_container(theme: &Theme) -> container::Style {
    let base = container::transparent(theme);

    container::Style {
        background: Some(Background::Color(COLOR_AGENT_BACKGROUND)),
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
    use iced::Background;
    use iced::Theme;

    use super::COLOR_AGENT_BACKGROUND;
    use super::COLOR_BORDER;
    use super::style_agent_container;

    #[test]
    fn agent_container_paints_orange_background_with_black_border() -> Result<()> {
        let style = style_agent_container(&Theme::Light);

        match style.background {
            Some(Background::Color(color)) if color == COLOR_AGENT_BACKGROUND => {}
            other => bail!("expected COLOR_AGENT_BACKGROUND, got {other:?}"),
        }

        if style.border.color != COLOR_BORDER {
            bail!("expected COLOR_BORDER border, got {:?}", style.border.color);
        }

        Ok(())
    }
}
