 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use egui::{Context, Id, Pos2, Vec2};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

 
 
const MAX_DT: f32 = 1.0 / 15.0;

 
const SUBSTEPS: u32 = 4;

 
const STALE_AFTER: f64 = 3.0;

 
pub const SPEED_MIN: f32 = 0.25;
pub const SPEED_MAX: f32 = 3.0;

 
 
static SPEED_BITS: AtomicU32 = AtomicU32::new(0x3f80_0000);

 

 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Easing {
    Linear,
    In,
    Out,
    InOut,
     
    OutBack,
}

impl Easing {
     
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::In => t * t * t,
            Easing::Out => 1.0 - (1.0 - t).powi(3),
            Easing::OutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
            }
            Easing::InOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
        }
    }
}

 

 
#[derive(Clone, Copy, Debug)]
pub struct SpringCfg {
    pub stiffness: f32,
    pub damping: f32,
}

impl SpringCfg {
     
    pub const PANEL: SpringCfg = SpringCfg {
        stiffness: 320.0,
        damping: 24.0,
    };
     
    pub const SNAPPY: SpringCfg = SpringCfg {
        stiffness: 520.0,
        damping: 32.0,
    };
}

 

struct Tween {
    from: f32,
    target: f32,
    duration: f32,
    easing: Easing,
    start: f64,
    last_seen: f64,
}

struct SpringState {
    value: Vec2,
    velocity: Vec2,
    last_seen: f64,
}

#[derive(Default)]
struct Store {
    tweens: HashMap<Id, Tween>,
    springs: HashMap<Id, SpringState>,
}

fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Store::default()))
}

 
pub fn speed() -> f32 {
    f32::from_bits(SPEED_BITS.load(Ordering::Relaxed))
}

 
 
pub fn set_speed(speed: f32) {
    let speed = speed.clamp(SPEED_MIN, SPEED_MAX);
    SPEED_BITS.store(speed.to_bits(), Ordering::Relaxed);
}

 
fn clock(ctx: &Context) -> (f64, f32) {
    ctx.input(|i| (i.time, i.stable_dt.clamp(0.0, MAX_DT)))
}

 

 
 
 
 
pub fn ease_to(ctx: &Context, id: Id, target: f32, duration: f32, easing: Easing) -> f32 {
    let (now, _) = clock(ctx);
    let mut store = store().lock().unwrap();
    let tween = store.tweens.entry(id).or_insert(Tween {
        from: target,
        target,
        duration,
        easing,
        start: now,
        last_seen: now,
    });
    tween.last_seen = now;

    if (tween.target - target).abs() > f32::EPSILON {
        tween.from = sample(tween, now);
        tween.target = target;
        tween.duration = duration;
        tween.easing = easing;
        tween.start = now;
    }
    sample(tween, now)
}

 
pub fn toggle(ctx: &Context, id: Id, on: bool, duration: f32, easing: Easing) -> f32 {
    ease_to(ctx, id, if on { 1.0 } else { 0.0 }, duration, easing)
}

fn sample(t: &Tween, now: f64) -> f32 {
    if t.duration <= 0.0 {
        return t.target;
    }
    let progress = ((now - t.start) as f32 * speed() / t.duration).clamp(0.0, 1.0);
    t.from + (t.target - t.from) * t.easing.apply(progress)
}

 

 
pub fn spring(ctx: &Context, id: Id, target: f32, cfg: SpringCfg) -> f32 {
    spring_vec2(ctx, id, Vec2::new(target, 0.0), cfg).x
}

 
pub fn spring_pos(ctx: &Context, id: Id, target: Pos2, cfg: SpringCfg) -> Pos2 {
    spring_vec2(ctx, id, target.to_vec2(), cfg).to_pos2()
}

 
pub fn spring_vec2(ctx: &Context, id: Id, target: Vec2, cfg: SpringCfg) -> Vec2 {
    let (now, dt) = clock(ctx);
    let mut store = store().lock().unwrap();
    let spring = store.springs.entry(id).or_insert(SpringState {
        value: target,
        velocity: Vec2::ZERO,
        last_seen: now,
    });
    spring.last_seen = now;

    let sub_dt = dt * speed() / SUBSTEPS as f32;
    for _ in 0..SUBSTEPS {
        let displacement = spring.value - target;
        let accel = -cfg.stiffness * displacement - cfg.damping * spring.velocity;
        spring.velocity += accel * sub_dt;
        spring.value += spring.velocity * sub_dt;
    }
    spring.value
}

 

 
 
pub fn gc(ctx: &Context) {
    let now = ctx.input(|i| i.time);
    let mut store = store().lock().unwrap();
    store.tweens.retain(|_, t| now - t.last_seen < STALE_AFTER);
    store.springs.retain(|_, s| now - s.last_seen < STALE_AFTER);
}
