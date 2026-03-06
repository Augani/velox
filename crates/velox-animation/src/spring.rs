use std::time::Duration;

use velox_scene::{Color, Point, Rect, Size};

use crate::interpolate::Interpolatable;

#[derive(Clone)]
pub struct SpringConfig {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
    pub rest_threshold: f32,
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self {
            stiffness: 170.0,
            damping: 26.0,
            mass: 1.0,
            rest_threshold: 0.001,
        }
    }
}

pub trait SpringValue: Interpolatable {
    fn spring_distance(&self, other: &Self) -> f32;
    fn spring_advance(
        &self,
        target: &Self,
        velocity: &mut Vec<f32>,
        config: &SpringConfig,
        dt: f32,
    ) -> Self;
    fn initial_velocity_components(&self) -> Vec<f32>;
}

impl SpringValue for f32 {
    fn spring_distance(&self, other: &Self) -> f32 {
        (self - other).abs()
    }

    fn spring_advance(
        &self,
        target: &Self,
        velocity: &mut Vec<f32>,
        config: &SpringConfig,
        dt: f32,
    ) -> Self {
        let (new_pos, new_vel) = step_spring(*self, *target, velocity[0], config, dt);
        velocity[0] = new_vel;
        new_pos
    }

    fn initial_velocity_components(&self) -> Vec<f32> {
        vec![0.0]
    }
}

impl SpringValue for Point {
    fn spring_distance(&self, other: &Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    fn spring_advance(
        &self,
        target: &Self,
        velocity: &mut Vec<f32>,
        config: &SpringConfig,
        dt: f32,
    ) -> Self {
        let (nx, vx) = step_spring(self.x, target.x, velocity[0], config, dt);
        let (ny, vy) = step_spring(self.y, target.y, velocity[1], config, dt);
        velocity[0] = vx;
        velocity[1] = vy;
        Point { x: nx, y: ny }
    }

    fn initial_velocity_components(&self) -> Vec<f32> {
        vec![0.0; 2]
    }
}

impl SpringValue for Rect {
    fn spring_distance(&self, other: &Self) -> f32 {
        ((self.x - other.x).powi(2)
            + (self.y - other.y).powi(2)
            + (self.width - other.width).powi(2)
            + (self.height - other.height).powi(2))
        .sqrt()
    }

    fn spring_advance(
        &self,
        target: &Self,
        velocity: &mut Vec<f32>,
        config: &SpringConfig,
        dt: f32,
    ) -> Self {
        let (nx, vx) = step_spring(self.x, target.x, velocity[0], config, dt);
        let (ny, vy) = step_spring(self.y, target.y, velocity[1], config, dt);
        let (nw, vw) = step_spring(self.width, target.width, velocity[2], config, dt);
        let (nh, vh) = step_spring(self.height, target.height, velocity[3], config, dt);
        velocity[0] = vx;
        velocity[1] = vy;
        velocity[2] = vw;
        velocity[3] = vh;
        Rect {
            x: nx,
            y: ny,
            width: nw,
            height: nh,
        }
    }

    fn initial_velocity_components(&self) -> Vec<f32> {
        vec![0.0; 4]
    }
}

impl SpringValue for Size {
    fn spring_distance(&self, other: &Self) -> f32 {
        ((self.width - other.width).powi(2) + (self.height - other.height).powi(2)).sqrt()
    }

    fn spring_advance(
        &self,
        target: &Self,
        velocity: &mut Vec<f32>,
        config: &SpringConfig,
        dt: f32,
    ) -> Self {
        let (nw, vw) = step_spring(self.width, target.width, velocity[0], config, dt);
        let (nh, vh) = step_spring(self.height, target.height, velocity[1], config, dt);
        velocity[0] = vw;
        velocity[1] = vh;
        Size {
            width: nw,
            height: nh,
        }
    }

    fn initial_velocity_components(&self) -> Vec<f32> {
        vec![0.0; 2]
    }
}

impl SpringValue for Color {
    fn spring_distance(&self, other: &Self) -> f32 {
        ((self.r as f32 - other.r as f32).powi(2)
            + (self.g as f32 - other.g as f32).powi(2)
            + (self.b as f32 - other.b as f32).powi(2)
            + (self.a as f32 - other.a as f32).powi(2))
        .sqrt()
    }

