use crate::{Palette, RadiusScale, SpaceScale, TypographyTokens};

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub palette: Palette,
    pub space: SpaceScale,
    pub radius: RadiusScale,
    pub typography: TypographyTokens,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            name: "light",
            palette: Palette::light(),
            space: SpaceScale::compact(),
            radius: RadiusScale::rounded(),
            typography: TypographyTokens::desktop_defaults(),
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "dark",
            palette: Palette::dark(),
            space: SpaceScale::compact(),
            radius: RadiusScale::rounded(),
            typography: TypographyTokens::desktop_defaults(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}
