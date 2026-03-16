mod color;
mod macros;
mod manager;
mod palette;
mod theme;
mod tokens;

pub use color::ThemeColor;
pub use manager::ThemeManager;
pub use palette::Palette;
pub use theme::Theme;
pub use tokens::{FontSize, Radius, RadiusScale, SpaceScale, Spacing, TypographyTokens};

mod generated {
    use crate::{
        FontSize, Palette, Radius, RadiusScale, SpaceScale, Spacing, Theme, ThemeColor,
        TypographyTokens,
    };

    include!(concat!(env!("OUT_DIR"), "/generated_theme.rs"));
}

pub use generated::generated_default_theme;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_theme_is_available() {
        let theme = generated_default_theme();
        assert_eq!(theme.name, "generated-default");
    }

    #[test]
    fn dsl_macro_creates_theme() {
        let theme = theme! {
            name: "macro-theme",
            palette: {
                background: [10, 11, 12, 255],
                surface: [20, 21, 22, 255],
                surface_alt: [30, 31, 32, 255],
                text_primary: [40, 41, 42, 255],
                text_muted: [50, 51, 52, 255],
                accent: [60, 61, 62, 255],
                accent_hover: [70, 71, 72, 255],
                selection: [80, 81, 82, 120],
            },
            space: { xs: 2.0, sm: 4.0, md: 8.0, lg: 12.0, xl: 16.0 },
            radius: { sm: 4.0, md: 8.0, lg: 12.0 },
            typography: { body: 14.0, heading: 18.0, mono: 13.0 },
        };

        assert_eq!(theme.name, "macro-theme");
        assert_eq!(theme.palette.accent.as_rgba_u8(), [60, 61, 62, 255]);
        assert_eq!(theme.space.md.value(), 8.0);
    }
}
