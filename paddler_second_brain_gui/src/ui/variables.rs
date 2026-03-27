use iced::Color;

// Font sizes

const FONT_SIZE_BASE: f32 = 14.0;
const FONT_SIZE_L1: f32 = 1.5 * FONT_SIZE_BASE;
pub const FONT_SIZE_L2: f32 = 1.5 * FONT_SIZE_L1;

// Spacing

pub const SPACING_BASE: f32 = 16.0;
pub const SPACING_2X: f32 = 2.0 * SPACING_BASE;
pub const SPACING_HALF: f32 = 0.5 * SPACING_BASE;

// Colors

pub const COLOR_BODY_BACKGROUND: Color = Color::WHITE;
pub const COLOR_BODY_FONT: Color = Color {
    r: 0.067,
    g: 0.067,
    b: 0.067,
    a: 1.0,
};
pub const COLOR_AGENT_BACKGROUND: Color = Color {
    r: 250.0 / 255.0,
    g: 240.0 / 255.0,
    b: 230.0 / 255.0,
    a: 1.0,
};
pub const COLOR_BORDER: Color = Color::BLACK;
pub const COLOR_ERROR: Color = Color {
    r: 204.0 / 255.0,
    g: 51.0 / 255.0,
    b: 51.0 / 255.0,
    a: 1.0,
};
