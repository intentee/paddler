use iced::Shadow;
use iced::Theme;
use iced::Vector;
use iced::widget::container;

use super::variables::COLOR_BORDER;

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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::bail;
    use iced::Theme;

    use super::COLOR_BORDER;
    use super::style_field_container;

    #[test]
    fn field_container_casts_a_solid_offset_shadow_in_border_color() -> Result<()> {
        let style = style_field_container(&Theme::Light);

        if style.shadow.color != COLOR_BORDER {
            bail!("expected shadow color == COLOR_BORDER");
        }

        if style.shadow.offset.x != 4.0 || style.shadow.offset.y != 4.0 {
            bail!("expected shadow offset (4.0, 4.0)");
        }

        Ok(())
    }
}
