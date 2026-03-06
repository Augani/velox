#[derive(Clone)]
pub enum Easing {
    Linear,
    InQuad,
    OutQuad,
    InOutQuad,
    InCubic,
    OutCubic,
    InOutCubic,
    InExpo,
    OutExpo,
    InOutExpo,
    Custom(fn(f32) -> f32),
}

impl Easing {
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::InQuad => t * t,
            Easing::OutQuad => t * (2.0 - t),
            Easing::InOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Easing::InCubic => t * t * t,
            Easing::OutCubic => {
                let t1 = t - 1.0;
                t1 * t1 * t1 + 1.0
            }
            Easing::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let t1 = 2.0 * t - 2.0;
                    0.5 * t1 * t1 * t1 + 1.0
                }
            }
            Easing::InExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    (2.0_f32).powf(10.0 * (t - 1.0))
                }
            }
            Easing::OutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - (2.0_f32).powf(-10.0 * t)
                }
            }
            Easing::InOutExpo => {
                if t == 0.0 {
                    return 0.0;
                }
                if t == 1.0 {
                    return 1.0;
                }
                if t < 0.5 {
                    (2.0_f32).powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - (2.0_f32).powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            Easing::Custom(f) => f(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_returns_t() {
        assert_eq!(Easing::Linear.apply(0.0), 0.0);
        assert_eq!(Easing::Linear.apply(0.5), 0.5);
        assert_eq!(Easing::Linear.apply(1.0), 1.0);
    }

    #[test]
    fn boundary_values() {
        let easings = [
            Easing::InQuad,
            Easing::OutQuad,
            Easing::InOutQuad,
            Easing::InCubic,
            Easing::OutCubic,
            Easing::InOutCubic,
            Easing::InExpo,
            Easing::OutExpo,
            Easing::InOutExpo,
        ];
        for easing in &easings {
            let at_zero = easing.apply(0.0);
            let at_one = easing.apply(1.0);
            assert!((at_zero - 0.0).abs() < 1e-6, "easing at 0.0 was {at_zero}");
            assert!((at_one - 1.0).abs() < 1e-6, "easing at 1.0 was {at_one}");
        }
    }

    #[test]
    fn in_quad_at_half() {
        let result = Easing::InQuad.apply(0.5);
        assert!((result - 0.25).abs() < 1e-6);
    }

    #[test]
    fn clamps_input() {
        assert_eq!(Easing::Linear.apply(-0.5), 0.0);
        assert_eq!(Easing::Linear.apply(1.5), 1.0);
    }
}
