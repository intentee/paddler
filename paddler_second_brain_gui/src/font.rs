use iced::Font;
use iced::font::Family;
use iced::font::Weight;

pub const BOLD: Font = Font {
    family: Family::Name("JetBrains Mono"),
    weight: Weight::Bold,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};
