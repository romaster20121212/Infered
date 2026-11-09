use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

 
 
 
 
 
 
 
pub struct BlockGroup {
     
    pub label: &'static str,
     
    pub ids: &'static [&'static str],
}

 
 
 
pub const STORAGE_BLOCK_IDS: &[&str] = &[
    "minecraft:chest",
    "minecraft:trapped_chest",
    "minecraft:ender_chest",
    "minecraft:barrel",
    "minecraft:shulker_box",
    "minecraft:hopper",
    "minecraft:dispenser",
    "minecraft:dropper",
    "minecraft:furnace",
    "minecraft:blast_furnace",
    "minecraft:smoker",
    "minecraft:brewing_stand",
    "minecraft:decorated_pot",
];

 
pub const BLOCK_GROUPS: &[BlockGroup] = &[
    BlockGroup {
         
         
        label: "Ores",
        ids: crate::module::render::xray::ORE_BLOCK_IDS,
    },
    BlockGroup {
        label: "Containers",
        ids: &[
            "minecraft:chest",
            "minecraft:trapped_chest",
            "minecraft:ender_chest",
            "minecraft:barrel",
            "minecraft:shulker_box",
            "minecraft:hopper",
            "minecraft:dispenser",
            "minecraft:dropper",
            "minecraft:furnace",
            "minecraft:blast_furnace",
            "minecraft:smoker",
            "minecraft:brewing_stand",
            "minecraft:decorated_pot",
        ],
    },
    BlockGroup {
        label: "Rare",
        ids: &[
            "minecraft:spawner",
            "minecraft:trial_spawner",
            "minecraft:vault",
            "minecraft:beacon",
            "minecraft:enchanting_table",
            "minecraft:end_portal_frame",
            "minecraft:respawn_anchor",
            "minecraft:lodestone",
            "minecraft:conduit",
            "minecraft:budding_amethyst",
            "minecraft:reinforced_deepslate",
            "minecraft:sculk_sensor",
            "minecraft:calibrated_sculk_sensor",
            "minecraft:suspicious_sand",
            "minecraft:suspicious_gravel",
        ],
    },
];

 
 
pub fn all_catalog_ids() -> impl Iterator<Item = &'static str> {
    BLOCK_GROUPS
        .iter()
        .flat_map(|group| group.ids.iter().copied())
}

impl BlockGroup {
    fn ids(&self) -> impl Iterator<Item = &'static str> {
        self.ids.iter().copied()
    }
}

 
pub fn group_label_of(id: &str) -> Option<&'static str> {
    BLOCK_GROUPS
        .iter()
        .find(|group| group.ids.contains(&id))
        .map(|group| group.label)
}

 
pub fn display_name(id: &str) -> String {
    id.strip_prefix("minecraft:")
        .unwrap_or(id)
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

 
 
 
pub fn normalize_id(input: &str) -> Option<String> {
    let id = input.trim().to_lowercase();
    if id.is_empty() {
        return None;
    }
    let normalized = match id.split_once(':') {
        Some((namespace, path)) => {
            if namespace.is_empty() || path.is_empty() {
                return None;
            }
            id
        }
        None => format!("minecraft:{id}"),
    };
    Some(normalized)
}

 
 
 
 
pub fn all_blocks() -> &'static [String] {
    static ALL_BLOCKS: OnceLock<Vec<String>> = OnceLock::new();
    ALL_BLOCKS.get_or_init(|| match crate::mapping::chunk::registered_block_ids() {
        Ok(ids) => ids,
        Err(error) => {
            log::debug!("Block ESP: registry enumeration unavailable: {error}");
            all_catalog_ids().map(str::to_string).collect()
        }
    })
}

 
 
