 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::client::options::Perspective;
use crate::mapping::entity::EntityRef;
use crate::module::combat::rotation::{self, BodyAimOwner, Rotation};
use crate::module::world::fast_scaffold_aim::{self, Aim};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, mapping, minecraft};
use jni::objects::{JClass, JValue};
use std::sync::atomic::{AtomicBool, Ordering};

 
pub(crate) const TECHNIQUES: [&str; 2] = ["Normal", "Expand"];
 
pub(crate) const TOWERS: [&str; 2] = ["Off", "Motion"];
 
pub(crate) const DEFAULT_AIM_SPEED: f32 = 180.0;
pub(crate) const AIM_SPEED_RANGE: (f32, f32) = (1.0, 180.0);
 
pub(crate) const DEFAULT_DELAY: f32 = 0.0;
pub(crate) const DELAY_RANGE: (f32, f32) = (0.0, 10.0);
 
pub(crate) const DEFAULT_MIN_DISTANCE: f32 = 0.0;
pub(crate) const MIN_DISTANCE_RANGE: (f32, f32) = (0.0, 0.25);
 
pub(crate) const DEFAULT_EXPAND_LENGTH: f32 = 4.0;
pub(crate) const EXPAND_LENGTH_RANGE: (f32, f32) = (1.0, 10.0);

 
 
 
 
static FIRST_PERSON: AtomicBool = AtomicBool::new(true);
 
 
 
 
static BODY_HELD: AtomicBool = AtomicBool::new(false);

 
 
fn body_aim_available() -> bool {
    FIRST_PERSON.load(Ordering::Relaxed) && crate::jvmti::camera_frame_hook_installed()
}

 
 
 
fn first_person_view() -> bool {
    minecraft()
        .options()
        .and_then(|options| options.perspective())
        .map(|perspective| perspective == Perspective::FirstPerson)
        .unwrap_or(true)
}

 
 
 
 
 
 
 
 
 
 
pub(crate) fn arm_body(yaw: f32, pitch: f32) -> anyhow::Result<()> {
    anyhow::ensure!(
        body_aim_available(),
        "the aim can only ride the body in first person with the camera restore live"
    );
    rotation::aim_body_for(BodyAimOwner::FastScaffold, Rotation::new(yaw, pitch))?;
    BODY_HELD.store(true, Ordering::Relaxed);
    Ok(())
}

 
#[derive(Debug)]
pub struct FastScaffoldModule {
    pub module: ModuleData,
     
     
    aim_held: AtomicBool,
     
     
    blocked: AtomicBool,
    slot_yield: AtomicBool,
     
    warned: AtomicBool,
}

