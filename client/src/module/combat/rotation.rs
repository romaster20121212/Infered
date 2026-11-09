 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::platform;
use crate::mapping::client::options::KeyMapping;
use crate::mapping::entity::EntityRef;
use crate::mapping::math::Vec3;
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
 
const TARGET_TIMEOUT: Duration = Duration::from_millis(200);

 
const MAX_FRAME_DT: f32 = 0.1;

 
 
const NOMINAL_FRAME_DT: f32 = 1.0 / 60.0;

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
}

impl Rotation {
    pub const fn new(yaw: f32, pitch: f32) -> Rotation {
        Rotation { yaw, pitch }
    }

     
    pub fn towards(from: Vec3, to: Vec3) -> Rotation {
        let dx = to.x() - from.x();
        let dy = to.y() - from.y();
        let dz = to.z() - from.z();
        let ground = (dx * dx + dz * dz).sqrt();
        Rotation {
            yaw: wrap_degrees((dz.atan2(dx).to_degrees() - 90.0) as f32),
            pitch: (-dy.atan2(ground).to_degrees() as f32).clamp(-90.0, 90.0),
        }
    }

     
    pub fn angle_to(self, other: Rotation) -> f32 {
        let dyaw = wrap_degrees(other.yaw - self.yaw);
        let dpitch = other.pitch - self.pitch;
        (dyaw * dyaw + dpitch * dpitch).sqrt()
    }

     
     
