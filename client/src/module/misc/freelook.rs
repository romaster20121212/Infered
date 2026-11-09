 
use crate::graphic::input;
use crate::mapping::client::camera::Camera;
use crate::mapping::client::options::Perspective;
use crate::mapping::entity::EntityRef;
use crate::mapping::math::Vec3;
use crate::mapping::MappedObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{mapping, minecraft};
use jni::objects::JObject;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

 
static OWNS_MOUSE: AtomicBool = AtomicBool::new(false);
 
static AUTO_BODY_VIEW: AtomicBool = AtomicBool::new(false);

 
pub(crate) fn render_reveal_active() -> bool {
    OWNS_MOUSE.load(Ordering::Acquire)
}

pub fn owns_mouse() -> bool {
     
     
    OWNS_MOUSE.load(Ordering::Relaxed)
        && minecraft()
            .options()
            .and_then(|options| options.perspective())
            .is_ok_and(|perspective| third_person_freelook(true, perspective))
}

 
 
pub(crate) fn request_body_view() {
    AUTO_BODY_VIEW.store(true, Ordering::Release);
}

 
pub(crate) fn release_body_view() {
    AUTO_BODY_VIEW.store(false, Ordering::Release);
}

 
pub(crate) fn body_view_requested() -> bool {
    AUTO_BODY_VIEW.load(Ordering::Acquire) || input::player_action_active()
}

fn third_person_freelook(enabled: bool, perspective: Perspective) -> bool {
    enabled && perspective != Perspective::FirstPerson
}

fn set_capture(enabled: bool) -> anyhow::Result<()> {
    crate::camera_bridge::set_freelook_no_clip(enabled)?;
    if OWNS_MOUSE.swap(enabled, Ordering::Relaxed) != enabled {
        input::freelook_drop_in();
    }
    Ok(())
}

 
 
 
 
fn effective_capture(enabled: bool, perspective: Perspective) -> bool {
    third_person_freelook(enabled, perspective) && !body_view_requested()
}

 
fn sync_capture_for_player_action(enabled: bool, perspective: Perspective) -> anyhow::Result<()> {
    set_capture(effective_capture(enabled, perspective))
}

 
fn capture_allowed(enabled: bool, perspective: Perspective) -> bool {
    effective_capture(enabled, perspective)
}

pub fn hold_mode(data: &ModuleData) -> bool {
    matches!(
        data.get_setting("Activation"),
        Some(ModuleSetting::Choice { value: 1, .. })
    )
}

#[derive(Debug)]
struct FreelookState {
    yaw: f32,
    pitch: f32,
    orbit_active: bool,
    returning: bool,
    return_position: Vec3,
    return_weight: f32,
    rendered_yaw: f32,
    rendered_pitch: f32,
    last_frame: Instant,
}

#[derive(Debug)]
pub struct FreelookModule {
    pub module: ModuleData,
    state: Mutex<Option<FreelookState>>,
}

impl FreelookModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Freelook,
                description: "Independent orbit camera only while F5 is in third person".into(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyF6,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Activation".into(),
                        value: 0,
                        options: vec!["Toggle".into(), "Hold".into()],
                    },
                    ModuleSetting::Slider {
                        name: "Distance".into(),
                        value: 4.0,
                        min: 0.0,
                        max: 32.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Smooth Return".into(),
                        value: false,
                    },
                    ModuleSetting::Slider {
                        name: "Return Speed".into(),
                        value: 12.0,
                        min: 1.0,
                        max: 40.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Invert Mouse".into(),
                        value: false,
                    },
                ],
            },
            state: Mutex::new(None),
        }
    }

    fn toggle_setting(&self, name: &str) -> bool {
        self.module
            .get_setting(name)
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(false)
    }

    fn slider(&self, name: &str, default: f32, min: f32, max: f32) -> f32 {
        let value = self
            .module
            .get_setting(name)
            .and_then(ModuleSetting::get_slider_value)
            .unwrap_or(default);
        if value.is_finite() {
            value.clamp(min, max)
        } else {
            default
        }
    }

    fn clear(&self) -> anyhow::Result<()> {
        if self
            .state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_some()
        {
            let result = self.on_stop();
            self.state.lock().unwrap_or_else(|p| p.into_inner()).take();
            result?;
        }
        Ok(())
    }
}

