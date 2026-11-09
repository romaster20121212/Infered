 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use std::sync::Mutex;

use crate::friends::ignores_entity;
use crate::mapping::entity::player::{LocalPlayer, Player};
use crate::mapping::entity::{Entity, EntityRef, LivingEntityRef};
use crate::mapping::inventory::Inventory;
use crate::mapping::math::Vec3;
use crate::mapping::MappedObject;
use crate::module::combat::interaction::{self, Action};
use crate::module::combat::rotation::{self, BodyAimOwner, Rotation};
use crate::module::combat::{
    crosshair_on_target, interaction_reach, shadow_crosshair_on_target, within_reach_up_to,
};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};

 
 
 
const DEFAULT_MIN_FALL: f64 = 5.0;
const MIN_FALL_RANGE: (f32, f32) = (1.0, 20.0);

 
const DEFAULT_AIM_SPEED: f32 = 100_000.0;
const AIM_SPEED_RANGE: (f32, f32) = (10.0, 100_000.0);
const DEFAULT_TARGET_RANGE: f32 = 3.0;
const TARGET_RANGE: (f32, f32) = (1.0, 6.0);

#[derive(Clone, Copy)]
struct Settings {
    min_fall: f64,
    aim_speed: f32,
    target_range: f64,
}

impl Settings {
    fn read(data: &ModuleData) -> Self {
        let slider = |name: &str, fallback: f32, range: (f32, f32)| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_slider_value)
                .filter(|value| value.is_finite())
                .unwrap_or(fallback)
                .clamp(range.0, range.1)
        };
        Self {
            min_fall: f64::from(slider(
                "Min Fall Distance",
                DEFAULT_MIN_FALL as f32,
                MIN_FALL_RANGE,
            )),
            aim_speed: slider("Aim Speed", DEFAULT_AIM_SPEED, AIM_SPEED_RANGE),
            target_range: f64::from(slider("Target Range", DEFAULT_TARGET_RANGE, TARGET_RANGE)),
        }
    }
}

 
#[derive(Debug)]
struct Runtime {
     
    peak_fall: f64,
     
    hit_targets: Vec<i32>,
     
    descending: bool,
     
    descent_start_y: Option<f64>,
     
    aiming: bool,
    label: &'static str,
}

impl Runtime {
    const fn new() -> Runtime {
        Runtime {
            peak_fall: 0.0,
            hit_targets: Vec::new(),
            descending: false,
            descent_start_y: None,
            aiming: false,
            label: "IDLE",
        }
    }
}

static RUNTIME: Mutex<Runtime> = Mutex::new(Runtime::new());
static AUTO_RUNTIME: Mutex<Runtime> = Mutex::new(Runtime::new());

 
#[derive(Debug)]
pub struct SilentMaceModule {
    pub module: ModuleData,
}

 
#[derive(Debug)]
pub struct AutoMaceModule {
    pub module: ModuleData,
}

pub fn auto_mace_hud_line() -> Option<String> {
    let handle = client().modules.get(ModuleId::AutoMace)?;
    let module = handle.lock().ok()?;
    if !module.get_module_data().enabled {
        return None;
    }
    let rt = AUTO_RUNTIME.lock().ok()?;
    Some(if rt.peak_fall > 0.0 {
        format!("AutoMace · {} · {:.1}b", rt.label, rt.peak_fall)
    } else {
        format!("AutoMace · {}", rt.label)
    })
}

fn mace_module_data(id: ModuleId, title: &str) -> ModuleData {
    ModuleData {
        id,
        description: format!("{title} aims at players once per falling segment while a mace is held. Ignores attack cooldown."),
        category: ModuleCategory::Combat,
        key_bind: KeyboardKey::KeyNone,
        enabled: false,
        settings: vec![
            ModuleSetting::Slider { name: "Min Fall Distance".into(), value: DEFAULT_MIN_FALL as f32, min: MIN_FALL_RANGE.0, max: MIN_FALL_RANGE.1 },
            ModuleSetting::Slider { name: "Target Range".into(), value: DEFAULT_TARGET_RANGE, min: TARGET_RANGE.0, max: TARGET_RANGE.1 },
            ModuleSetting::Slider { name: "Aim Speed".into(), value: DEFAULT_AIM_SPEED, min: AIM_SPEED_RANGE.0, max: AIM_SPEED_RANGE.1 },
        ],
    }
}

