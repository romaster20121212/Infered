 

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::mapping::entity::player::{LocalPlayer, Player};
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::inventory::Inventory;
use crate::mapping::{MappedObject, MinecraftClassType};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{mapping, minecraft};

const ACTION_DELAY: Duration = Duration::from_millis(250);
const RESTORE_DELAY: Duration = Duration::from_millis(150);
const POTION_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug)]
struct SlotLease {
    player: LocalPlayer,
    previous: i32,
    selected: i32,
    restore_at: Instant,
}

#[derive(Debug, Default)]
struct State {
    lease: Option<SlotLease>,
    last_action: Option<Instant>,
    declined_target: Option<i32>,
}

#[derive(Debug)]
pub struct PracticalCombat {
    module: ModuleData,
    state: Mutex<State>,
}

impl PracticalCombat {
    fn new(id: ModuleId, description: &str, settings: Vec<ModuleSetting>) -> Self {
        Self {
            module: ModuleData {
                id,
                description: description.into(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings,
            },
            state: Mutex::new(State::default()),
        }
    }

    fn slider(&self, name: &str, fallback: f32, min: f32, max: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(ModuleSetting::get_slider_value)
            .filter(|v| v.is_finite())
            .unwrap_or(fallback)
            .clamp(min, max)
    }

    fn settle(&self, force: bool) -> anyhow::Result<bool> {
        let mut state = self.state.lock().unwrap();
        let Some(lease) = state.lease.as_ref() else {
            return Ok(false);
        };
        if !force && Instant::now() < lease.restore_at {
            return Ok(true);
        }
        if let Some(player) = minecraft().player()? {
            if player.is_same(&lease.player) {
                let inventory = player.get_inventory()?;
                if inventory.get_selected()? == lease.selected {
                    inventory.set_selected(lease.previous)?;
                }
            }
        }
        state.lease = None;
        Ok(true)
    }

    fn ready(&self) -> bool {
        let delay = if self.module.id == ModuleId::AutoPotion {
            POTION_DELAY
        } else {
            ACTION_DELAY
        };
        self.state
            .lock()
            .unwrap()
            .last_action
            .is_none_or(|at| at.elapsed() >= delay)
    }

    fn record_action(&self) {
        self.state.lock().unwrap().last_action = Some(Instant::now());
    }

    fn select(&self, player: &LocalPlayer, selected: i32, previous: i32) -> anyhow::Result<()> {
        if selected == previous {
            return Ok(());
        }
        player.get_inventory()?.set_selected(selected)?;
        self.state.lock().unwrap().lease = Some(SlotLease {
            player: player.clone(),
            previous,
            selected,
            restore_at: Instant::now() + RESTORE_DELAY,
        });
        Ok(())
    }

    fn tick(&self) -> anyhow::Result<()> {
        let mc = minecraft();
        if mc.current_screen().is_open() || crate::graphic::input::menu_open() {
            self.settle(true)?;
            return Ok(());
        }
        if self.module.id != ModuleId::ShieldBreaker && self.settle(false)? {
            return Ok(());
        }
        let Some(player) = mc.player()? else {
            return Ok(());
        };
        if !player.is_alive()? || player.container_menu()?.container_id()? != 0 {
            self.settle(true)?;
            return Ok(());
        }
        if self.module.id != ModuleId::ShieldBreaker && !self.ready() {
            return Ok(());
        }
        match self.module.id {
            ModuleId::AntiFireball => self.anti_fireball(&player),
            ModuleId::ShieldBreaker => self.shield_breaker(&player),
            ModuleId::AutoPotion => self.auto_potion(&player),
            ModuleId::AutoCobweb => self.auto_cobweb(&player),
            ModuleId::TotemRefill => self.totem_refill(&player),
            _ => Ok(()),
        }
    }

    pub(crate) fn anti_fireball(&self, player: &LocalPlayer) -> anyhow::Result<()> {
        let Some(target) = minecraft().crosshair_entity()? else {
            return Ok(());
        };
        let is_fireball =
            mapping().is_instance_of(MinecraftClassType::Fireball, target.jni_ref().as_obj())?;
        if !is_fireball
            || !target.call_method("isPickable", &[])?.z()?
            || !super::within_reach_up_to(player, &target, 3.0)?
        {
            return Ok(());
        }
        let Some(mode) = minecraft().game_mode()? else {
            return Ok(());
        };
        if !super::interaction::take(player, super::interaction::Action::Entity) {
            return Ok(());
        }
        self.record_action();
        mode.attack(player, &target)?;
        player.swing()?;
        Ok(())
    }

    pub(crate) fn shield_breaker(&self, player: &LocalPlayer) -> anyhow::Result<()> {
         
        let target = minecraft().crosshair_entity()?;
        let eligible = if let Some(target) = &target {
            !player.is_using_item()?
                && target.instance_of::<Player>()
                && !crate::friends::ignores_entity(target, ModuleId::ShieldBreaker)
                && target.as_living().is_some_and(|living| {
                    living.is_alive().unwrap_or(false) && living.is_blocking().unwrap_or(false)
                })
                && super::within_reach_up_to(player, target, 3.0)?
        } else {
            false
        };
        if !eligible {
            self.settle(true)?;
            self.state.lock().unwrap().declined_target = None;
            return Ok(());
        }
        let target_id = target.expect("eligible target").id()?;
        let inventory = player.get_inventory()?;
        let previous = inventory.get_selected()?;
        {
            let mut state = self.state.lock().unwrap();
            if let Some(lease) = &state.lease {
                if lease.selected == previous {
                    return Ok(());
                }
                 
                state.lease = None;
                state.declined_target = Some(target_id);
                return Ok(());
            }
            if state.declined_target == Some(target_id) {
                return Ok(());
            }
            state.declined_target = None;
        }
        let Some(axe) = find_hotbar(&inventory, previous, |name| name.ends_with("_axe"))? else {
            return Ok(());
        };
        self.select(player, axe, previous)?;
        self.record_action();
        Ok(())
    }

    pub(crate) fn auto_potion(&self, player: &LocalPlayer) -> anyhow::Result<()> {
        let health = player.get_health()?;
        let pitch = player.get_pitch()?;
        if !pitch.is_finite()
            || !health.is_finite()
            || health <= 0.0
            || health > self.slider("Health", 8.0, 1.0, 20.0)
            || player.is_using_item()?
            || !player.on_ground()?
            || pitch < 70.0
        {
            return Ok(());
        }
        let use_key = minecraft().options()?.key_use()?;
        if use_key.is_down()? {
            return Ok(());
        }
        let inventory = player.get_inventory()?;
        let previous = inventory.get_selected()?;
        let mut potion = None;
        for slot in hotbar_order(previous) {
            if inventory.get_item(slot)?.is_healing_potion()? {
                potion = Some(slot);
                break;
            }
        }
        let Some(potion) = potion else { return Ok(()) };
        let Some(mode) = minecraft().game_mode()? else {
            return Ok(());
        };
        self.select(player, potion, previous)?;
        let result = mode.use_item(player);
        self.record_action();
        if !result? {
            self.settle(true)?;
        }
        Ok(())
    }

    pub(crate) fn auto_cobweb(&self, player: &LocalPlayer) -> anyhow::Result<()> {
         
        if player.is_using_item()? || minecraft().options()?.key_use()?.is_down()? {
            return Ok(());
        }
        let Some((anchor, face, hit)) = minecraft().hit_block_face_in_reach()? else {
            return Ok(());
        };
        let (dx, dy, dz) = face.offset();
        let placed = anchor.offset(dx, dy, dz);
        let Some(world) = minecraft().world()? else {
            return Ok(());
        };
        let destination = world.block_registry_name(&placed)?;
        if !matches!(
            destination.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        ) {
            return Ok(());
        }
        let center = [
            placed.x() as f64 + 0.5,
            placed.y() as f64 + 0.5,
            placed.z() as f64 + 0.5,
        ];
        let own_id = player.id()?;
        let mut found = false;
        for target in world.get_entities()? {
            if !target.instance_of::<Player>()
                || target.id()? == own_id
                || crate::friends::ignores_entity(&target, ModuleId::AutoCobweb)
            {
                continue;
            }
            let Some(living) = target.as_living() else {
                continue;
            };
            if !living.is_alive()? {
                continue;
            }
            let pos = target.get_position()?;
            if near_target(center, [pos.x(), pos.y() + 0.5, pos.z()]) {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(());
        }
        let inventory = player.get_inventory()?;
        let previous = inventory.get_selected()?;
        let Some(slot) = find_hotbar(&inventory, previous, |name| name == "minecraft:cobweb")?
        else {
            return Ok(());
        };
        let Some(mode) = minecraft().game_mode()? else {
            return Ok(());
        };
        if !super::interaction::take(player, super::interaction::Action::Block) {
            return Ok(());
        }
        self.select(player, slot, previous)?;
        self.record_action();
        mode.place_block_on(player, &anchor, face, hit)?;
        Ok(())
    }

    pub(crate) fn totem_refill(&self, player: &LocalPlayer) -> anyhow::Result<()> {
        let inventory = player.get_inventory()?;
        let destination = self.slider("Hotbar Slot", 9.0, 1.0, 9.0).round() as i32 - 1;
        if !inventory.get_item(destination)?.is_empty()? {
            return Ok(());
        }
        let mut source = None;
        for slot in 9..36 {
            let stack = inventory.get_item(slot)?;
            if !stack.is_empty()?
                && stack.get_item()?.registry_name()? == "minecraft:totem_of_undying"
            {
                source = Some(slot);
                break;
            }
        }
        let Some(source) = source else { return Ok(()) };
        if !super::interaction::window_click_allowed(player) {
            return Ok(());
        }
        let Some(mode) = minecraft().game_mode()? else {
            return Ok(());
        };
        self.record_action();
        mode.container_swap(0, source, destination, player)?;
        Ok(())
    }
}

impl Module for PracticalCombat {
    fn on_start(&self) -> anyhow::Result<()> {
        self.settle(true)?;
        *self.state.lock().unwrap() = State::default();
        Ok(())
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        self.settle(true)?;
        *self.state.lock().unwrap() = State::default();
        Ok(())
    }
    fn on_tick(&self) -> anyhow::Result<()> {
        match self.tick() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.settle(true)?;
                Err(error)
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

fn hotbar_order(selected: i32) -> impl Iterator<Item = i32> {
    std::iter::once(selected).chain((0..9).filter(move |slot| *slot != selected))
}

fn find_hotbar(
    inventory: &Inventory,
    selected: i32,
    matches: impl Fn(&str) -> bool,
) -> anyhow::Result<Option<i32>> {
    for slot in hotbar_order(selected) {
        let stack = inventory.get_item(slot)?;
        if !stack.is_empty()? && matches(&stack.get_item()?.registry_name()?) {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

fn near_target(block: [f64; 3], target: [f64; 3]) -> bool {
    block
        .into_iter()
        .zip(target)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        <= 2.25
}

pub fn modules() -> [PracticalCombat; 5] {
    [
        PracticalCombat::new(
            ModuleId::AntiFireball,
            "Hits a deflectable fireball under your crosshair within reach.",
            vec![],
        ),
        PracticalCombat::new(
            ModuleId::ShieldBreaker,
            "Selects a hotbar axe while your crosshair is on a blocking player; restores your slot.",
            vec![],
        ),
        PracticalCombat::new(
            ModuleId::AutoPotion,
            "Throws a healing splash potion at low health when looking down on the ground.",
            vec![ModuleSetting::Slider {
                name: "Health".into(),
                value: 8.0,
                min: 1.0,
                max: 20.0,
            }],
        ),
        PracticalCombat::new(
            ModuleId::AutoCobweb,
            "Places a cobweb at the block face you aim at near a player.",
            vec![],
        ),
        PracticalCombat::new(
            ModuleId::TotemRefill,
            "Moves a spare totem into an empty chosen hotbar slot.",
            vec![ModuleSetting::Slider {
                name: "Hotbar Slot".into(),
                value: 9.0,
                min: 1.0,
                max: 9.0,
            }],
        ),
    ]
}