impl Module for FreelookModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
        anyhow::ensure!(
            crate::jvmti::transform_installed("Camera"),
            "Freelook camera hook unavailable"
        );
        anyhow::ensure!(
            crate::jvmti::transform_installed("CameraCollision"),
            "Freelook collision hook unavailable"
        );
        let mut guard = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(state) = guard.as_mut().filter(|state| !state.returning) {
             
             
            state.last_frame = Instant::now();
        } else {
            let Some(player) = minecraft().player()? else {
                return Ok(());
            };
            let yaw = player.get_yaw()?;
            let pitch = player.get_pitch()?;
            let position = player.get_eye_position()?;
            *guard = Some(FreelookState {
                yaw,
                pitch,
                orbit_active: false,
                returning: false,
                return_position: position,
                return_weight: 1.0,
                last_frame: Instant::now(),
                rendered_yaw: yaw,
                rendered_pitch: pitch,
            });
        }
         
        let perspective = minecraft().options()?.perspective()?;
        sync_capture_for_player_action(self.module.enabled, perspective)
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        set_capture(false)?;
        let mut guard = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let Some(state) = guard.as_mut() else {
            return Ok(());
        };
        let third_person = minecraft()
            .options()
            .and_then(|options| options.perspective())
            .is_ok_and(|perspective| third_person_freelook(true, perspective));
        if state.orbit_active
            && third_person
            && self.toggle_setting("Smooth Return")
            && crate::RUNNING.load(Ordering::SeqCst)
        {
            state.returning = true;
            state.return_weight = 1.0;
            state.last_frame = Instant::now();
        } else {
            *guard = None;
        }
        Ok(())
    }

    fn on_world_suspend(&self) -> anyhow::Result<()> {
        if !crate::RUNNING.load(Ordering::SeqCst) {
            return self.on_stop();
        }
        set_capture(false)
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        let connected = minecraft()
            .in_frame(|| {
                Ok(!minecraft()
                    .call_method("getConnection", &[])?
                    .l()?
                    .is_null())
            })
            .unwrap_or(false);
        if !connected || !crate::RUNNING.load(Ordering::SeqCst) {
            self.clear()?;
        }
        Ok(())
    }

    fn on_camera_setup_object(&self, object: &JObject) -> anyhow::Result<()> {
        if !crate::state::client().modules.is_active() {
            return set_capture(false);
        }
        let perspective = minecraft().options()?.perspective()?;
        let mut guard = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let Some(state) = guard.as_mut() else {
            return set_capture(false);
        };
        if perspective == Perspective::FirstPerson {
             
             
            set_capture(false)?;
            state.orbit_active = false;
            state.last_frame = Instant::now();
            if state.returning || !self.module.enabled {
                *guard = None;
            }
            return Ok(());
        }
        if !self.module.enabled && !state.returning {
            drop(guard);
            self.on_stop()?;
            return Ok(());
        }
        sync_capture_for_player_action(self.module.enabled, perspective)?;
        if !capture_allowed(self.module.enabled, perspective) {
             
             
             
            state.last_frame = Instant::now();
            return Ok(());
        }
        mapping()
            .in_frame(|| {
                let camera = Camera::new(mapping().get_env()?.new_global_ref(object)?);
                if !state.orbit_active && !state.returning {
                     
                     
                    let Some(player) = minecraft().player()? else {
                        return Ok(false);
                    };
                    state.yaw = player.get_yaw()?;
                    state.pitch = player.get_pitch()?;
                    state.orbit_active = true;
                }
                let dt = state.last_frame.elapsed().as_secs_f32().min(0.1);
                state.last_frame = Instant::now();
                if state.returning {
                    state.return_weight *=
                        (-self.slider("Return Speed", 12.0, 1.0, 40.0) * dt).exp();
                    if state.return_weight < 0.001 {
                        return Ok(true);
                    }
                    let target = camera.position()?;
                    camera.set_position(blend_position(
                        target,
                        state.return_position,
                        state.return_weight,
                    ))?;
                    camera.set_rotation(
                        camera.yaw()?
                            + wrap_degrees(state.rendered_yaw - camera.yaw()?)
                                * state.return_weight,
                        camera.pitch()?
                            + wrap_degrees(state.rendered_pitch - camera.pitch()?)
                                * state.return_weight,
                    )?;
                    return Ok(false);
                }
                if input::menu_open() {
                    input::freelook_drop_in();
                } else {
                    let (dx, dy) = input::freelook_take_look_delta();
                    let sensitivity = minecraft().options()?.sensitivity()?.get_double()?;
                    let scale = mouse_scale(sensitivity);
                    state.yaw = wrap_degrees(state.yaw + (dx * scale) as f32);
                    state.pitch = wrap_degrees(
                        state.pitch
                            + (dy * scale) as f32
                                * if self.toggle_setting("Invert Mouse") {
                                    -1.0
                                } else {
                                    1.0
                                },
                    );
                }
                let (yaw, pitch) = perspective_rotation(state.yaw, state.pitch, perspective);
                 
                 
                let pivot = camera.focus_position()?;
                let distance = self.slider("Distance", 4.0, 0.0, 32.0) as f64;
                let position = orbit_position(pivot, yaw, pitch, distance);
                camera.set_rotation(yaw, pitch)?;
                camera.set_position(position)?;
                state.return_position = position;
                state.rendered_yaw = yaw;
                state.rendered_pitch = pitch;
                Ok(false)
            })
            .map(|finished| {
                if finished {
                    *guard = None;
                }
            })
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

fn wrap_degrees(angle: f32) -> f32 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

fn mouse_scale(sensitivity: f64) -> f64 {
    (sensitivity.clamp(0.0, 1.0) * 0.6 + 0.2).powi(3) * 8.0 * 0.15
}

fn perspective_rotation(yaw: f32, pitch: f32, perspective: Perspective) -> (f32, f32) {
    if perspective == Perspective::ThirdPersonFront {
        (wrap_degrees(yaw + 180.0), -pitch)
    } else {
        (yaw, pitch)
    }
}

fn orbit_position(pivot: Vec3, yaw: f32, pitch: f32, distance: f64) -> Vec3 {
    let yaw = (yaw as f64).to_radians();
    let pitch = (pitch as f64).to_radians();
    Vec3::new(
        pivot.x() + yaw.sin() * pitch.cos() * distance,
        pivot.y() + pitch.sin() * distance,
        pivot.z() - yaw.cos() * pitch.cos() * distance,
    )
}

fn blend_position(target: Vec3, start: Vec3, weight: f32) -> Vec3 {
    let w = weight as f64;
    Vec3::new(
        target.x() + (start.x() - target.x()) * w,
        target.y() + (start.y() - target.y()) * w,
        target.z() + (start.z() - target.z()) * w,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freelook_requires_enabled_module_and_a_third_person_view() {
        for perspective in [
            Perspective::FirstPerson,
            Perspective::ThirdPersonBack,
            Perspective::ThirdPersonFront,
        ] {
            assert!(!third_person_freelook(false, perspective));
        }
        assert!(!third_person_freelook(true, Perspective::FirstPerson));
        assert!(third_person_freelook(true, Perspective::ThirdPersonBack));
        assert!(third_person_freelook(true, Perspective::ThirdPersonFront));
        assert!(FreelookModule::new()
            .module
            .get_setting("Force Third Person")
            .is_none());
    }

    #[test]
    fn pitch_is_periodic_but_never_clamped_at_the_poles() {
        assert_eq!(wrap_degrees(120.0), 120.0);
        assert_eq!(wrap_degrees(450.0), 90.0);
        assert_eq!(wrap_degrees(-120.0), -120.0);
    }

    #[test]
    fn f5_changes_base_view_without_changing_owned_angles() {
        let (yaw, pitch) = perspective_rotation(30.0, 120.0, Perspective::ThirdPersonFront);
        assert_eq!((yaw, pitch), (-150.0, -120.0));
        assert_eq!(
            perspective_rotation(30.0, 120.0, Perspective::FirstPerson),
            (30.0, 120.0)
        );
    }

    #[test]
    fn orbit_is_constant_radius_even_upside_down() {
        let pivot = Vec3::new(1.0, 65.0, -3.0);
        for pitch in [-270.0, -120.0, -90.0, 0.0, 90.0, 120.0, 270.0] {
            let pos = orbit_position(pivot, 42.0, pitch, 4.0);
            let radius = ((pos.x() - pivot.x()).powi(2)
                + (pos.y() - pivot.y()).powi(2)
                + (pos.z() - pivot.z()).powi(2))
            .sqrt();
            assert!((radius - 4.0).abs() < 1e-9);
        }
        let first = orbit_position(pivot, 0.0, 0.0, 0.0);
        assert_eq!(
            (first.x(), first.y(), first.z()),
            (pivot.x(), pivot.y(), pivot.z())
        );
    }

    #[test]
    fn mouse_scale_matches_vanilla_and_return_converges() {
        assert!((mouse_scale(0.5) - 0.15).abs() < 1e-9);
        let weight = (-12.0_f32 / 60.0).exp().powi(60);
        assert!(weight < 0.001);
        assert_eq!(wrap_degrees(1.0 - 359.0), 2.0);
    }

    #[test]
    fn freelook_yields_to_a_physical_click_without_disarming_the_orbit() {
        for perspective in [Perspective::ThirdPersonBack, Perspective::ThirdPersonFront] {
            assert!(capture_allowed(true, perspective));
            assert!(!effective_capture(false, perspective));
        }
        assert!(!capture_allowed(true, Perspective::FirstPerson));
    }

    #[test]
    fn automatic_body_view_handoff_is_explicit_and_released() {
        release_body_view();
        assert!(!body_view_requested());
        request_body_view();
        assert!(body_view_requested());
        release_body_view();
        assert!(!body_view_requested());
    }
}
