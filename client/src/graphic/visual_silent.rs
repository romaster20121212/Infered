 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::client::options::Perspective;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::EntityRef;
use crate::mapping::{FieldType, MappedObject};
use crate::module::combat::rotation::{align_yaw, silent_rotation, Rotation};
use crate::state::minecraft;
use jni::objects::JValue;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

 
pub const SILENT_AURA: u8 = 1 << 0;
 
pub const SCAFFOLD: u8 = 1 << 1;
 
pub const FAST_SCAFFOLD: u8 = 1 << 2;

 
 
 
 
const HOLD: Duration = Duration::from_millis(250);

 
 
static OWNERS: AtomicU8 = AtomicU8::new(0);

 
 
static HELD: Mutex<Option<(Rotation, Instant)>> = Mutex::new(None);

 
 
static APPLIED: Mutex<Option<Saved>> = Mutex::new(None);

 
 
 
 
#[derive(Debug, Clone, Copy, PartialEq)]
struct Saved {
    yaw: f32,
    yaw_old: f32,
    pitch: f32,
    pitch_old: f32,
    body_yaw: f32,
    body_yaw_old: f32,
    head_yaw: f32,
    head_yaw_old: f32,
}

fn owners() -> u8 {
    OWNERS.load(Ordering::Relaxed)
}

fn held() -> MutexGuard<'static, Option<(Rotation, Instant)>> {
    HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn applied() -> MutexGuard<'static, Option<Saved>> {
    APPLIED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
 
pub fn set_owner(owner: u8, enabled: bool) {
    if enabled {
        OWNERS.fetch_or(owner, Ordering::Relaxed);
        return;
    }
    let previous = OWNERS.fetch_and(!owner, Ordering::Relaxed);
    if previous & !owner == 0 {
        *held() = None;
    }
}

 
pub fn enabled() -> bool {
    owners() != 0
}

 
 
fn live_angle() -> Option<Rotation> {
    let mask = owners();
    let silent_aura = mask & SILENT_AURA != 0;
    let scaffold = mask & SCAFFOLD != 0;
    let fast_scaffold = mask & FAST_SCAFFOLD != 0;
    if silent_aura {
        if let Some(aim) = silent_rotation() {
            return Some(aim);
        }
    }
    if scaffold {
        if let Some(aim) = crate::module::world::scaffold_aim::current() {
            return Some(Rotation::new(aim.yaw, aim.pitch));
        }
    }
    if fast_scaffold {
        if let Some(aim) = crate::module::world::fast_scaffold_aim::current() {
            return Some(Rotation::new(aim.yaw, aim.pitch));
        }
    }
    None
}

 
 
fn drawn_angle(
    live: Option<Rotation>,
    held: &mut Option<(Rotation, Instant)>,
    now: Instant,
) -> Option<Rotation> {
    match live.filter(|aim| aim.yaw.is_finite() && aim.pitch.is_finite()) {
        Some(aim) => {
            *held = Some((aim, now));
            Some(aim)
        }
        None => match held.as_ref() {
            Some((aim, at)) if now.saturating_duration_since(*at) <= HOLD => Some(*aim),
            _ => None,
        },
    }
}

 
 
 
fn aimed_pose(aim: Rotation, view_yaw: f32) -> Rotation {
    Rotation::new(align_yaw(aim.yaw, view_yaw), aim.pitch.clamp(-90.0, 90.0))
}

 
 
 
fn write_pose(player: &LocalPlayer, pose: Rotation) -> anyhow::Result<()> {
    player.call_method("setYRot", &[JValue::Float(pose.yaw)])?;
    player.call_method("setXRot", &[JValue::Float(pose.pitch)])?;
    player.set_field("yRotO", FieldType::Float, JValue::Float(pose.yaw))?;
    player.set_field("xRotO", FieldType::Float, JValue::Float(pose.pitch))?;
     
     
     
    for field in ["yBodyRot", "yBodyRotO", "yHeadRot", "yHeadRotO"] {
        player.set_field(field, FieldType::Float, JValue::Float(pose.yaw))?;
    }
    Ok(())
}

fn float(player: &LocalPlayer, field: &str) -> Option<f32> {
    player.get_field(field, FieldType::Float).ok()?.f().ok()
}

 
 
 
pub fn apply() {
     
     
    restore();
    if !enabled() || !third_person() {
        return;
    }
    let Some(aim) = drawn_angle(live_angle(), &mut held(), Instant::now()) else {
        return;
    };
    let Ok(Some(player)) = minecraft().player() else {
        return;
    };
    let (Ok(view_yaw), Ok(view_pitch)) = (player.get_yaw(), player.get_pitch()) else {
        return;
    };
    let saved = Saved {
        yaw: view_yaw,
        yaw_old: float(&player, "yRotO").unwrap_or(view_yaw),
        pitch: view_pitch,
        pitch_old: float(&player, "xRotO").unwrap_or(view_pitch),
        body_yaw: float(&player, "yBodyRot").unwrap_or(view_yaw),
        body_yaw_old: float(&player, "yBodyRotO").unwrap_or(view_yaw),
        head_yaw: float(&player, "yHeadRot").unwrap_or(view_yaw),
        head_yaw_old: float(&player, "yHeadRotO").unwrap_or(view_yaw),
    };
    if let Err(error) = write_pose(&player, aimed_pose(aim, view_yaw)) {
        log::debug!("Visual Silent: model pose skipped: {error}");
        return;
    }
    *applied() = Some(saved);
}

 
 
