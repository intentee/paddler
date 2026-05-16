use iced::Background;
use iced::Border;
use iced::Color;
use iced::Theme;
use iced::overlay::menu;

use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BODY_FONT;
use super::variables::COLOR_BORDER;

pub fn style_field_pick_list_menu(theme: &Theme) -> menu::Style {
    let base = menu::default(theme);

    menu::Style {
        background: Background::Color(COLOR_BODY_BACKGROUND),
        border: Border {
            color: COLOR_BORDER,
            width: 1.0,
            radius: 0.into(),
        },
        text_color: COLOR_BODY_FONT,
        selected_text_color: COLOR_BODY_FONT,
        selected_background: Background::Color(Color::from_rgb(0.9, 0.9, 0.9)),
        ..base
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::bail;
    use iced::Theme;

    use super::COLOR_BORDER;
    use super::style_field_pick_list_menu;

    #[test]
    fn pick_list_open_menu_outlines_options_with_border_color() -> Result<()> {
        let style = style_field_pick_list_menu(&Theme::Light);

        if style.border.color != COLOR_BORDER {
            bail!("expected menu border in COLOR_BORDER");
        }

        Ok(())
    }
}
