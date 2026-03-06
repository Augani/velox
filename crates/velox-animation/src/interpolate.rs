use velox_scene::{Color, Point, Rect, Size};

pub trait Interpolatable: Clone + Send + 'static {
    fn lerp(&self, target: &Self, t: f32) -> Self;
}

impl Interpolatable for f32 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t
    }
}

impl Interpolatable for f64 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t as f64
    }
}

impl Interpolatable for Point {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Point {
            x: self.x + (target.x - self.x) * t,
            y: self.y + (target.y - self.y) * t,
        }
    }
}

impl Interpolatable for Rect {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Rect {
            x: self.x + (target.x - self.x) * t,
            y: self.y + (target.y - self.y) * t,
            width: self.width + (target.width - self.width) * t,
            height: self.height + (target.height - self.height) * t,
        }
    }
}

impl Interpolatable for Size {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Size {
            width: self.width + (target.width - self.width) * t,
            height: self.height + (target.height - self.height) * t,
        }
    }
}

impl Interpolatable for Color {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Color {
            r: lerp_u8(self.r, target.r, t),
            g: lerp_u8(self.g, target.g, t),
            b: lerp_u8(self.b, target.b, t),
            a: lerp_u8(self.a, target.a, t),
        }
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let result = a as f32 + (b as f32 - a as f32) * t;
    result.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_lerp() {
        assert!((0.0_f32.lerp(&10.0, 0.5) - 5.0).abs() < 1e-6);
        assert!((0.0_f32.lerp(&10.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((0.0_f32.lerp(&10.0, 1.0) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn point_lerp() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 20.0);
        let mid = a.lerp(&b, 0.5);
        assert!((mid.x - 5.0).abs() < 1e-6);
        assert!((mid.y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn color_lerp() {
        let a = Color::rgba(0, 0, 0, 255);
        let b = Color::rgba(200, 100, 50, 0);
        let mid = a.lerp(&b, 0.5);
        assert_eq!(mid.r, 100);
        assert_eq!(mid.g, 50);
        assert_eq!(mid.b, 25);
        assert_eq!(mid.a, 128);
    }

    #[test]
    fn size_lerp() {
        let a = Size::new(100.0, 200.0);
        let b = Size::new(200.0, 400.0);
        let mid = a.lerp(&b, 0.5);
        assert!((mid.width - 150.0).abs() < 1e-6);
        assert!((mid.height - 300.0).abs() < 1e-6);
    }
}