pub fn restore() {
    let Some(saved) = applied().take() else {
        return;
    };
     
     
    let Some(game) = crate::state::try_minecraft() else {
        return;
    };
    let Ok(Some(player)) = game.player() else {
        return;
    };
    let _ = player.call_method("setYRot", &[JValue::Float(saved.yaw)]);
    let _ = player.call_method("setXRot", &[JValue::Float(saved.pitch)]);
    for (field, value) in [
        ("yRotO", saved.yaw_old),
        ("xRotO", saved.pitch_old),
        ("yBodyRot", saved.body_yaw),
        ("yBodyRotO", saved.body_yaw_old),
        ("yHeadRot", saved.head_yaw),
        ("yHeadRotO", saved.head_yaw_old),
    ] {
        let _ = player.set_field(field, FieldType::Float, JValue::Float(value));
    }
}

 
 
 
fn third_person() -> bool {
    minecraft()
        .options()
        .and_then(|options| options.perspective())
        .map(|perspective| perspective != Perspective::FirstPerson)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
    fn guard() -> MutexGuard<'static, ()> {
        static TEST: Mutex<()> = Mutex::new(());
        let guard = TEST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        OWNERS.store(0, Ordering::Relaxed);
        *held() = None;
        guard
    }

    #[test]
    fn any_owner_turns_the_effect_on_and_the_last_one_off_turns_it_off() {
        let _guard = guard();
        assert!(!enabled(), "nothing is drawn before a module asks");
        set_owner(SILENT_AURA, true);
        assert!(enabled());
        set_owner(SCAFFOLD, true);
        set_owner(SILENT_AURA, false);
        assert!(enabled(), "Scaffold still wants its aim drawn");
        set_owner(SCAFFOLD, false);
        assert!(!enabled(), "the last owner out turns it off");
    }

    #[test]
    fn every_owner_is_its_own_bit() {
        let _guard = guard();
        for owner in [SILENT_AURA, SCAFFOLD, FAST_SCAFFOLD] {
            set_owner(owner, true);
        }
        assert_eq!(owners(), SILENT_AURA | SCAFFOLD | FAST_SCAFFOLD);
        set_owner(SCAFFOLD, false);
        assert_eq!(owners(), SILENT_AURA | FAST_SCAFFOLD);
        assert!(enabled());
    }

    #[test]
    fn the_aim_is_drawn_on_the_players_own_yaw_branch() {
         
         
         
        let pose = aimed_pose(Rotation::new(-170.0, 12.0), 175.0);
        assert!((pose.yaw - (-170.0)).abs() < 1e-3 || (pose.yaw - 190.0).abs() < 1e-3);
        assert!(
            (pose.yaw - 175.0).abs() < 20.0,
            "drawn on the player's branch, not a turn away: {pose:?}"
        );
        assert_eq!(pose.pitch, 12.0, "pitch is passed through as is");
    }

    #[test]
    fn the_pitch_stays_inside_the_range_the_game_accepts() {
        assert_eq!(aimed_pose(Rotation::new(10.0, 140.0), 10.0).pitch, 90.0);
        assert_eq!(aimed_pose(Rotation::new(10.0, -140.0), 10.0).pitch, -90.0);
    }

    #[test]
    fn a_live_angle_is_drawn_and_remembered() {
        let now = Instant::now();
        let mut held = None;
        let aim = Rotation::new(42.0, -8.0);
        assert_eq!(drawn_angle(Some(aim), &mut held, now), Some(aim));
        assert_eq!(held.map(|(aim, _)| aim), Some(aim), "remembered");
    }

    #[test]
    fn the_last_angle_stays_through_a_gap_and_then_expires() {
        let now = Instant::now();
        let mut held = None;
        let aim = Rotation::new(42.0, -8.0);
        drawn_angle(Some(aim), &mut held, now);
        assert_eq!(
            drawn_angle(None, &mut held, now + HOLD / 2),
            Some(aim),
            "an aim between refreshes keeps the model on it"
        );
        assert_eq!(
            drawn_angle(None, &mut held, now + HOLD * 2),
            None,
            "a released aim hands the model back"
        );
    }

    #[test]
    fn a_corrupt_angle_is_never_drawn() {
        let now = Instant::now();
        let mut held = None;
        assert_eq!(
            drawn_angle(Some(Rotation::new(f32::NAN, 0.0)), &mut held, now),
            None
        );
        assert!(held.is_none(), "a NaN angle is not remembered either");
    }

    #[test]
    fn leaving_clears_the_remembered_angle() {
        let _guard = guard();
        set_owner(SILENT_AURA, true);
        *held() = Some((Rotation::new(1.0, 2.0), Instant::now()));
        set_owner(SILENT_AURA, false);
        assert!(
            held().is_none(),
            "re-enabling must not draw the angle from before"
        );
    }
}
