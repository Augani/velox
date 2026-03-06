#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThemeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ThemeColor {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    pub const fn as_rgba_u8(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_has_full_alpha() {
        let color = ThemeColor::rgb(1, 2, 3);
        assert_eq!(color.a, 255);
    }

    #[test]
    fn with_alpha_overrides_alpha() {
        let color = ThemeColor::rgb(12, 34, 56).with_alpha(99);
        assert_eq!(color.as_rgba_u8(), [12, 34, 56, 99]);
    }
}