impl FastScaffoldModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::FastScaffold,
                description:
                    "Experimental Scaffold copy: two-tick placement, tick-driven aim and edge hold"
                        .into(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Technique".into(),
                        value: 0,
                        options: TECHNIQUES.iter().map(|name| (*name).to_string()).collect(),
                    },
                    ModuleSetting::Slider {
                        name: "Expand Length".into(),
                        value: DEFAULT_EXPAND_LENGTH,
                        min: EXPAND_LENGTH_RANGE.0,
                        max: EXPAND_LENGTH_RANGE.1,
                    },
                    ModuleSetting::Choice {
                        name: "Tower".into(),
                        value: 0,
                        options: TOWERS.iter().map(|name| (*name).to_string()).collect(),
                    },
                    ModuleSetting::Slider {
                        name: "Aim Speed".into(),
                        value: DEFAULT_AIM_SPEED,
                        min: AIM_SPEED_RANGE.0,
                        max: AIM_SPEED_RANGE.1,
                    },
                    ModuleSetting::Slider {
                        name: "Delay".into(),
                        value: DEFAULT_DELAY,
                        min: DELAY_RANGE.0,
                        max: DELAY_RANGE.1,
                    },
                    ModuleSetting::Slider {
                        name: "Min Distance".into(),
                        value: DEFAULT_MIN_DISTANCE,
                        min: MIN_DISTANCE_RANGE.0,
                        max: MIN_DISTANCE_RANGE.1,
                    },
                    ModuleSetting::Toggle {
                        name: "Auto Block".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Safe Walk".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Silent Aim".into(),
                        value: true,
                    },
                     
                     
                     
                     
                    ModuleSetting::Toggle {
                        name: "Visual Silent".into(),
                        value: true,
                    },
                ],
            },
            aim_held: AtomicBool::new(false),
            blocked: AtomicBool::new(false),
            slot_yield: AtomicBool::new(false),
            warned: AtomicBool::new(false),
        }
    }

     
     
    fn slider(&self, name: &str, default: f32, (min, max): (f32, f32)) -> f32 {
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

    fn aim_speed(&self) -> f32 {
        self.slider("Aim Speed", DEFAULT_AIM_SPEED, AIM_SPEED_RANGE)
    }

    fn technique(&self) -> i32 {
        match self.module.get_setting("Technique") {
            Some(ModuleSetting::Choice { value, .. }) => (*value).min(TECHNIQUES.len() - 1) as i32,
            _ => 0,
        }
    }

    fn tower(&self) -> i32 {
        match self.module.get_setting("Tower") {
            Some(ModuleSetting::Choice { value, .. }) => (*value).min(TOWERS.len() - 1) as i32,
            _ => 0,
        }
    }

    fn expand_length(&self) -> i32 {
        self.slider("Expand Length", DEFAULT_EXPAND_LENGTH, EXPAND_LENGTH_RANGE)
            .round() as i32
    }

    fn delay(&self) -> i32 {
        self.slider("Delay", DEFAULT_DELAY, DELAY_RANGE).round() as i32
    }

    fn min_distance(&self) -> f32 {
        self.slider("Min Distance", DEFAULT_MIN_DISTANCE, MIN_DISTANCE_RANGE)
    }

    fn toggle(&self, name: &str) -> bool {
        self.module
            .get_setting(name)
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(true)
    }

     
     
    fn silent_aim(&self) -> bool {
        self.module
            .get_setting("Silent Aim")
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(true)
    }

     
     
    fn publish_visual_silent(&self) {
        crate::graphic::visual_silent::set_owner(
            crate::graphic::visual_silent::FAST_SCAFFOLD,
            self.silent_aim() && self.toggle("Visual Silent"),
        );
    }

     
     
    fn update_bridge(&self) -> anyhow::Result<()> {
        let bridge = crate::camera_bridge::fast_scaffold_bridge()
            .ok_or_else(|| anyhow::anyhow!("FastScaffold bridge unavailable"))?;
         
         
        let busy = |id: ModuleId| {
            client().modules.get(id).is_some_and(|handle| {
                handle
                    .try_lock()
                    .map(|module| module.get_module_data().enabled)
                    .unwrap_or(true)
            })
        };
        let camera_owner = camera_conflicts().into_iter().find(|id| busy(*id));
         
         
         
         
         
        let aim_owner = if crate::module::combat::rotation::silent_rotation().is_some() {
            aim_conflicts().into_iter().find(|id| busy(*id))
        } else {
            None
        };
        let slot_owner = conflicts_with_slots().into_iter().find(|id| busy(*id));
         
        let blocked = busy(ModuleId::Scaffold) || camera_owner.is_some() || aim_owner.is_some();
         
         
         
        let auto_block = self.toggle("Auto Block") && slot_owner.is_none();

        let mut env = mapping().get_env()?;
        let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
        env.call_static_method(
            &class,
            "settings",
            "(IIIIFFZZ)V",
            &[
                JValue::Int(self.technique()),
                JValue::Int(self.tower()),
                JValue::Int(self.expand_length()),
                JValue::Int(self.delay()),
                JValue::Float(self.aim_speed()),
                JValue::Float(self.min_distance()),
                JValue::Bool(auto_block.into()),
                JValue::Bool(self.toggle("Safe Walk").into()),
            ],
        )?;
        env.set_static_field(
            &class,
            (&class, "BLOCKED", "Z"),
            JValue::Bool(blocked.into()),
        )?;
        env.set_static_field(
            &class,
            (&class, "SILENT_AIM", "Z"),
            JValue::Bool(self.silent_aim().into()),
        )?;
        env.set_static_field(
            &class,
            (&class, "MENU_OPEN", "Z"),
            JValue::Bool(
                crate::graphic::input::GUI_OPEN
                    .load(std::sync::atomic::Ordering::Relaxed)
                    .into(),
            ),
        )?;
        if self.blocked.swap(blocked, Ordering::Relaxed) != blocked {
            if let Some(id) = camera_owner {
                log::info!("FastScaffold standing down: {id:?} owns the camera");
            } else if let Some(id) = aim_owner {
                log::info!("FastScaffold standing down: {id:?} is aiming right now");
            } else {
                log::info!("FastScaffold active");
            }
        }
        if self
            .slot_yield
            .swap(slot_owner.is_some(), Ordering::Relaxed)
            != slot_owner.is_some()
        {
            match slot_owner {
                Some(id) => log::info!("FastScaffold Auto Block off: {id:?} owns the hotbar"),
                None => log::info!("FastScaffold Auto Block restored"),
            }
        }
        Ok(())
    }

     
     
     
    fn sync_aim(&self) -> anyhow::Result<()> {
        match wanted_aim() {
            Ok(Some(aim)) => {
                 
                 
                 
                if fast_scaffold_aim::current().is_none() {
                    if let Ok(Some(player)) = minecraft().player() {
                        if let (Ok(yaw), Ok(pitch)) = (player.get_yaw(), player.get_pitch()) {
                            fast_scaffold_aim::seed(yaw, pitch);
                        }
                    }
                }
                fast_scaffold_aim::request(aim, self.aim_speed());
                self.aim_held.store(true, Ordering::Relaxed);
                Ok(())
            }
            Ok(None) => {
                self.release_aim();
                Ok(())
            }
            Err(error) => {
                self.release_aim();
                Err(error)
            }
        }
    }

     
    fn release_aim(&self) {
        if self.aim_held.swap(false, Ordering::Relaxed) {
            fast_scaffold_aim::clear();
        }
    }

     
     
     
    fn warn_if_silent_unavailable(&self) {
        if self.silent_aim() && !body_aim_available() && !self.warned.swap(true, Ordering::Relaxed)
        {
            log::warn!(
                "FastScaffold: Silent Aim needs first person and the camera restore; \
                 placing along the camera"
            );
        }
    }

     
     
     
     
     
     
    fn wire_aim(&self) -> Option<Aim> {
        if self.aim_held.load(Ordering::Relaxed) && bridge_wants_aim() {
            fast_scaffold_aim::current()
        } else {
            None
        }
    }
}

 
 
