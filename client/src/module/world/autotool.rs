use crate::mapping::inventory::Inventory;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
 
fn tool_tier(name: &str) -> Option<u8> {
    let bare = name.strip_prefix("minecraft:").unwrap_or(name);
    let tier = if bare.contains("netherite") {
        5
    } else if bare.contains("diamond") {
        4
    } else if bare.contains("iron") {
        3
    } else if bare.contains("golden") {
        2
    } else if bare.contains("stone") {
        1
    } else {
        0
    };
    let is_tool = ["_pickaxe", "_axe", "_shovel", "_hoe", "_sword"]
        .iter()
        .any(|suffix| bare.ends_with(suffix));
    is_tool.then_some(tier)
}

 
fn suits_block(item_name: &str, block_name: &str) -> bool {
    let block = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    let pickaxe_blocks = [
        "stone",
        "cobblestone",
        "ore",
        "deepslate",
        "granite",
        "diorite",
        "andesite",
        "netherrack",
        "obsidian",
        "iron_block",
        "gold_block",
        "diamond_block",
        "copper",
        "coal_block",
        "bricks",
        "concrete",
    ];
    let axe_blocks = [
        "log",
        "wood",
        "planks",
        "fence",
        "stairs",
        "slab",
        "door",
        "chest",
        "barrel",
        "crafting_table",
        "bookshelf",
        "oak",
        "spruce",
        "birch",
        "jungle",
        "acacia",
        "dark_oak",
        "mangrove",
        "cherry",
        "bamboo",
    ];
    let shovel_blocks = [
        "dirt",
        "grass_block",
        "sand",
        "gravel",
        "clay",
        "snow",
        "mud",
        "soul_",
    ];
    let hoe_blocks = [
        "sculk",
        "hay_block",
        "moss_block",
        "leaves",
        "wart_block",
        "sponge",
    ];

    let pick = pickaxe_blocks.iter().any(|b| block.contains(b));
    let axe = axe_blocks.iter().any(|b| block.contains(b));
    let shovel = shovel_blocks.iter().any(|b| block.contains(b));
    let hoe = hoe_blocks.iter().any(|b| block.contains(b));
    let bare = block.ends_with("wool") || block.contains("leaves");

    if pick {
        item_name.contains("pickaxe")
    } else if axe {
        item_name.contains("axe") && !item_name.contains("pickaxe")
    } else if shovel {
        item_name.contains("shovel")
    } else if hoe {
        item_name.contains("hoe")
    } else if bare {
        item_name.contains("hoe") || item_name.contains("sword")
    } else {
        false
    }
}

 
 
 
#[derive(Debug)]
pub struct AutoToolModule {
    pub module: ModuleData,
    state: Mutex<()>,
}

impl AutoToolModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoTool,
                description: "Automatically selects the appropriate tool".to_string(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Toggle {
                    name: "Prefer Tier".to_string(),
                    value: true,
                }],
            },
            state: Mutex::new(()),
        }
    }

     
     
    fn best_slot(
        &self,
        inventory: &Inventory,
        block_name: &str,
        prefer_tier: bool,
    ) -> anyhow::Result<Option<i32>> {
        let selected = inventory.get_selected()?;
        let held = inventory.get_item(selected)?;
        let held_name = if held.is_empty()? {
            String::new()
        } else {
            held.get_item()?.registry_name()?
        };

        let mut best: Option<(i32, bool, u8)> = None;  
        for slot in 0..9 {
            let stack = inventory.get_item(slot)?;
            if stack.is_empty()? {
                continue;
            }
            let name = stack.get_item()?.registry_name()?;
            let suits = suits_block(&name, block_name);
            let tier = tool_tier(&name).unwrap_or(0);
            let better = match best {
                None => suits || prefer_tier && tier > 0,
                Some((_, b_suits, b_tier)) => suits && (!b_suits || tier > b_tier),
            };
            if better {
                best = Some((slot, suits, tier));
            }
        }

        let Some((slot, suits, _)) = best else {
            return Ok(None);
        };
         
         
        if slot == selected {
            return Ok(None);
        }
        let held_suits = suits_block(&held_name, block_name);
        if held_suits
            && (!suits
                || tool_tier(&held_name).unwrap_or(0)
                    >= tool_tier(&inventory.get_item(slot)?.get_item()?.registry_name()?)
                        .unwrap_or(0))
        {
            return Ok(None);
        }
        Ok(Some(slot))
    }
}

impl Module for AutoToolModule {
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
         
        let left_down = !crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed)
            && crate::graphic::input::MOUSE_STATE
                .lock()
                .map(|mouse| mouse.left_down)
                .unwrap_or(false);
        if !left_down {
            return Ok(());
        }
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        let Some(hit) = minecraft().hit_block()? else {
            return Ok(());
        };
        let block_name = player_world_block_name(&hit)?;
        if block_name.is_empty() {
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        let prefer_tier = self
            .module
            .get_setting("Prefer Tier")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true);
        if let Some(slot) = self.best_slot(&inventory, &block_name, prefer_tier)? {
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

 
fn player_world_block_name(pos: &crate::mapping::math::BlockPos) -> anyhow::Result<String> {
    let Some(world) = minecraft().world()? else {
        return Ok(String::new());
    };
    world.block_registry_name(pos)
}
