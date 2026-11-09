 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use std::sync::Mutex;
use std::time::Instant;

 
 
const NOMINAL_DT: f32 = 1.0 / 60.0;
const MAX_DT: f32 = 0.1;
 
 
const MAX_SPEED: f32 = 180.0;

 
 
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Aim {
    pub yaw: f32,
    pub pitch: f32,
}

impl Aim {
    pub(crate) fn new(yaw: f32, pitch: f32) -> Aim {
        Aim { yaw, pitch }
    }

    fn finite(self) -> Option<Aim> {
        (self.yaw.is_finite() && self.pitch.is_finite()).then_some(self)
    }

     
    fn lerp_towards(self, target: Aim, alpha: f32) -> Aim {
        let alpha = alpha.clamp(0.0, 1.0);
        let mut dyaw = (target.yaw - self.yaw) % 360.0;
        if dyaw > 180.0 {
            dyaw -= 360.0;
        } else if dyaw < -180.0 {
            dyaw += 360.0;
        }
        Aim {
            yaw: self.yaw + dyaw * alpha,
            pitch: self.pitch + (target.pitch - self.pitch) * alpha,
        }
    }
}

#[derive(Default)]
struct State {
    target: Option<Aim>,
    current: Option<Aim>,
    last_frame: Option<Instant>,
}

static STATE: Mutex<State> = Mutex::new(State {
    target: None,
    current: None,
    last_frame: None,
});

fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
 
 
 
 
pub(crate) fn seed(yaw: f32, pitch: f32) {
    let Some(view) = Aim::new(yaw, pitch).finite() else {
        return;
    };
    let mut state = lock();
    if state.current.is_none() {
        state.current = Some(view);
    }
}

 
 
pub(crate) fn request(target: Aim, speed: f32) {
    let Some(target) = target.finite() else {
        return;
    };
    let speed = if speed.is_finite() {
        speed.clamp(1.0, MAX_SPEED)
    } else {
        10.0
    };
    let mut state = lock();
    let dt = state
        .last_frame
        .map_or(NOMINAL_DT, |last| last.elapsed().as_secs_f32().min(MAX_DT));
    let from = state.current.unwrap_or(target);
    let alpha = 1.0 - (-speed * dt).exp();
     
     
    state.current = Some(from.lerp_towards(target, alpha).finite().unwrap_or(target));
    state.target = Some(target);
}

 
 
 
pub(crate) fn step(speed: f32) {
    let speed = if speed.is_finite() {
        speed.clamp(1.0, MAX_SPEED)
    } else {
        10.0
    };
    let mut state = lock();
    let now = Instant::now();
    let dt = state.last_frame.map_or(NOMINAL_DT, |last| {
        now.duration_since(last).as_secs_f32().min(MAX_DT)
    });
    state.last_frame = Some(now);
    if let (Some(target), Some(current)) = (state.target, state.current) {
        let alpha = 1.0 - (-speed * dt).exp();
        state.current = Some(current.lerp_towards(target, alpha));
    }
}

 
pub(crate) fn current() -> Option<Aim> {
    let state = lock();
    state.target.and(state.current)
}

 
pub(crate) fn clear() {
    let mut state = lock();
    state.target = None;
    state.current = None;
    state.last_frame = None;
}

#[cfg(test)]
mod tests {
    use super::*;

     
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        static TEST: Mutex<()> = Mutex::new(());
        let guard = TEST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        clear();
        guard
    }

    #[test]
    fn a_request_is_published_immediately_and_eases_toward_the_target() {
        let _guard = guard();
         
        seed(0.0, 0.0);
        request(Aim::new(90.0, 40.0), 10.0);
        let first = current().expect("a fresh request is on the wire");
         
         
        assert!(first.yaw > 0.0 && first.yaw < 90.0, "{first:?}");
        assert!(first.pitch > 0.0 && first.pitch < 40.0, "{first:?}");
         
         
         
        let mut previous = first;
        for _ in 0..8 {
            step(10.0);
            let now = current().expect("still held");
            assert!((now.yaw - previous.yaw).abs() < 90.0, "{now:?}");
            assert!(now.yaw >= previous.yaw - 1e-4, "{now:?} {previous:?}");
            assert!(now.yaw <= 90.0, "{now:?}");
            previous = now;
        }
         
        let landed = Aim::new(0.0, 0.0).lerp_towards(Aim::new(90.0, 40.0), 1.0);
        assert_eq!(landed, Aim::new(90.0, 40.0));
    }

    #[test]
    fn the_shortest_way_round_is_taken() {
        let _guard = guard();
        request(Aim::new(179.0, 0.0), 10.0);
        for _ in 0..2 {
            step(45.0);
        }
        request(Aim::new(-179.0, 0.0), 45.0);
        let crossed = current().expect("held").yaw;
         
        assert!(crossed > 175.0, "{crossed}");
    }

     
     
    #[test]
    fn an_unseeded_request_publishes_the_target_angle() {
        let _guard = guard();
        request(Aim::new(40.0, 12.0), 10.0);
        let first = current().expect("a fresh request is on the wire");
        assert_eq!(first, Aim::new(40.0, 12.0));
    }

    #[test]
    fn clearing_releases_the_wire_angle() {
        let _guard = guard();
        request(Aim::new(45.0, 20.0), 10.0);
        assert!(current().is_some());
        clear();
        assert!(current().is_none(), "a released aim stamps nothing");
        step(10.0);
        assert!(current().is_none(), "stepping cannot resurrect it");
    }

    #[test]
    fn corrupt_requests_are_refused_or_clamped() {
        let _guard = guard();
        request(Aim::new(f32::NAN, 10.0), 10.0);
        assert!(current().is_none(), "a NaN angle is never published");
        request(Aim::new(10.0, 10.0), f32::NAN);
        assert!(
            current().is_some(),
            "a corrupt speed falls back to a finite one"
        );
        request(Aim::new(10.0, 10.0), 1.0e9);
        step(1.0e9);
        let settled = current().expect("held");
        assert!(settled.yaw.is_finite() && settled.pitch.is_finite());
    }
}
