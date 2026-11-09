 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::LivingEntityRef;
use crate::module::combat::{crosshair_on_target, nearest_combatant};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;

 
 
const FULL_DRAW_TICKS: i32 = 20;

#[derive(Debug)]
pub struct AutoBowModule {
    pub module: ModuleData,
}

impl AutoBowModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoBow,
                description: "Releases a fully drawn bow when the crosshair is on a player"
                    .to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Range".to_string(),
                    value: 32.0,
                    min: 5.0,
                    max: 64.0,
                }],
            },
        }
    }

    fn range(&self) -> f64 {
        self.module
            .get_setting("Range")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(32.0)
            .clamp(1.0, 128.0) as f64
    }

    fn tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        let Some(player) = minecraft.player()? else {
            return Ok(());
        };
        if minecraft.current_screen().is_open()
            || !player.is_alive()?
            || player.container_menu()?.container_id()? != 0
        {
            return Ok(());
        }
        if !player.is_using_item()? {
            return Ok(());
        }
        let use_key = minecraft.options()?.key_use()?;
         
         
        if !use_key.is_down()? {
            return Ok(());
        }
        let inventory = player.get_inventory()?;
        let held = inventory.get_item(inventory.get_selected()?)?;
        if held.is_empty()? || !is_bow(&held.get_item()?.registry_name()?) {
            return Ok(());
        }
        if player.get_ticks_using_item()? < FULL_DRAW_TICKS {
            return Ok(());
        }
        let Some(target) = nearest_combatant(&player, self.range(), true, false)? else {
            return Ok(());
        };
        if !crosshair_on_target(&player, &target, self.range())? {
            return Ok(());
        }
        use_key.set_down(false)?;
        Ok(())
    }
}

impl Default for AutoBowModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoBowModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        self.tick()
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

fn is_bow(registry_name: &str) -> bool {
    registry_name == "minecraft:bow"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_identity_and_binding() {
        let module = AutoBowModule::new();
        assert_eq!(module.module.id, ModuleId::AutoBow);
        assert_eq!(module.module.id.display_name(), "AutoBow");
        assert_eq!(module.module.category, ModuleCategory::Combat);
        assert_eq!(module.range(), 32.0);
        assert_eq!(module.module.settings.len(), 1);
    }

    #[test]
    fn bows_are_not_crossbows() {
        assert!(is_bow("minecraft:bow"));
        assert!(!is_bow("minecraft:crossbow"));
        assert!(!is_bow("minecraft:bow_of_doom"));
    }

    #[test]
    fn a_full_draw_is_one_second() {
        assert_eq!(FULL_DRAW_TICKS, 20);
    }
}
