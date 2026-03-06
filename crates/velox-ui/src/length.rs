#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Px(f32),
    Pct(f32),
    Fr(f32),
    Auto,
}

impl Length {
    pub const ZERO: Self = Self::Px(0.0);
}

pub fn px(val: f32) -> Length {
    Length::Px(val)
}

pub fn pct(val: f32) -> Length {
    Length::Pct(val)
}

pub fn fr(val: f32) -> Length {
    Length::Fr(val)
}

pub fn auto() -> Length {
    Length::Auto
}

impl From<f32> for Length {
    fn from(val: f32) -> Self {
        Length::Px(val)
    }
}

impl From<i32> for Length {
    fn from(val: i32) -> Self {
        Length::Px(val as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_constructor() {
        assert_eq!(px(16.0), Length::Px(16.0));
    }

    #[test]
    fn pct_constructor() {
        assert_eq!(pct(50.0), Length::Pct(50.0));
    }

    #[test]
    fn fr_constructor() {
        assert_eq!(fr(1.0), Length::Fr(1.0));
    }

    #[test]
    fn auto_constructor() {
        assert_eq!(auto(), Length::Auto);
    }

    #[test]
    fn zero_constant() {
        assert_eq!(Length::ZERO, Length::Px(0.0));
    }

    #[test]
    fn from_f32() {
        let len: Length = 10.0_f32.into();
        assert_eq!(len, Length::Px(10.0));
    }

    #[test]
    fn from_i32() {
        let len: Length = 10_i32.into();
        assert_eq!(len, Length::Px(10.0));
    }
}