impl SilentMaceModule {
    pub fn new() -> Self {
        Self {
            module: mace_module_data(ModuleId::SilentMace, "Silent Mace"),
        }
    }
}

impl AutoMaceModule {
    pub fn new() -> Self {
        Self {
            module: mace_module_data(ModuleId::AutoMace, "Auto Mace"),
        }
    }
}

impl Module for SilentMaceModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *RUNTIME.lock().unwrap_or_else(|p| p.into_inner()) = Runtime::new();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.release();
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.release();
        Ok(())
    }

     
     
     
    fn on_input_phase(&self) -> anyhow::Result<()> {
        self.step()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        if !crate::jvmti::crystal_input_hook_installed() {
            self.step()?;
        }
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

impl Module for AutoMaceModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *AUTO_RUNTIME.lock().unwrap_or_else(|p| p.into_inner()) = Runtime::new();
        Ok(())
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        self.release();
        Ok(())
    }
    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.release();
        Ok(())
    }
    fn on_input_phase(&self) -> anyhow::Result<()> {
        self.step()
    }
    fn on_tick(&self) -> anyhow::Result<()> {
        if !crate::jvmti::crystal_input_hook_installed() {
            self.step()?;
        }
        Ok(())
    }
    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }
    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

impl SilentMaceModule {
     
     
    fn step(&self) -> anyhow::Result<()> {
        self.step_mode(false)
    }

