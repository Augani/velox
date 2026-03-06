use crate::ThemeColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub background: ThemeColor,
    pub surface: ThemeColor,
    pub surface_alt: ThemeColor,
    pub text_primary: ThemeColor,
    pub text_muted: ThemeColor,
    pub accent: ThemeColor,
    pub accent_hover: ThemeColor,
    pub selection: ThemeColor,
}

impl Palette {
    pub fn light() -> Self {
        Self {
            background: ThemeColor::rgb(244, 246, 250),
            surface: ThemeColor::rgb(255, 255, 255),
            surface_alt: ThemeColor::rgb(236, 240, 246),
            text_primary: ThemeColor::rgb(24, 28, 34),
            text_muted: ThemeColor::rgb(92, 102, 117),
            accent: ThemeColor::rgb(39, 109, 255),
            accent_hover: ThemeColor::rgb(24, 94, 238),
            selection: ThemeColor::rgba(39, 109, 255, 72),
        }
    }

    pub fn dark() -> Self {
        Self {
            background: ThemeColor::rgb(18, 20, 24),
            surface: ThemeColor::rgb(26, 29, 35),
            surface_alt: ThemeColor::rgb(34, 38, 46),
            text_primary: ThemeColor::rgb(235, 240, 246),
            text_muted: ThemeColor::rgb(155, 166, 179),
            accent: ThemeColor::rgb(104, 163, 255),
            accent_hover: ThemeColor::rgb(126, 177, 255),
            selection: ThemeColor::rgba(104, 163, 255, 78),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_palettes_are_distinct() {
        assert_ne!(Palette::light(), Palette::dark());
    }
}