    fn spring_advance(
        &self,
        target: &Self,
        velocity: &mut Vec<f32>,
        config: &SpringConfig,
        dt: f32,
    ) -> Self {
        let (nr, vr) = step_spring(self.r as f32, target.r as f32, velocity[0], config, dt);
        let (ng, vg) = step_spring(self.g as f32, target.g as f32, velocity[1], config, dt);
        let (nb, vb) = step_spring(self.b as f32, target.b as f32, velocity[2], config, dt);
        let (na, va) = step_spring(self.a as f32, target.a as f32, velocity[3], config, dt);
        velocity[0] = vr;
        velocity[1] = vg;
        velocity[2] = vb;
        velocity[3] = va;
        Color {
            r: nr.round().clamp(0.0, 255.0) as u8,
            g: ng.round().clamp(0.0, 255.0) as u8,
            b: nb.round().clamp(0.0, 255.0) as u8,
            a: na.round().clamp(0.0, 255.0) as u8,
        }
    }

    fn initial_velocity_components(&self) -> Vec<f32> {
        vec![0.0; 4]
    }
}

fn step_spring(
    current: f32,
    target: f32,
    velocity: f32,
    config: &SpringConfig,
    dt: f32,
) -> (f32, f32) {
    let displacement = current - target;
    let spring_force = -config.stiffness * displacement;
    let damping_force = -config.damping * velocity;
    let acceleration = (spring_force + damping_force) / config.mass.max(0.001);
    let new_velocity = velocity + acceleration * dt;
    let new_position = current + new_velocity * dt;
    (new_position, new_velocity)
}

pub struct Spring<T: SpringValue> {
    current: T,
    target: T,
    velocity: Vec<f32>,
    config: SpringConfig,
    at_rest: bool,
}

impl<T: SpringValue> Spring<T> {
    pub fn new(initial: T, config: SpringConfig) -> Self {
        let velocity = initial.initial_velocity_components();
        Self {
            target: initial.clone(),
            current: initial,
            velocity,
            config,
            at_rest: true,
        }
    }

    pub fn set_target(&mut self, target: T) {
        self.target = target;
        self.at_rest = false;
    }

    pub fn advance(&mut self, dt: Duration) -> T {
        if self.at_rest {
            return self.current.clone();
        }

        let dt_secs = dt.as_secs_f32();
        if dt_secs <= 0.0 {
            return self.current.clone();
        }

        self.current =
            self.current
                .spring_advance(&self.target, &mut self.velocity, &self.config, dt_secs);

        let distance = self.current.spring_distance(&self.target);
        let max_velocity = self.velocity.iter().fold(0.0_f32, |m, v| m.max(v.abs()));

        if distance < self.config.rest_threshold && max_velocity < self.config.rest_threshold {
            self.current = self.target.clone();
            for v in &mut self.velocity {
                *v = 0.0;
            }
            self.at_rest = true;
        }

        self.current.clone()
    }

    pub fn is_at_rest(&self) -> bool {
        self.at_rest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convergence_to_target() {
        let mut spring = Spring::new(0.0_f32, SpringConfig::default());
        spring.set_target(100.0);

        let dt = Duration::from_millis(16);
        for _ in 0..1000 {
            spring.advance(dt);
            if spring.is_at_rest() {
                break;
            }
        }

        assert!(spring.is_at_rest());
        assert!((spring.advance(dt) - 100.0).abs() < 0.01);
    }

    #[test]
    fn retargeting_mid_flight() {
        let mut spring = Spring::new(0.0_f32, SpringConfig::default());
        spring.set_target(100.0);

        let dt = Duration::from_millis(16);
        for _ in 0..10 {
            spring.advance(dt);
        }
        assert!(!spring.is_at_rest());

        spring.set_target(50.0);
        assert!(!spring.is_at_rest());

        for _ in 0..1000 {
            spring.advance(dt);
            if spring.is_at_rest() {
                break;
            }
        }

        assert!(spring.is_at_rest());
        assert!((spring.advance(dt) - 50.0).abs() < 0.01);
    }

    #[test]
    fn at_rest_detection() {
        let spring = Spring::new(0.0_f32, SpringConfig::default());
        assert!(spring.is_at_rest());
    }
}
