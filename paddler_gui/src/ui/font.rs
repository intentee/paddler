use iced::Font;
use iced::font::Family;
use iced::font::Stretch;
use iced::font::Style;
use iced::font::Weight;

pub const REGULAR: Font = Font {
    family: Family::Name("JetBrains Mono"),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

pub const BOLD: Font = Font {
    family: Family::Name("JetBrains Mono"),
    weight: Weight::Bold,
    stretch: Stretch::Normal,
    style: Style::Normal,
};
