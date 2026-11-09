 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::{EndCrystal, Entity, EntityRef};
use crate::mapping::{MappedObject, MinecraftClassType};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{mapping, minecraft};
use jni::objects::JObject;
use jni::sys::jobject;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

 
 
static LAST_HIT: AtomicI32 = AtomicI32::new(-1);

 
 
static FIRST_HIDE_REPORTED: AtomicBool = AtomicBool::new(false);

 
 
 
pub(crate) extern "system" fn note_attack(
    _env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    target: jobject,
) {
    if target.is_null() {
        return;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> anyhow::Result<()> {
        let target = unsafe { JObject::from_raw(target) };
        let id = mapping()
            .call_method(MinecraftClassType::Entity, &target, "getId", &[])?
            .i()?;
        record_hit(id);
        Ok(())
    }));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            log::debug!("Crystal Optimizer: could not read the attacked entity: {error}");
        }
        Err(_) => log::debug!("Crystal Optimizer: the attack hook panicked"),
    }
     
     
    if let Ok(env) = mapping().get_env() {
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }
}

 
fn record_hit(id: i32) {
    LAST_HIT.store(id, Ordering::Release);
}

 
 
fn take_hit() -> Option<i32> {
    let id = LAST_HIT.swap(-1, Ordering::AcqRel);
    (id >= 0).then_some(id)
}

 
 
 
#[derive(Debug)]
pub struct CrystalOptimizerModule {
    pub module: ModuleData,
}

impl CrystalOptimizerModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::CrystalOptimizer,
                description: "Takes a crystal out of your world the moment you hit it, instead of waiting for the server's kill to come back".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Toggle {
                    name: "Require Reach".to_string(),
                    value: true,
                }],
            },
        }
    }

     
     
     
     
    fn require_reach(&self) -> bool {
        self.module
            .get_setting("Require Reach")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true)
    }
}

impl Module for CrystalOptimizerModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
        LAST_HIT.store(-1, Ordering::Release);
        if let Err(error) = crate::camera_bridge::set_crystal_optimizer_enabled(true) {
            log::warn!("Crystal Optimizer: {error}; the module stays inert on this build");
            return Ok(());
        }
        if !crate::jvmti::transform_installed(
            crate::jvmti::CRYSTAL_OPTIMIZER_TRANSFORM_PARAMS.log_label,
        ) {
            log::warn!(
                "Crystal Optimizer: the attack hook is not installed on this build, so hits \
                 cannot be seen"
            );
        }
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        LAST_HIT.store(-1, Ordering::Release);
        if let Err(error) = crate::camera_bridge::set_crystal_optimizer_enabled(false) {
            log::debug!("Crystal Optimizer: {error}");
        }
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(id) = take_hit() else {
            return Ok(());
        };
        let Some(world) = minecraft().world()? else {
            return Ok(());
        };
         
         
        let Some(entity) = world.get_entity(id)? else {
            return Ok(());
        };
        if !entity.instance_of::<EndCrystal>() {
            return Ok(());
        }
        if entity.is_removed()? {
            return Ok(());
        }
        let reach_ok = match minecraft().player()? {
            Some(player) => within_reach(&player, &entity)?,
            None => false,
        };
        if !may_hide(true, false, reach_ok, self.require_reach()) {
            log::debug!("Crystal Optimizer: hit outside interaction reach, crystal left alone");
            return Ok(());
        }
        entity.discard()?;
        if !FIRST_HIDE_REPORTED.swap(true, Ordering::AcqRel) {
            log::info!(
                "Crystal Optimizer: hit crystals are removed from the client's world immediately"
            );
        } else {
            log::debug!("Crystal Optimizer: hid the crystal entity {id}");
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

 
 
 
fn may_hide(is_crystal: bool, removed: bool, reach_ok: bool, require_reach: bool) -> bool {
    is_crystal && !removed && (reach_ok || !require_reach)
}

 
 
fn within_reach(player: &LocalPlayer, entity: &Entity) -> anyhow::Result<bool> {
    let eye = player.get_eye_position()?;
    let (min, max) = super::target_box(entity)?;
    Ok(
        super::point_box_distance([eye.x(), eye.y(), eye.z()], min, max)
            <= super::interaction_reach(player),
    )
}

#[cfg(test)]
mod tests {
    use super::{may_hide, record_hit, take_hit, CrystalOptimizerModule};
    use crate::module::{Module, ModuleId};

     
     
    #[test]
    fn a_hit_is_taken_once() {
        assert_eq!(take_hit(), None);
        record_hit(42);
        assert_eq!(take_hit(), Some(42));
        assert_eq!(take_hit(), None);
    }

     
     
    #[test]
    fn only_a_reachable_crystal_is_taken_out() {
        assert!(may_hide(true, false, true, true));
        assert!(!may_hide(true, false, false, true));
         
        assert!(may_hide(true, false, false, false));
         
         
        assert!(!may_hide(false, false, true, true));
        assert!(!may_hide(true, true, true, true));
    }

     
    #[test]
    fn the_module_ships_with_its_reach_guard() {
        let mut module = CrystalOptimizerModule::new();
        let data = module.get_module_data_mut();
        assert_eq!(data.id, ModuleId::CrystalOptimizer);
        assert_eq!(data.id.display_name(), "Crystal Optimizer");
        assert!(!data.enabled);
        let setting = data
            .get_setting("Require Reach")
            .expect("Require Reach is missing");
        assert_eq!(setting.get_toggle_value(), Some(true));
        data.get_setting_mut("Require Reach")
            .expect("checked above")
            .set_toggle_value(false);
        assert!(!module.require_reach());
    }
}
