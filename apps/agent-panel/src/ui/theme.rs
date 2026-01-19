//! Catppuccin theme colors
//!
//! Matches the Waybar theme for consistency

use iced::Color;

/// Theme colors
#[derive(Debug, Clone)]
pub struct CatppuccinTheme {
    // Base colors
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,

    // Surface colors
    pub surface0: Color,
    pub surface1: Color,
    pub surface2: Color,

    // Text colors
    pub text: Color,
    pub subtext0: Color,
    pub subtext1: Color,

    // Accent colors
    pub blue: Color,
    pub green: Color,
    pub peach: Color,
    pub red: Color,
    pub mauve: Color,
    pub teal: Color,
    pub yellow: Color,
}

impl CatppuccinTheme {
    /// Catppuccin Mocha palette
    pub fn mocha() -> Self {
        Self {
            // Base
            base: hex_to_color("#1e1e2e"),
            mantle: hex_to_color("#181825"),
            crust: hex_to_color("#11111b"),

            // Surface
            surface0: hex_to_color("#313244"),
            surface1: hex_to_color("#45475a"),
            surface2: hex_to_color("#585b70"),

            // Text
            text: hex_to_color("#cdd6f4"),
            subtext0: hex_to_color("#a6adc8"),
            subtext1: hex_to_color("#bac2de"),

            // Accents
            blue: hex_to_color("#89b4fa"),
            green: hex_to_color("#a6e3a1"),
            peach: hex_to_color("#fab387"),
            red: hex_to_color("#f38ba8"),
            mauve: hex_to_color("#cba6f7"),
            teal: hex_to_color("#94e2d5"),
            yellow: hex_to_color("#f9e2af"),
        }
    }
}

/// Convert hex color string to iced Color
fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
    Color::from_rgb(r, g, b)
}
