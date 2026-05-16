use iced::Background;
use iced::Border;
use iced::Color;
use iced::Theme;
use iced::widget::container;

pub fn style_status_indicator(theme: &Theme) -> container::Style {
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
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::bail;
    use iced::Background;
    use iced::Color;
    use iced::Theme;

    use super::style_status_indicator;

    #[test]
    fn status_indicator_paints_a_pale_green_pill_against_a_muted_green_border() -> Result<()> {
        let style = style_status_indicator(&Theme::Light);

        match style.background {
            Some(Background::Color(color)) if color == Color::from_rgb8(0xEE, 0xFF, 0xEE) => {}
            other => bail!("expected pale green background, got {other:?}"),
        }

        if style.border.color != Color::from_rgb8(0xCC, 0xDD, 0xCC) {
            bail!("expected muted green border");
        }

        Ok(())
    }
}