    fn step_mode(&self, visible: bool) -> anyhow::Result<()> {
        if !client().modules.is_active() {
            self.release_mode(visible);
            return Ok(());
        }
        let mc = minecraft();
        let settings = Settings::read(&self.module);
        let Some(player) = mc.player()? else {
            self.release_mode(visible);
            return Ok(());
        };
         
        if mc.current_screen().is_open() {
            return Ok(());
        }
        let grounded = player.on_ground()?;
        let vertical_speed = player.get_delta_movement()?.y();
        let y = player.get_position()?.y();
        let mut rt = if visible {
            AUTO_RUNTIME.lock().unwrap_or_else(|p| p.into_inner())
        } else {
            RUNTIME.lock().unwrap_or_else(|p| p.into_inner())
        };
        let descending = !grounded && vertical_speed < 0.0;
        if grounded || !descending {
             
             
            rt.peak_fall = 0.0;
            rt.hit_targets.clear();
            rt.descent_start_y = None;
        } else {
            if !rt.descending || rt.descent_start_y.is_none() {
                rt.descent_start_y = Some(y);
            }
            rt.peak_fall = rt.descent_start_y.map_or(0.0, |start| (start - y).max(0.0));
        }
        rt.descending = descending;
        if !fall_far_enough(rt.peak_fall, settings.min_fall) {
            rt.label = "IDLE";
            self.release_aim(&mut rt, visible);
            return Ok(());
        }

         
        if !holds_mace(&player)? {
            rt.label = "NO MACE";
            self.release_aim(&mut rt, visible);
            return Ok(());
        }

        let module_id = if visible {
            ModuleId::AutoMace
        } else {
            ModuleId::SilentMace
        };
        let Some(target) = pick_target(&player, &rt.hit_targets, settings.target_range, module_id)?
        else {
            rt.label = "FALLING";
            self.release_aim(&mut rt, visible);
            return Ok(());
        };
        rt.label = "TARGET";

         
         
         
         
        let desired = aim_rotation(&player, &target)?;
        if visible {
            rotation::aim_real_mouse(desired, settings.aim_speed);
        } else {
            rotation::aim_silent(desired, settings.aim_speed);
        }
        if visible {
            rt.aiming = true;
            if within_reach_up_to(&player, &target, settings.target_range)?
                && crosshair_on_target(
                    &player,
                    &target,
                    interaction_reach(&player).min(settings.target_range),
                )?
            {
                if interaction::take(&player, Action::Entity) {
                    if let Some(game_mode) = mc.game_mode()? {
                        let target_id = target.id()?;
                        if player.is_using_item().unwrap_or(false) {
                            let _ = game_mode.release_using_item(&player);
                        }
                        game_mode.attack(&player, &target)?;
                        if !rt.hit_targets.contains(&target_id) {
                            rt.hit_targets.push(target_id);
                        }
                        let _ = player.swing();
                        rt.label = "HIT";
                    }
                }
            }
            return Ok(());
        }
        let previous_wire = rotation::silent_rotation();
        let wire = rotation::silent_rotation();
        rt.aiming = true;

        let Some(wire) = wire else {
            return Ok(());
        };
        if !within_reach_up_to(&player, &target, settings.target_range)? {
            return Ok(());
        }
        let body_aim = can_body_aim();
        if body_aim {
            if rotation::aim_body_for(BodyAimOwner::SilentMace, wire).is_err() {
                return Ok(());
            }
        } else if !previous_wire.is_some_and(|previous| {
            shadow_crosshair_on_target(
                &player,
                &target,
                previous,
                interaction_reach(&player).min(settings.target_range),
            )
            .unwrap_or(false)
        }) {
             
             
             
            return Ok(());
        }
        if !shadow_crosshair_on_target(
            &player,
            &target,
            wire,
            interaction_reach(&player).min(settings.target_range),
        )? {
            if body_aim {
                let _ = rotation::restore_body_for(BodyAimOwner::SilentMace);
            }
            return Ok(());
        }
        if !interaction::take(&player, Action::Entity) {
            if body_aim {
                let _ = rotation::restore_body_for(BodyAimOwner::SilentMace);
            }
            return Ok(());
        }
        let Some(game_mode) = mc.game_mode()? else {
            if body_aim {
                let _ = rotation::restore_body_for(BodyAimOwner::SilentMace);
            }
            return Ok(());
        };
        let target_id = target.id()?;
         
         
        if player.is_using_item().unwrap_or(false) {
            let _ = game_mode.release_using_item(&player);
        }
        game_mode.attack(&player, &target)?;
        if !rt.hit_targets.contains(&target_id) {
            rt.hit_targets.push(target_id);
        }
        let _ = player.swing();
        rt.label = "HIT";
        Ok(())
    }

    fn release_aim(&self, rt: &mut Runtime, visible: bool) {
        if rt.aiming {
            if visible {
                rotation::clear();
            } else {
                let _ = rotation::restore_body_for(BodyAimOwner::SilentMace);
                rotation::clear_silent();
            }
            rt.aiming = false;
        }
    }

     
    fn release(&self) {
        self.release_mode(false);
    }

    fn release_mode(&self, visible: bool) {
        let aiming = {
            let mut rt = if visible {
                AUTO_RUNTIME.lock().unwrap_or_else(|p| p.into_inner())
            } else {
                RUNTIME.lock().unwrap_or_else(|p| p.into_inner())
            };
            let aiming = rt.aiming;
            *rt = Runtime::new();
            aiming
        };
        if aiming {
            if visible {
                rotation::clear();
            } else {
                let _ = rotation::restore_body_for(BodyAimOwner::SilentMace);
                rotation::clear_silent();
            }
        }
    }
}