fn bridge_wants_aim() -> bool {
    let Some(bridge) = crate::camera_bridge::fast_scaffold_bridge() else {
        return false;
    };
    (|| -> anyhow::Result<bool> {
        let mut env = mapping().get_env()?;
        let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
        Ok(env.get_static_field(&class, "AIM_ACTIVE", "Z")?.z()?)
    })()
    .unwrap_or(false)
}

 
 
fn wanted_aim() -> anyhow::Result<Option<Aim>> {
    let Some(bridge) = crate::camera_bridge::fast_scaffold_bridge() else {
        return Ok(None);
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    if !env.get_static_field(&class, "AIM_ACTIVE", "Z")?.z()? {
        return Ok(None);
    }
    let yaw = env.get_static_field(&class, "AIM_YAW", "F")?.f()?;
    let pitch = env.get_static_field(&class, "AIM_PITCH", "F")?.f()?;
    Ok(Some(Aim::new(yaw, pitch)))
}

 
 
fn publish_shadow(shadow: Option<Aim>) -> anyhow::Result<()> {
    let Some(bridge) = crate::camera_bridge::fast_scaffold_bridge() else {
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let (active, yaw, pitch) = match shadow {
        Some(aim) => (true, aim.yaw, aim.pitch),
        None => (false, 0.0, 0.0),
    };
    env.set_static_field(&class, (&class, "SHADOW_YAW", "F"), JValue::Float(yaw))?;
    env.set_static_field(&class, (&class, "SHADOW_PITCH", "F"), JValue::Float(pitch))?;
    env.set_static_field(
        &class,
        (&class, "SHADOW_ACTIVE", "Z"),
        JValue::Bool(active.into()),
    )?;
    Ok(())
}

 
 
 
fn camera_conflicts() -> [ModuleId; 6] {
    [
        ModuleId::Freecam,
        ModuleId::Freelook,
        ModuleId::Fly,
        ModuleId::Speed,
        ModuleId::Strafe,
        ModuleId::Nuker,
    ]
}

 
 
 
fn aim_conflicts() -> [ModuleId; 5] {
    [
        ModuleId::KillAura,
        ModuleId::MobAura,
        ModuleId::SilentAura,
        ModuleId::Aimbot,
        ModuleId::AutoCrystal,
    ]
}

 
fn conflicts_with_slots() -> [ModuleId; 9] {
    [
        ModuleId::AutoTool,
        ModuleId::AutoWeapon,
        ModuleId::InventoryManager,
        ModuleId::InvCleaner,
        ModuleId::Refill,
        ModuleId::AutoArmor,
        ModuleId::AutoTotem,
        ModuleId::ArmorSwitch,
        ModuleId::ChestStealer,
    ]
}

impl Module for FastScaffoldModule {
    fn on_start(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            crate::jvmti::transform_installed("ScaffoldInputPhase")
                && crate::jvmti::transform_installed("ScaffoldEdgeSafety"),
            "FastScaffold requires Minecraft 1.21.4 input and edge-safety hooks"
        );
        FIRST_PERSON.store(first_person_view(), Ordering::Relaxed);
        self.warn_if_silent_unavailable();
        self.update_bridge()?;
        self.publish_visual_silent();
        crate::camera_bridge::set_fast_scaffold_active(true)
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
         
        let _ = rotation::restore_body_for(BodyAimOwner::FastScaffold);
        BODY_HELD.store(false, Ordering::Relaxed);
        crate::graphic::visual_silent::set_owner(
            crate::graphic::visual_silent::FAST_SCAFFOLD,
            false,
        );
        self.release_aim();
        crate::camera_bridge::set_fast_scaffold_active(false)?;
        publish_shadow(None)
    }

    fn on_input_phase(&self) -> anyhow::Result<()> {
         
         
         
         
        if BODY_HELD.swap(false, Ordering::Relaxed) {
            let _ = rotation::restore_body_for(BodyAimOwner::FastScaffold);
        }
        FIRST_PERSON.store(first_person_view(), Ordering::Relaxed);
         
         
        self.publish_visual_silent();
        self.update_bridge()?;
         
         
        self.sync_aim()?;
        publish_shadow(
            self.wire_aim()
                .filter(|_| self.silent_aim() && body_aim_available()),
        )?;
        let bridge = crate::camera_bridge::fast_scaffold_bridge()
            .ok_or_else(|| anyhow::anyhow!("Fast Scaffold bridge unavailable"))?;
        let mut env = mapping().get_env()?;
        let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
        env.call_static_method(&class, "beforeInput", "()V", &[])?;
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        self.warn_if_silent_unavailable();
        self.update_bridge()
    }

    fn on_render(&self) {
        if let Err(error) = self.update_bridge() {
            log::warn!("FastScaffold settings/gate failed; stopping FastScaffold only: {error}");
            if let Err(stop_error) = crate::camera_bridge::set_fast_scaffold_active(false) {
                log::warn!("FastScaffold stop failed: {stop_error}");
            }
        }
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_scaffold_ships_only_the_controls_that_change_its_behaviour() {
        let module = FastScaffoldModule::new();
        assert_eq!(module.module.id, ModuleId::FastScaffold);
        assert_eq!(module.module.category, ModuleCategory::World);
        assert!(!module.module.enabled);
        assert_eq!(
            module
                .module
                .settings
                .iter()
                .map(ModuleSetting::name)
                .collect::<Vec<_>>(),
            [
                "Technique",
                "Expand Length",
                "Tower",
                "Aim Speed",
                "Delay",
                "Min Distance",
                "Auto Block",
                "Safe Walk",
                "Silent Aim",
                "Visual Silent"
            ]
        );
         
         
        assert!(module.silent_aim());
         
         
        assert!(module.toggle("Visual Silent"));
        assert_eq!(TECHNIQUES, ["Normal", "Expand"]);
        assert_eq!(TOWERS, ["Off", "Motion"]);
        assert_eq!(module.technique(), 0);
        assert_eq!(module.tower(), 0);
        assert_eq!(module.aim_speed(), DEFAULT_AIM_SPEED);
        assert_eq!(module.expand_length(), DEFAULT_EXPAND_LENGTH as i32);
        assert_eq!(module.delay(), DEFAULT_DELAY as i32);
        assert_eq!(module.min_distance(), DEFAULT_MIN_DISTANCE);
         
        let mut broken = FastScaffoldModule::new();
        broken
            .module
            .get_setting_mut("Aim Speed")
            .expect("Aim Speed is a setting")
            .set_slider_value(f32::NAN);
        assert!(broken.aim_speed().is_finite());
        assert!(
            (AIM_SPEED_RANGE.0..=AIM_SPEED_RANGE.1).contains(&broken.aim_speed()),
            "{}",
            broken.aim_speed()
        );
    }

    #[test]
    fn fast_scaffold_yields_to_the_modules_it_would_fight_with() {
        for id in [
            ModuleId::Freecam,
            ModuleId::Freelook,
            ModuleId::Fly,
            ModuleId::Speed,
            ModuleId::Strafe,
            ModuleId::Nuker,
        ] {
            assert!(camera_conflicts().contains(&id), "{id:?} owns the camera");
        }
        for id in [
            ModuleId::KillAura,
            ModuleId::MobAura,
            ModuleId::SilentAura,
            ModuleId::Aimbot,
            ModuleId::AutoCrystal,
        ] {
            assert!(aim_conflicts().contains(&id), "{id:?} aims silently");
        }
         
        assert!(!camera_conflicts().contains(&ModuleId::SilentAura));
         
        assert!(conflicts_with_slots().contains(&ModuleId::InventoryManager));
        assert!(conflicts_with_slots().contains(&ModuleId::AutoTotem));
        assert!(!camera_conflicts().contains(&ModuleId::FastScaffold));
        assert!(!aim_conflicts().contains(&ModuleId::FastScaffold));
        assert!(!conflicts_with_slots().contains(&ModuleId::FastScaffold));
    }

     
     
     
    #[test]
    fn fast_scaffold_refuses_a_body_aim_without_the_camera_restore() {
         
         
        let error = arm_body(0.0, 0.0).expect_err("no camera restore, no body aim");
        assert!(
            error.to_string().contains("first person"),
            "unexpected refusal: {error}"
        );
        assert!(
            !BODY_HELD.load(Ordering::Relaxed),
            "a refused arm leaves nothing held"
        );
    }
}
