use std::time::Duration;

use slotmap::SlotMap;
use velox_runtime::{PowerClass, PowerPolicy};

use crate::animation::{AnimationId, AnimationState};
use crate::interpolate::Interpolatable;
use crate::spring::{Spring, SpringValue};
use crate::tween::Tween;

trait ManagedAnimation: Send {
    fn tick(&mut self, dt: Duration) -> bool;
    fn power_class(&self) -> PowerClass;
    fn state(&self) -> AnimationState;
    fn cancel(&mut self);
}

struct ManagedTween<T: Interpolatable> {
    tween: Tween<T>,
    class: PowerClass,
    state: AnimationState,
    on_update: Box<dyn Fn(T) + Send>,
}

impl<T: Interpolatable> ManagedAnimation for ManagedTween<T> {
    fn tick(&mut self, dt: Duration) -> bool {
        if self.state != AnimationState::Running {
            return false;
        }

        let value = self.tween.advance(dt);
        (self.on_update)(value);

        if self.tween.is_finished() {
            self.state = AnimationState::Finished;
            return false;
        }
        true
    }

    fn power_class(&self) -> PowerClass {
        self.class
    }

    fn state(&self) -> AnimationState {
        self.state
    }

    fn cancel(&mut self) {
        self.state = AnimationState::Cancelled;
    }
}

struct ManagedSpring<T: SpringValue> {
    spring: Spring<T>,
    class: PowerClass,
    state: AnimationState,
    on_update: Box<dyn Fn(T) + Send>,
}

impl<T: SpringValue> ManagedAnimation for ManagedSpring<T> {
    fn tick(&mut self, dt: Duration) -> bool {
        if self.state != AnimationState::Running {
            return false;
        }

        let value = self.spring.advance(dt);
        (self.on_update)(value);

        if self.spring.is_at_rest() {
            self.state = AnimationState::Finished;
            return false;
        }
        true
    }

    fn power_class(&self) -> PowerClass {
        self.class
    }

    fn state(&self) -> AnimationState {
        self.state
    }

    fn cancel(&mut self) {
        self.state = AnimationState::Cancelled;
    }
}

pub struct AnimationManager {
    animations: SlotMap<AnimationId, Box<dyn ManagedAnimation>>,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            animations: SlotMap::with_key(),
        }
    }

    pub fn register_tween<T: Interpolatable>(
        &mut self,
        tween: Tween<T>,
        class: PowerClass,
        on_update: impl Fn(T) + Send + 'static,
    ) -> AnimationId {
        let managed = ManagedTween {
            tween,
            class,
            state: AnimationState::Running,
            on_update: Box::new(on_update),
        };
        self.animations.insert(Box::new(managed))
    }

    pub fn register_spring<T: SpringValue>(
        &mut self,
        spring: Spring<T>,
        class: PowerClass,
        on_update: impl Fn(T) + Send + 'static,
    ) -> AnimationId {
        let managed = ManagedSpring {
            spring,
            class,
            state: AnimationState::Running,
            on_update: Box::new(on_update),
        };
        self.animations.insert(Box::new(managed))
    }

    pub fn cancel(&mut self, id: AnimationId) {
        if let Some(anim) = self.animations.get_mut(id) {
            anim.cancel();
        }
    }

    pub fn tick(&mut self, dt: Duration, policy: PowerPolicy) {
        let mut to_remove = Vec::new();

        for (id, anim) in &mut self.animations {
            if !policy.should_run(anim.power_class()) {
                continue;
            }

            if !anim.tick(dt) {
                to_remove.push(id);
            }
        }

        for id in to_remove {
            self.animations.remove(id);
        }
    }

    pub fn has_running(&self) -> bool {
        self.animations
            .values()
            .any(|a| a.state() == AnimationState::Running)
    }
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::easing::Easing;

    #[test]
    fn tick_advances_animations() {
        let mut mgr = AnimationManager::new();
        let values = Arc::new(Mutex::new(Vec::new()));
        let values_clone = values.clone();

        let tween = Tween::new(0.0_f32, 100.0, Duration::from_secs(1), Easing::Linear);
        mgr.register_tween(tween, PowerClass::Essential, move |v| {
            values_clone.lock().unwrap().push(v);
        });

        mgr.tick(Duration::from_millis(500), PowerPolicy::Adaptive);
        let captured = values.lock().unwrap();
        assert!(!captured.is_empty());
        assert!((captured[0] - 50.0).abs() < 1.0);
    }

    #[test]
    fn power_policy_suppresses_decorative() {
        let mut mgr = AnimationManager::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        let tween = Tween::new(0.0_f32, 100.0, Duration::from_secs(1), Easing::Linear);
        mgr.register_tween(tween, PowerClass::Decorative, move |_| {
            *called_clone.lock().unwrap() = true;
        });

        mgr.tick(Duration::from_millis(100), PowerPolicy::Saving);
        assert!(!*called.lock().unwrap());
    }

    #[test]
    fn cancel_removes_animation() {
        let mut mgr = AnimationManager::new();
        let tween = Tween::new(0.0_f32, 100.0, Duration::from_secs(1), Easing::Linear);
        let id = mgr.register_tween(tween, PowerClass::Essential, |_| {});

        assert!(mgr.has_running());
        mgr.cancel(id);
        mgr.tick(Duration::from_millis(16), PowerPolicy::Adaptive);
        assert!(!mgr.has_running());
    }

    #[test]
    fn has_running_false_when_empty() {
        let mgr = AnimationManager::new();
        assert!(!mgr.has_running());
    }
}
