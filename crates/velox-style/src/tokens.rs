#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing(f32);

impl Spacing {
    pub const fn px(value: f32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radius(f32);

impl Radius {
    pub const fn px(value: f32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontSize(f32);

impl FontSize {
    pub const fn px(value: f32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpaceScale {
    pub xs: Spacing,
    pub sm: Spacing,
    pub md: Spacing,
    pub lg: Spacing,
    pub xl: Spacing,
}

impl SpaceScale {
    pub fn compact() -> Self {
        Self {
            xs: Spacing::px(2.0),
            sm: Spacing::px(4.0),
            md: Spacing::px(8.0),
            lg: Spacing::px(12.0),
            xl: Spacing::px(16.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadiusScale {
    pub sm: Radius,
    pub md: Radius,
    pub lg: Radius,
}

impl RadiusScale {
    pub fn rounded() -> Self {
        Self {
            sm: Radius::px(4.0),
            md: Radius::px(8.0),
            lg: Radius::px(12.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypographyTokens {
    pub body: FontSize,
    pub heading: FontSize,
    pub mono: FontSize,
}

impl TypographyTokens {
    pub fn desktop_defaults() -> Self {
        Self {
            body: FontSize::px(14.0),
            heading: FontSize::px(18.0),
            mono: FontSize::px(13.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_values_roundtrip() {
        let spacing = Spacing::px(9.0);
        assert_eq!(spacing.value(), 9.0);

        let radius = Radius::px(6.0);
        assert_eq!(radius.value(), 6.0);

        let font_size = FontSize::px(15.0);
        assert_eq!(font_size.value(), 15.0);
    }
}