    fn lerp_towards(self, target: Rotation, alpha: f32) -> Rotation {
        let dyaw = wrap_degrees(target.yaw - self.yaw);
        let dpitch = target.pitch - self.pitch;
        Rotation {
            yaw: self.yaw + dyaw * alpha,
            pitch: (self.pitch + dpitch * alpha).clamp(-90.0, 90.0),
        }
    }
}

 
pub fn wrap_degrees(mut angle: f32) -> f32 {
    angle %= 360.0;
    if angle >= 180.0 {
        angle -= 360.0;
    } else if angle < -180.0 {
        angle += 360.0;
    }
    angle
}

 
 
 
 
 
 
 
 
 
pub fn align_yaw(angle: f32, reference: f32) -> f32 {
    reference + wrap_degrees(angle - reference)
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimPath {
     
     
    Write,
     
     
    RealMouse,
     
     
     
     
     
    Silent,
}

 
#[derive(Debug, Clone, Copy)]
struct Target {
    rotation: Rotation,
     
    speed: f32,
     
    refreshed: Instant,
     
    path: AimPath,
}

 
struct RotationController {
    target: Option<Target>,
    last_frame: Option<Instant>,
     
     
     
     
    current: Option<Rotation>,
}

impl RotationController {
    const fn new() -> RotationController {
        RotationController {
            target: None,
            last_frame: None,
            current: None,
        }
    }
}

static CONTROLLER: Mutex<RotationController> = Mutex::new(RotationController::new());

 
 
 
 
static SILENT: Mutex<Option<Rotation>> = Mutex::new(None);

 
 
 
pub fn aim(rotation: Rotation, speed: f32) {
    set_target(rotation, speed, AimPath::Write);
}

 
 
 
 
 
 
pub fn aim_real_mouse(rotation: Rotation, speed: f32) {
    set_target(rotation, speed, AimPath::RealMouse);
}

 
 
 
 
pub fn aim_silent(rotation: Rotation, speed: f32) {
    set_target(rotation, speed, AimPath::Silent);
}

fn set_target(rotation: Rotation, speed: f32, path: AimPath) {
    let mut published = None;
    if let Ok(mut controller) = CONTROLLER.lock() {
         
         
         
         
         
         
        let dt = controller.last_frame.map_or(NOMINAL_FRAME_DT, |last| {
            last.elapsed().as_secs_f32().min(MAX_FRAME_DT)
        });
        let from = controller.current.unwrap_or(rotation);
        let alpha = 1.0 - (-speed * dt).exp();
        let step = from.lerp_towards(rotation, alpha);
        controller.current = Some(step);
        controller.target = Some(Target {
            rotation,
            speed,
            refreshed: Instant::now(),
            path,
        });
        if path == AimPath::Silent {
            published = Some(step);
        }
    }
     
     
     
     
    if let Some(step) = published {
        if let Ok(mut shadow) = SILENT.lock() {
            *shadow = Some(step);
        }
    }
}

 
 
pub fn clear() {
    if let Ok(mut controller) = CONTROLLER.lock() {
        controller.target = None;
    }
    if let Ok(mut shadow) = SILENT.lock() {
        *shadow = None;
    }
}

 
 
pub fn clear_silent() {
    if let Ok(mut controller) = CONTROLLER.lock() {
        if controller
            .target
            .is_some_and(|target| target.path == AimPath::Silent)
        {
            controller.target = None;
        }
    }
    if let Ok(mut shadow) = SILENT.lock() {
        *shadow = None;
    }
}

 
 
fn silent_target_active() -> bool {
    CONTROLLER
        .lock()
        .map(|controller| {
            controller.target.is_some_and(|target| {
                target.path == AimPath::Silent && target.refreshed.elapsed() <= TARGET_TIMEOUT
            })
        })
        .unwrap_or(false)
}

 
 
 
 
 
 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyAimOwner {
     
    SilentAura,
     
    SilentMace,
     
    SilentAssist,
     
    Scaffold,
     
    FastScaffold,
     
    Clutch,
}

 
 
 
 
 
const BODY_AIM_TIMEOUT: Duration = Duration::from_millis(500);

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
#[derive(Debug, Default)]
struct BodyAim {
     
    owner: Option<BodyAimOwner>,
     
     
    view: Option<Rotation>,
     
     
     
     
    aim: Option<Rotation>,
     
     
    held: MoveKeys,
     
     
    applied: MoveKeys,
     
    armed_at: Option<Instant>,
}

impl BodyAim {
     
     
    fn pending(&self) -> Option<(Rotation, Option<Rotation>, MoveKeys, MoveKeys)> {
        self.view
            .map(|view| (view, self.aim, self.held, self.applied))
    }

     
    fn owner(&self) -> Option<BodyAimOwner> {
        self.view.and(self.owner)
    }

     
     
     
    fn stale(&self, now: Instant) -> bool {
        self.view.is_some()
            && self
                .armed_at
                .is_some_and(|armed| now.duration_since(armed) > BODY_AIM_TIMEOUT)
    }

     
     
     
    fn begin(&mut self, owner: BodyAimOwner, view: Rotation, held: MoveKeys, now: Instant) {
        self.owner = Some(owner);
        self.view = Some(view);
        self.aim = None;
        self.held = held;
        self.applied = held;
        self.armed_at = Some(now);
    }

     
    fn set_applied(&mut self, applied: MoveKeys) {
        self.applied = applied;
    }

     
    fn set_aim(&mut self, aim: Rotation) {
        self.aim = Some(aim);
    }

     
     
    fn clear(&mut self) {
        *self = BodyAim::default();
    }

     
     
    fn take(&mut self) -> Option<(Rotation, Option<Rotation>, MoveKeys, MoveKeys)> {
        let taken = self.pending();
        self.clear();
        taken
    }
}

 
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MoveKeys {
    forward: bool,
    back: bool,
    left: bool,
    right: bool,
}

impl MoveKeys {
     
     
     
     
     
     
    fn angle(self) -> Option<f32> {
        let forward = self.forward as i32 - self.back as i32;
        let strafe = self.left as i32 - self.right as i32;
        if forward == 0 && strafe == 0 {
            return None;
        }
        Some(-(strafe as f32).atan2(forward as f32).to_degrees())
    }
}

 
const fn keys(forward: bool, back: bool, left: bool, right: bool) -> MoveKeys {
    MoveKeys {
        forward,
        back,
        left,
        right,
    }
}

 
 
 
const INPUT_COMBOS: [(f32, MoveKeys); 8] = [
    (0.0, keys(true, false, false, false)),
    (45.0, keys(true, false, false, true)),
    (90.0, keys(false, false, false, true)),
    (135.0, keys(false, true, false, true)),
    (180.0, keys(false, true, false, false)),
    (-135.0, keys(false, true, true, false)),
    (-90.0, keys(false, false, true, false)),
    (-45.0, keys(true, false, true, false)),
];

 
 
 
 
 
 
 
 
 
 
 
fn movement_fix(held: MoveKeys, view_yaw: f32, aim_yaw: f32) -> Option<MoveKeys> {
    let held_angle = held.angle()?;
    let turn = wrap_degrees(aim_yaw - view_yaw);
    let wanted = held_angle - turn;
    let mut best = INPUT_COMBOS[0];
    let mut best_error = f32::INFINITY;
    for combo in INPUT_COMBOS {
        let error = wrap_degrees(combo.0 - wanted).abs();
        if error < best_error {
            best_error = error;
            best = combo;
        }
    }
    Some(best.1)
}

 
#[derive(Debug)]
struct Movement {
    forward: KeyMapping,
    back: KeyMapping,
    left: KeyMapping,
    right: KeyMapping,
}

impl Movement {
    fn read() -> anyhow::Result<Movement> {
        let options = minecraft().options()?;
        Ok(Movement {
            forward: options.key_up()?,
            back: options.key_back()?,
            left: options.key_left()?,
            right: options.key_right()?,
        })
    }

     
     
    fn held(&self) -> anyhow::Result<MoveKeys> {
        Ok(MoveKeys {
            forward: self.forward.is_down()?,
            back: self.back.is_down()?,
            left: self.left.is_down()?,
            right: self.right.is_down()?,
        })
    }

     
     
     
     
     
    fn physical_or(&self, fallback: MoveKeys) -> MoveKeys {
        MoveKeys {
            forward: physical_down(&self.forward).unwrap_or(fallback.forward),
            back: physical_down(&self.back).unwrap_or(fallback.back),
            left: physical_down(&self.left).unwrap_or(fallback.left),
            right: physical_down(&self.right).unwrap_or(fallback.right),
        }
    }

    fn write(&self, keys: MoveKeys) -> anyhow::Result<()> {
        self.forward.set_down(keys.forward)?;
        self.back.set_down(keys.back)?;
        self.left.set_down(keys.left)?;
        self.right.set_down(keys.right)?;
        Ok(())
    }
}

 
 
fn physical_down(mapping: &KeyMapping) -> Option<bool> {
    if !mapping.key_name().ok()?.starts_with("key.keyboard.") {
        return None;
    }
    Some(crate::graphic::input::key_down(mapping.key_value().ok()?))
}

 
 
 
 
 
 
 
 
 
 
fn hold_movement(
    held: MoveKeys,
    applied: MoveKeys,
    view: Rotation,
    aim: Rotation,
) -> anyhow::Result<MoveKeys> {
    let Some(wanted) = movement_fix(held, view.yaw, aim.yaw) else {
        return Ok(applied);
    };
    if wanted == applied {
        return Ok(applied);
    }
    Movement::read()?.write(wanted)?;
    Ok(wanted)
}

 
 
 
 
 
 
 
fn restore_movement(held: MoveKeys) {
    let Ok(movement) = Movement::read() else {
        return;
    };
    let _ = movement.write(movement.physical_or(held));
}

 
static BODY_AIM: Mutex<BodyAim> = Mutex::new(BodyAim {
    owner: None,
    view: None,
    aim: None,
    held: keys(false, false, false, false),
    applied: keys(false, false, false, false),
    armed_at: None,
});

 
 
fn body_aim() -> std::sync::MutexGuard<'static, BodyAim> {
    BODY_AIM
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
 
 
pub fn aim_body(aim: Rotation) -> anyhow::Result<()> {
    aim_body_for(BodyAimOwner::SilentAura, aim)
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
pub fn aim_body_for(owner: BodyAimOwner, aim: Rotation) -> anyhow::Result<()> {
    let Some(minecraft) = crate::state::try_minecraft() else {
        anyhow::bail!("no game state to turn the body");
    };
    let Some(player) = minecraft.player()? else {
        anyhow::bail!("no player to turn");
    };
    let mut view = Rotation::new(player.get_yaw()?, player.get_pitch()?);
    let now = Instant::now();

     
     
     
     
     
    let stale = body_aim().stale(now);
    if stale {
        let _ = restore_body();
         
         
        view = Rotation::new(player.get_yaw()?, player.get_pitch()?);
    }

     
     
     
    let pending_owner = body_aim().owner();
    anyhow::ensure!(
        pending_owner.is_none() || pending_owner == Some(owner),
        "another module turned the body for this frame"
    );
    let pending = body_aim().pending();
    let (frame_view, held, applied, armed) = match pending {
        Some((frame_view, previous_aim, held, applied)) => {
             
             
            let frame_view = previous_aim
                .map(|previous| restored_view(frame_view, previous, view))
                .unwrap_or(frame_view);
            let held = Movement::read()?.physical_or(held);
            {
                let mut body = body_aim();
                body.view = Some(frame_view);
                body.aim = Some(view);
                body.held = held;
            }
            (frame_view, held, applied, false)
        }
        None => {
            let movement = Movement::read()?;
            let held = movement.held()?;
            body_aim().begin(owner, view, held, now);
            (view, held, held, true)
        }
    };

     
     
     
    let aim = Rotation::new(align_yaw(aim.yaw, frame_view.yaw), aim.pitch);

     
     
     
     
    match hold_movement(held, applied, frame_view, aim) {
        Ok(applied) => body_aim().set_applied(applied),
        Err(error) => {
            if armed {
                body_aim().clear();
                restore_movement(held);
            }
            return Err(error);
        }
    }
    if let Err(error) = player.set_rotation(aim.yaw, aim.pitch) {
         
         
         
        body_aim().clear();
        restore_movement(held);
        let _ = player.set_rotation(frame_view.yaw, frame_view.pitch);
        return Err(error);
    }
    body_aim().set_aim(aim);
    Ok(())
}

 
 
pub fn restore_body_for(owner: BodyAimOwner) -> Option<Rotation> {
    if body_aim().owner() != Some(owner) {
        return None;
    }
    restore_body()
}

 
 
 
 
 
 
 
 
fn restored_view(view: Rotation, aim: Rotation, current: Rotation) -> Rotation {
    Rotation::new(
        current.yaw + wrap_degrees(view.yaw - aim.yaw),
        (current.pitch + (view.pitch - aim.pitch)).clamp(-90.0, 90.0),
    )
}

 
 
 
pub fn restore_body() -> Option<Rotation> {
    let (view, aim, held, applied) = body_aim().take()?;
     
    if applied != held {
        restore_movement(held);
    }
    let Some(minecraft) = crate::state::try_minecraft() else {
        return Some(view);
    };
    let Ok(Some(player)) = minecraft.player() else {
        return Some(view);
    };
    let restored = match (aim, player.get_yaw(), player.get_pitch()) {
        (Some(aim), Ok(yaw), Ok(pitch)) => restored_view(view, aim, Rotation::new(yaw, pitch)),
        _ => view,
    };
    let _ = player.set_rotation(restored.yaw, restored.pitch);
    Some(restored)
}

 
 
 
 
 
 
pub fn silent_rotation() -> Option<Rotation> {
    if !silent_target_active() {
        return None;
    }
    SILENT.lock().ok().and_then(|shadow| *shadow)
}

pub fn is_active() -> bool {
    CONTROLLER
        .lock()
        .map(|controller| {
            controller
                .target
                .is_some_and(|target| target.refreshed.elapsed() <= TARGET_TIMEOUT)
        })
        .unwrap_or(false)
}

 
 
 
 
 
 
 
 
 
pub fn current_rotation() -> Option<Rotation> {
    CONTROLLER
        .lock()
        .ok()
        .and_then(|controller| match controller.target {
            Some(target) if target.refreshed.elapsed() <= TARGET_TIMEOUT => {
                Some(controller.current.unwrap_or(target.rotation))
            }
            _ => None,
        })
}

 
 
pub fn update() {
     
     
     
     
    if crate::module::misc::freelook::owns_mouse()
        && !crate::graphic::input::player_action_active()
        && !silent_target_active()
    {
        clear();
        return;
    }
    let step = {
        let Ok(mut controller) = CONTROLLER.lock() else {
            return;
        };
        let now = Instant::now();
        let dt = controller
            .last_frame
            .map_or(0.0, |last| (now - last).as_secs_f32())
            .min(MAX_FRAME_DT);
        controller.last_frame = Some(now);

        match controller.target {
            Some(target) if target.refreshed.elapsed() <= TARGET_TIMEOUT => Some((target, dt)),
            _ => {
                controller.target = None;
                None
            }
        }
    };

    let Some((target, dt)) = step else {
        return;
    };
    if dt <= 0.0 {
        return;
    }
     
    let _ = ease_camera(target, dt);
}

 
 
fn ease_camera(target: Target, dt: f32) -> anyhow::Result<()> {
    let Some(player) = minecraft().player()? else {
        return Ok(());
    };
     
    let alpha = 1.0 - (-target.speed * dt).exp();
    if target.path == AimPath::Silent {
         
         
         
        let next = {
            let mut shadow = SILENT.lock().unwrap_or_else(|p| p.into_inner());
            let from = shadow.unwrap_or(target.rotation);
            let next = from.lerp_towards(target.rotation, alpha);
             
             
             
             
            let next = Rotation::new(wrap_degrees(next.yaw), next.pitch);
            *shadow = Some(next);
            next
        };
        if let Ok(mut controller) = CONTROLLER.lock() {
            controller.current = Some(next);
        }
        return Ok(());
    }
    let current = Rotation::new(player.get_yaw()?, player.get_pitch()?);
    let next = current.lerp_towards(target.rotation, alpha);

     
     
     
     
    if let Ok(mut controller) = CONTROLLER.lock() {
        controller.current = Some(next);
    }

    if target.path == AimPath::RealMouse {
         
         
        if crate::module::misc::freelook::owns_mouse() {
            return Ok(());
        }
         
         
        return inject_mouse_step(current, next);
    }
    player.set_rotation(next.yaw, next.pitch)
}

 
 
 
 
 
 
 
 
fn inject_mouse_step(current: Rotation, next: Rotation) -> anyhow::Result<()> {
    let dyaw = wrap_degrees(next.yaw - current.yaw) as f64;
    let dpitch = (next.pitch - current.pitch) as f64;
    let deg_per_px = degrees_per_pixel(minecraft().options()?.sensitivity()?.get_double()?);
    if deg_per_px <= 0.0 {
        return Ok(());
    }
    let dx = (dyaw / deg_per_px).round() as i32;
    let dy = (dpitch / deg_per_px).round() as i32;
    if dx == 0 && dy == 0 {
        return Ok(());
    }
     
    platform::move_mouse_by(dx, dy);
    Ok(())
}

 
 
fn degrees_per_pixel(sensitivity: f64) -> f64 {
    let factor = sensitivity * 0.6 + 0.2;
    factor * factor * factor * 8.0 * 0.15
}

 
 
 
 
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
    #[test]
    fn half_sensitivity_is_point_one_five_degrees_per_pixel() {
        assert!((degrees_per_pixel(0.5) - 0.15).abs() < 1e-9);
    }

     
     
    #[test]
    fn pixel_rounding_stays_within_half_a_pixel() {
        let deg_per_px = degrees_per_pixel(0.5);
        for step in [-37.0_f64, -1.0, 0.4, 12.5, 90.0] {
            let px = (step / deg_per_px).round();
            assert!((px * deg_per_px - step).abs() <= deg_per_px / 2.0 + 1e-9);
        }
        assert_eq!((0.0_f64 / deg_per_px).round() as i32, 0);
    }

     
     
     
    #[test]
    fn every_input_combination_points_at_its_own_yaw_offsets() {
        assert_eq!(keys(true, false, false, false).angle(), Some(0.0));
        assert_eq!(keys(true, false, true, false).angle(), Some(-45.0));
        assert_eq!(keys(false, false, true, false).angle(), Some(-90.0));
        assert_eq!(keys(false, true, true, false).angle(), Some(-135.0));
        assert_eq!(
            keys(false, true, false, false).angle().map(f32::abs),
            Some(180.0)
        );
        assert_eq!(keys(false, true, false, true).angle(), Some(135.0));
        assert_eq!(keys(false, false, false, true).angle(), Some(90.0));
        assert_eq!(keys(true, false, false, true).angle(), Some(45.0));
        assert_eq!(keys(false, false, false, false).angle(), None);
        assert_eq!(
            keys(true, true, false, false).angle(),
            None,
            "opposing keys cancel out"
        );
    }

     
     
     
     
    #[test]
    fn a_turn_keeps_the_step_within_half_a_key_step_of_the_players_heading() {
        for (_, held) in INPUT_COMBOS {
            for turn in [
                -179.0_f32, -121.0, -90.0, -45.0, -13.0, 0.0, 37.0, 90.0, 178.0,
            ] {
                let fixed = movement_fix(held, 0.0, turn).expect("a held input is preserved");
                let wanted = held.angle().unwrap() - turn;
                let err = wrap_degrees(fixed.angle().unwrap() - wanted).abs();
                assert!(
                    err <= 22.5 + 1e-3,
                    "{held:?} through {turn}° lands {err}° off"
                );
            }
        }
    }

     
     
    #[test]
    fn a_quarter_turn_swaps_forward_for_the_equivalent_strafe() {
        let forward = keys(true, false, false, false);
        assert_eq!(
            movement_fix(forward, 0.0, 90.0),
            Some(keys(false, false, true, false))
        );
        assert_eq!(
            movement_fix(forward, 40.0, -50.0),
            Some(keys(false, false, false, true)),
            "a turn is measured from the player's own yaw, not from zero"
        );
    }

     
     
    #[test]
    fn a_turn_with_no_movement_input_holds_nothing() {
        assert_eq!(movement_fix(MoveKeys::default(), 0.0, 90.0), None);
    }

     
     
     
     
    #[test]
    fn a_body_aim_remembers_the_first_view_of_a_frame_only() {
        let view = Rotation::new(90.0, 4.0);
        let held = keys(true, false, false, false);
        let now = Instant::now();
        let mut body = BodyAim::default();

        body.begin(BodyAimOwner::SilentAura, view, held, now);
        assert_eq!(
            body.pending().map(|(view, _, _, _)| view),
            Some(view),
            "the frame keeps the player's own view, not the aim"
        );
        assert_eq!(body.take().map(|(view, _, _, _)| view), Some(view));
        assert_eq!(body.take(), None, "a restore happens once");

         
        let later = Rotation::new(12.0, -8.0);
        body.begin(BodyAimOwner::SilentAura, later, MoveKeys::default(), now);
        assert_eq!(body.take().map(|(view, _, _, _)| view), Some(later));
    }

     
     
     
    #[test]
    fn a_second_turn_of_a_frame_counter_rotates_the_players_input_again() {
        let view = Rotation::new(10.0, 0.0);
        let held = keys(true, false, false, false);
        let now = Instant::now();
        let mut body = BodyAim::default();
        body.begin(BodyAimOwner::SilentAura, view, held, now);

        let first = movement_fix(held, view.yaw, 100.0).expect("forward is preserved");
        body.set_applied(first);
        let (frame_view, _, frame_held, applied) = body.pending().expect("the turn is pending");
        assert_eq!(frame_view, view, "the frame keeps the player's view");
        assert_eq!(
            frame_held, held,
            "the turn never becomes the player's input"
        );
        assert_eq!(applied, first);

        let second = movement_fix(frame_held, frame_view.yaw, 190.0).expect("still preserved");
        assert_ne!(
            second, first,
            "a different aim needs a different input in the turned frame"
        );
    }

     
     
     
    #[test]
    fn a_body_aim_goes_stale_when_its_frame_never_comes() {
        let now = Instant::now();
        let mut body = BodyAim::default();
        assert!(!body.stale(now), "no turn, nothing to hand back");

        body.begin(
            BodyAimOwner::SilentAura,
            Rotation::new(0.0, 0.0),
            keys(false, false, true, false),
            now,
        );
        assert!(!body.stale(now + BODY_AIM_TIMEOUT / 2));
        assert!(
            body.stale(now + BODY_AIM_TIMEOUT + Duration::from_millis(1)),
            "a turn older than a frame is orphaned"
        );
        assert!(body.take().is_some());
        assert!(!body.stale(now + BODY_AIM_TIMEOUT * 4), "taking clears it");
    }

     
     
     
    #[test]
    fn a_body_aim_belongs_to_the_module_that_made_it() {
        let now = Instant::now();
        let view = Rotation::new(20.0, 0.0);
        let held = keys(true, false, false, false);
        let mut body = BodyAim::default();
        assert_eq!(body.owner(), None, "no turn, no owner");

        body.begin(BodyAimOwner::Scaffold, view, held, now);
        assert_eq!(body.owner(), Some(BodyAimOwner::Scaffold));
        assert_eq!(body.take().map(|(view, _, _, _)| view), Some(view));
        assert_eq!(body.owner(), None, "taking clears the owner too");
    }

     
     
     
    #[test]
    fn clearing_releases_the_silent_shadow_rotation() {
        let _guard = test_guard();
        clear();
        aim_silent(Rotation::new(40.0, 10.0), 100_000.0);
        assert!(
            silent_rotation().is_some(),
            "an active silent aim publishes its shadow rotation"
        );

        clear();
        assert_eq!(silent_rotation(), None, "a released aim leaves no shadow");
        assert_eq!(current_rotation(), None, "no camera target remains");
    }

     
     
     
     
     
     
     
     
    #[test]
    fn a_snap_command_reads_back_as_already_arrived() {
        let _guard = test_guard();
        clear();
        assert_eq!(current_rotation(), None, "no controller, no rotation");

         
        let goal = Rotation::new(123.0, -45.0);
        aim_real_mouse(goal, 100_000.0);
        let commanded = current_rotation().expect("the command is active");
        assert!(
            commanded.angle_to(goal) < 1e-3,
            "a snap command must read back as arrived, got {commanded:?}"
        );

         
         
        let gentle_goal = Rotation::new(-90.0, 30.0);
        aim_real_mouse(gentle_goal, 4.0);
        let commanded = current_rotation().expect("the command is active");
        let residual = commanded.angle_to(gentle_goal);
        assert!(
            residual > 1.5,
            "a gentle command must not read back as arrived (residual {residual})"
        );

         
         
         
        assert_eq!(
            silent_rotation(),
            None,
            "a camera-writing aim must never publish a shadow rotation"
        );
        let silent_goal = Rotation::new(45.0, -20.0);
        aim_silent(silent_goal, 100_000.0);
        let advertised = silent_rotation().expect("a silent aim publishes its angle");
        assert!(
            advertised.angle_to(silent_goal) < 1e-3,
            "a silent snap must read back as arrived, got {advertised:?}"
        );
        assert!(
            current_rotation().is_some(),
            "the silent angle is the commanded rotation too"
        );
        aim_silent(Rotation::new(-9.0, 3.0), 4.0);
        let gentle = silent_rotation().expect("still silent");
        assert!(
            gentle.angle_to(Rotation::new(-9.0, 3.0)) > 0.0 || gentle.angle_to(silent_goal) > 0.0,
            "a gentle silent sweep eases instead of snapping"
        );
        clear();
        assert_eq!(
            silent_rotation(),
            None,
            "releasing the aim clears the shadow"
        );
    }

     
     
    #[test]
    fn an_aim_is_written_on_the_players_own_turn() {
        assert_eq!(align_yaw(45.0, 45.0), 45.0);
        assert!((align_yaw(-170.0, 350.0) - 190.0).abs() < 1e-4);
        assert!((align_yaw(170.0, -350.0) + 190.0).abs() < 1e-4);

        for (angle, reference) in [
            (-170.0, 350.0),
            (170.0, -350.0),
            (179.0, -179.0),
            (10.0, 730.0),
            (-179.0, 179.0),
        ] {
            let aligned = align_yaw(angle, reference);
            assert!(
                wrap_degrees(aligned - angle).abs() < 1e-3,
                "{aligned} must point the same way as {angle}"
            );
            assert!(
                (aligned - reference).abs() <= 180.0 + 1e-3,
                "{aligned} must be the short way from {reference}"
            );
        }
    }

     
     
    #[test]
    fn restoring_keeps_the_mouse_movement_made_during_the_turn() {
        let view = Rotation::new(45.0, 10.0);
        let aim = Rotation::new(135.0, -20.0);
        assert_eq!(restored_view(view, aim, aim), view);

         
        let moved = Rotation::new(145.0, -18.0);
        let restored = restored_view(view, aim, moved);
        assert!((restored.yaw - 55.0).abs() < 1e-3);
        assert!((restored.pitch - 12.0).abs() < 1e-3);

         
         
        let view = Rotation::new(179.0, 0.0);
        let aim = Rotation::new(-179.0, 0.0);
        let restored = restored_view(view, aim, aim);
        assert!(wrap_degrees(restored.yaw - 179.0).abs() < 1e-3);
        assert!((restored.yaw + 179.0).abs() <= 2.0 + 1e-3);
    }
}