pub fn is_known_block(id: &str) -> bool {
    static KNOWN: OnceLock<Mutex<Option<HashSet<String>>>> = OnceLock::new();
    let cache = KNOWN.get_or_init(|| Mutex::new(None));
    let Ok(mut guard) = cache.lock() else {
        return false;
    };
    if guard.is_none() {
        *guard = Some(all_blocks().iter().cloned().collect());
    }
    guard.as_ref().is_some_and(|set| set.contains(id))
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockShapeMode {
    Lines,
    Sides,
    Both,
}

impl BlockShapeMode {
     
    pub const OPTIONS: [&'static str; 3] = ["Lines", "Sides", "Both"];

    pub fn from_index(index: usize) -> BlockShapeMode {
        match index {
            0 => BlockShapeMode::Lines,
            1 => BlockShapeMode::Sides,
            _ => BlockShapeMode::Both,
        }
    }

    pub fn index(self) -> usize {
        match self {
            BlockShapeMode::Lines => 0,
            BlockShapeMode::Sides => 1,
            BlockShapeMode::Both => 2,
        }
    }

     
    pub fn lines(self) -> bool {
        !matches!(self, BlockShapeMode::Sides)
    }

     
    pub fn sides(self) -> bool {
        !matches!(self, BlockShapeMode::Lines)
    }
}

 
 
 
 
 
#[derive(Debug)]
pub struct BlockEspModule {
    pub module: ModuleData,
}

impl BlockEspModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::BlockEsp,
                description: "Renders specified blocks through walls.".to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::BlockList {
                        name: "Blocks".to_string(),
                        value: vec![
                            "minecraft:diamond_ore".to_string(),
                            "minecraft:deepslate_diamond_ore".to_string(),
                            "minecraft:ancient_debris".to_string(),
                            "minecraft:spawner".to_string(),
                        ],
                    },
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: [0.88, 0.18, 0.2, 1.0],
                    },
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: 128.0,
                        min: 8.0,
                        max: 512.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Distance".to_string(),
                        value: true,
                    },
                     
                     
                    ModuleSetting::Toggle {
                        name: "Tracers".to_string(),
                        value: false,
                    },
                     
                    ModuleSetting::Toggle {
                        name: "Shade".to_string(),
                        value: false,
                    },
                    ModuleSetting::Slider {
                        name: "Shade Opacity".to_string(),
                        value: 0.2,
                        min: 0.0,
                        max: 1.0,
                    },
                ],
            },
        }
    }
}

impl Module for BlockEspModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
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

    #[test]
    fn every_catalog_id_is_normalized_and_unique() {
        let mut seen = HashSet::new();
        for id in all_catalog_ids() {
            assert_eq!(Some(id), normalize_id(id).as_deref(), "{id} not normalized");
            assert!(
                id.starts_with("minecraft:"),
                "{id} must carry the minecraft namespace"
            );
            assert!(!id.contains(' '), "{id} must not contain spaces");
            assert!(seen.insert(id), "{id} appears twice in the catalog");
        }
    }

    #[test]
    fn display_name_prettifies_ids() {
        assert_eq!(display_name("minecraft:diamond_ore"), "Diamond Ore");
        assert_eq!(
            display_name("minecraft:deepslate_redstone_ore"),
            "Deepslate Redstone Ore"
        );
        assert_eq!(display_name("chest"), "Chest");
    }

    #[test]
    fn normalize_id_adds_the_namespace_and_rejects_junk() {
        assert_eq!(
            normalize_id("  Diamond_Ore ").as_deref(),
            Some("minecraft:diamond_ore")
        );
        assert_eq!(
            normalize_id("custom:block").as_deref(),
            Some("custom:block")
        );
        assert!(normalize_id("").is_none());
        assert!(normalize_id("   ").is_none());
        assert!(normalize_id("minecraft:").is_none());
        assert!(normalize_id(":chest").is_none());
    }

    #[test]
    fn group_lookup_finds_members_but_not_unknowns() {
        assert_eq!(group_label_of("minecraft:spawner"), Some("Rare"));
        assert_eq!(group_label_of("minecraft:diamond_ore"), Some("Ores"));
        assert_eq!(group_label_of("minecraft:obsidian"), None);
    }

    #[test]
    fn shape_mode_options_round_trip() {
        for (index, name) in BlockShapeMode::OPTIONS.iter().enumerate() {
            assert_eq!(BlockShapeMode::from_index(index).index(), index);
            assert_eq!(
                BlockShapeMode::OPTIONS[BlockShapeMode::from_index(index).index()],
                *name
            );
        }
        assert!(BlockShapeMode::Both.lines() && BlockShapeMode::Both.sides());
        assert!(BlockShapeMode::Lines.lines() && !BlockShapeMode::Lines.sides());
        assert!(!BlockShapeMode::Sides.lines() && BlockShapeMode::Sides.sides());
    }

    #[test]
    fn tracer_and_shade_settings_default_off() {
        let module = BlockEspModule::new();
        let toggle = |name: &str| {
            module
                .module
                .get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
        };
        assert_eq!(toggle("Tracers"), Some(false));
        assert_eq!(toggle("Shade"), Some(false));
         
        assert!(module.module.get_setting("Connected").is_none());
        let opacity = module
            .module
            .get_setting("Shade Opacity")
            .and_then(ModuleSetting::get_slider_value);
        assert_eq!(opacity, Some(0.2));
    }
}
