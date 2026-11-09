use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static ACTIVE: AtomicBool = AtomicBool::new(false);

 
 
 
 
 
 
pub const ORE_BLOCK_IDS: &[&str] = &[
    "minecraft:coal_ore",
    "minecraft:deepslate_coal_ore",
    "minecraft:copper_ore",
    "minecraft:deepslate_copper_ore",
    "minecraft:iron_ore",
    "minecraft:deepslate_iron_ore",
    "minecraft:gold_ore",
    "minecraft:deepslate_gold_ore",
    "minecraft:redstone_ore",
    "minecraft:deepslate_redstone_ore",
    "minecraft:lapis_ore",
    "minecraft:deepslate_lapis_ore",
    "minecraft:diamond_ore",
    "minecraft:deepslate_diamond_ore",
    "minecraft:emerald_ore",
    "minecraft:deepslate_emerald_ore",
    "minecraft:nether_quartz_ore",
    "minecraft:nether_gold_ore",
    "minecraft:ancient_debris",
];

 
pub fn is_ore_block(id: &str) -> bool {
    ORE_BLOCK_IDS.contains(&id)
}

#[derive(Debug)]
pub struct XrayModule {
    pub module: ModuleData,
    selection: Mutex<Vec<String>>,
}

impl XrayModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Xray,
                description: "Hide every block except your selected ores".to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                 
                 
                 
                settings: vec![
                    ModuleSetting::BlockList {
                        name: "Blocks".to_string(),
                        value: Self::targets().iter().map(|id| (*id).to_string()).collect(),
                    },
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                         
                         
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: 64.0,
                        min: 8.0,
                        max: 512.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Distance".to_string(),
                        value: false,
                    },
                ],
            },
            selection: Mutex::new(Vec::new()),
        }
    }
}

impl XrayModule {
     
     
    pub fn targets() -> &'static [&'static str] {
        ORE_BLOCK_IDS
    }

    pub fn is_active() -> bool {
        ACTIVE.load(Ordering::Acquire)
    }

     
    fn selected_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self
            .module
            .get_setting("Blocks")
            .and_then(ModuleSetting::get_block_list_value)
            .unwrap_or(&[])
            .iter()
            .filter_map(|id| crate::module::render::block_esp::normalize_id(id))
            .filter(|id| is_ore_block(id))
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    fn apply_selection(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            [
                "XrayRenderShape",
                "XrayOcclusion",
                "XrayBatched",
                "XrayLiquid",
                "XrayBlockEntity"
            ]
            .iter()
            .all(|label| crate::jvmti::transform_installed(label)),
            "Xray render hooks unavailable"
        );
        let ids = self.selected_ids();
        crate::camera_bridge::configure_xray(&ids, true)?;
        ACTIVE.store(true, Ordering::Release);
        *self.selection.lock().unwrap_or_else(|p| p.into_inner()) = ids;
        crate::camera_bridge::rebuild_world_render()
    }
}

impl Module for XrayModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.apply_selection()
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        ACTIVE.store(false, Ordering::Release);
        crate::camera_bridge::configure_xray(&[], false)?;
        self.selection
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
        crate::camera_bridge::rebuild_world_render()
    }
    fn on_tick(&self) -> anyhow::Result<()> {
        if !Self::is_active()
            || *self.selection.lock().unwrap_or_else(|p| p.into_inner()) != self.selected_ids()
        {
            self.apply_selection()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::render::block_esp::normalize_id;
    use std::collections::HashSet;

    #[test]
    fn selection_is_read_without_relocking_and_empty_means_empty() {
        let mut module = XrayModule::new();
        if let Some(ModuleSetting::BlockList { value, .. }) = module.module.settings.first_mut() {
            *value = vec![
                "diamond_ore".into(),
                "minecraft:diamond_ore".into(),
                "stone".into(),
            ];
        }
        assert_eq!(module.selected_ids(), vec!["minecraft:diamond_ore"]);
        if let Some(ModuleSetting::BlockList { value, .. }) = module.module.settings.first_mut() {
            value.clear();
        }
        assert!(module.selected_ids().is_empty());
    }

    #[test]
    fn ore_ids_are_normalized_and_unique() {
        let mut seen = HashSet::new();
        for id in ORE_BLOCK_IDS {
            assert_eq!(
                Some(*id),
                normalize_id(id).as_deref(),
                "{id} not normalized"
            );
            assert!(seen.insert(*id), "{id} appears twice in the ore list");
        }
    }

     
     
    #[test]
    fn only_ores_are_selectable() {
        assert!(is_ore_block("minecraft:diamond_ore"));
        assert!(is_ore_block("minecraft:ancient_debris"));
        for id in [
            "minecraft:chest",
            "minecraft:obsidian",
            "minecraft:lava",
            "minecraft:water",
            "minecraft:tnt",
            "minecraft:spawner",
            "minecraft:beacon",
            "minecraft:diamond_block",
            "minecraft:stone",
        ] {
            assert!(!is_ore_block(id), "{id} must not be selectable in Xray");
        }
    }

     
     
    #[test]
    fn default_targets_are_ores_only() {
        assert!(XrayModule::targets().iter().all(|id| is_ore_block(id)));
    }
}