impl AutoMaceModule {
    fn step(&self) -> anyhow::Result<()> {
        SilentMaceModule {
            module: self.module.clone(),
        }
        .step_mode(true)
    }
    fn release(&self) {
        SilentMaceModule {
            module: self.module.clone(),
        }
        .release_mode(true);
    }
}

 
fn pick_target(
    player: &LocalPlayer,
    hit_targets: &[i32],
    target_range: f64,
    module_id: ModuleId,
) -> anyhow::Result<Option<Entity>> {
    let Some(world) = minecraft().world()? else {
        return Ok(None);
    };
    let eye = player.get_eye_position()?;
    let mut best: Option<(f64, Entity)> = None;
    for entity in world.get_entities()? {
        if !valid_player_target(player, &entity, module_id)
            || entity.id().is_ok_and(|id| hit_targets.contains(&id))
        {
            continue;
        }
        let Ok(distance) = entity.distance_to_sqr(eye.x(), eye.y(), eye.z()) else {
            continue;
        };
        if !distance.is_finite()
            || !crate::module::combat::within_reach_up_to(player, &entity, target_range)?
        {
            continue;
        }
        if best.as_ref().map_or(true, |(best, _)| distance < *best) {
            best = Some((distance, entity));
        }
    }
    Ok(best.map(|(_, entity)| entity))
}

 
fn valid_player_target(player: &LocalPlayer, entity: &Entity, module_id: ModuleId) -> bool {
    if !entity.instance_of::<Player>() {
        return false;
    }
    if entity.id().ok() == player.id().ok() {
        return false;
    }
    if entity.is_removed().unwrap_or(true) || entity.is_spectator().unwrap_or(true) {
        return false;
    }
    if !entity
        .as_living()
        .is_some_and(|living| living.is_alive().unwrap_or(false))
    {
        return false;
    }
    !ignores_entity(entity, module_id)
}

 
 
fn aim_rotation(player: &LocalPlayer, target: &Entity) -> anyhow::Result<Rotation> {
    let eye = player.get_eye_position()?;
    let feet = target.get_position()?;
    let height = target.bb_height()? as f64;
    Ok(Rotation::towards(
        eye,
        Vec3::new(feet.x(), feet.y() + height * 0.5, feet.z()),
    ))
}

 
 
fn holds_mace(player: &LocalPlayer) -> anyhow::Result<bool> {
    let inventory = player.get_inventory()?;
    let selected = inventory.get_selected()?;
    Ok(stack_name(&inventory, selected).is_some_and(|name| is_mace_name(&name)))
}

 
fn stack_name(inventory: &Inventory, slot: i32) -> Option<String> {
    let stack = inventory.get_item(slot).ok()?;
    if stack.is_empty().unwrap_or(true) {
        return None;
    }
    stack.get_item().ok()?.registry_name().ok()
}

fn is_mace_name(name: &str) -> bool {
    name.ends_with("mace")
}

 
fn fall_far_enough(peak_fall: f64, minimum: f64) -> bool {
    peak_fall.is_finite() && peak_fall >= minimum
}

fn can_body_aim() -> bool {
    let first_person = minecraft()
        .options()
        .and_then(|options| options.perspective())
        .map(|perspective| perspective == crate::mapping::client::options::Perspective::FirstPerson)
        .unwrap_or(true);
    first_person
        && crate::jvmti::camera_frame_hook_installed()
        && crate::jvmti::crystal_input_hook_installed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fall_gate_uses_the_configured_threshold() {
        assert!(fall_far_enough(5.0, DEFAULT_MIN_FALL));
        assert!(fall_far_enough(2.0, 2.0));
        assert!(!fall_far_enough(1.99, 2.0));
        assert!(!fall_far_enough(0.0, DEFAULT_MIN_FALL));
        assert!(!fall_far_enough(f64::NAN, DEFAULT_MIN_FALL));
    }

    #[test]
    fn item_names_are_recognised() {
        assert!(is_mace_name("minecraft:mace"));
        assert!(!is_mace_name("minecraft:heavy_core"));
        assert!(!is_mace_name("minecraft:iron_sword"));
    }

    #[test]
    fn module_identity_and_defaults() {
        let module = SilentMaceModule::new();
        assert_eq!(module.module.id, ModuleId::SilentMace);
        assert_eq!(module.module.id.display_name(), "Silent Mace");
        assert_eq!(module.module.category, ModuleCategory::Combat);
        assert!(!module.module.enabled);
        assert_eq!(module.module.settings.len(), 3);
        assert_eq!(Settings::read(&module.module).min_fall, DEFAULT_MIN_FALL);
        assert_eq!(Settings::read(&module.module).target_range, 3.0);
        assert_eq!(Settings::read(&module.module).aim_speed, DEFAULT_AIM_SPEED);
    }
}
