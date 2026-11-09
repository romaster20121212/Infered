use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

#[derive(Debug)]
pub struct AutoWeaponModule {
    pub module: ModuleData,
}

impl AutoWeaponModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoWeapon,
                description: "Automatically switches to the best weapon before attacking"
                    .to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for AutoWeaponModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        if crate::module::world::autoeat::is_eating() {
            return Ok(());
        }
        if !crate::graphic::input::MOUSE_STATE
            .lock()
            .map(|mouse| mouse.left_down)
            .unwrap_or(false)
        {
            return Ok(());
        }
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        let inventory = player.get_inventory()?;
        let current = inventory.get_selected()?;
        let mut best = None;
        let mut best_score = -1_i32;

        for slot in 0..9 {
            let stack = inventory.get_item(slot)?;
            if stack.is_empty()? {
                continue;
            }
            let name = stack.get_item()?.registry_name()?;
            let score = weapon_score(&name);
            if score > best_score {
                best_score = score;
                best = Some(slot);
            }
        }
        if let Some(slot) = best.filter(|slot| *slot != current) {
            inventory.set_selected(slot)?;
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

fn weapon_score(id: &str) -> i32 {
    const TIERS: &[(&str, i32)] = &[
        ("netherite_sword", 120),
        ("diamond_sword", 110),
        ("netherite_axe", 105),
        ("diamond_axe", 95),
        ("iron_sword", 80),
        ("iron_axe", 75),
        ("stone_sword", 60),
        ("stone_axe", 55),
        ("golden_sword", 40),
        ("golden_axe", 35),
        ("wooden_sword", 20),
        ("wooden_axe", 15),
    ];
    TIERS
        .iter()
        .find(|(name, _)| id.ends_with(name))
        .map(|(_, score)| *score)
        .unwrap_or(-1)
}
