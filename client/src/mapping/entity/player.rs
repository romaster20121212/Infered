 
 

use crate::mapping::client::player_info::PlayerInfo;
use crate::mapping::entity::{EntityRef, LivingEntityRef, PlayerRef};
use crate::mapping::inventory::{AbstractContainerMenu, EquipmentSlot, Inventory, ItemStack};
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};
use jni::sys::jboolean;

 
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GearStack {
     
    pub name: String,
     
    pub damage: i32,
     
    pub max_damage: i32,
     
    pub count: i32,
}

impl GearStack {
    fn read(stack: &ItemStack) -> anyhow::Result<Self> {
        let name = stack
            .get_item()
            .and_then(|item| item.registry_name())
            .map(|full| full.strip_prefix("minecraft:").unwrap_or(&full).to_string())
            .unwrap_or_default();
        Ok(Self {
            name,
            damage: stack.get_damage_value().unwrap_or(0),
            max_damage: stack.get_max_damage().unwrap_or(0),
            count: stack.count().unwrap_or(0),
        })
    }

     
     
     
    pub fn durability(&self) -> Option<f32> {
        (self.max_damage > 0)
            .then(|| (1.0 - self.damage as f32 / self.max_damage as f32).clamp(0.0, 1.0))
    }
}

 
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Equipment {
     
    pub armor: [Option<GearStack>; 4],
    pub main_hand: Option<GearStack>,
    pub off_hand: Option<GearStack>,
}

impl Equipment {
     
    pub fn is_empty(&self) -> bool {
        self.armor.iter().all(Option::is_none)
            && self.main_hand.is_none()
            && self.off_hand.is_none()
    }
}

 
 
const ARMOR_SLOTS: [&str; 4] = ["HEAD", "CHEST", "LEGS", "FEET"];
const MAIN_HAND_SLOT: &str = "MAINHAND";
const OFF_HAND_SLOT: &str = "OFFHAND";

 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Player)]
pub struct Player {
    jni_ref: GlobalRef,
}

impl Player {
    pub fn new(jni_ref: GlobalRef) -> Player {
        Player { jni_ref }
    }

     
     
     
     
     
     
    pub fn player_info(&self) -> anyhow::Result<PlayerInfo> {
        mapping().in_frame(|| {
            let info = mapping()
                .get_field(
                    MinecraftClassType::AbstractClientPlayer,
                    self.jni_ref().as_obj(),
                    "playerInfo",
                    FieldType::Object(MinecraftClassType::PlayerInfo),
                )?
                .l()?;
            if info.is_null() {
                anyhow::bail!("AbstractClientPlayer.playerInfo is null");
            }
            Ok(PlayerInfo::new(mapping().new_global_ref(info)?))
        })
    }

     
     
     
     
    pub fn equipment(&self) -> anyhow::Result<Equipment> {
        let mut out = Equipment::default();
        for (index, slot_name) in ARMOR_SLOTS.iter().enumerate() {
            out.armor[index] = self.item_in_slot(slot_name)?;
        }
        out.main_hand = self.item_in_slot(MAIN_HAND_SLOT)?;
        out.off_hand = self.item_in_slot(OFF_HAND_SLOT)?;
        Ok(out)
    }

    fn item_in_slot(&self, slot_name: &str) -> anyhow::Result<Option<GearStack>> {
        mapping().in_frame(|| {
            let slot = mapping()
                .get_static_field(
                    MinecraftClassType::EquipmentSlot,
                    slot_name,
                    FieldType::Object(MinecraftClassType::EquipmentSlot),
                )?
                .l()?;
            let stack = mapping()
                .call_method(
                    MinecraftClassType::LivingEntity,
                    self.jni_ref().as_obj(),
                    "getItemBySlot",
                    &[JValue::Object(&slot)],
                )?
                .l()?;
            let stack = ItemStack::new(mapping().new_global_ref(stack)?);
            if stack.is_empty()? {
                return Ok(None);
            }
            Ok(Some(GearStack::read(&stack)?))
        })
    }
}

impl EntityRef for Player {}
impl LivingEntityRef for Player {}
impl PlayerRef for Player {}

 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = LocalPlayer)]
pub struct LocalPlayer {
    jni_ref: GlobalRef,
    pub abilities: Abilities,
}

impl LocalPlayer {
    pub fn new(player_ref: GlobalRef) -> anyhow::Result<Self> {
        Ok(Self {
            abilities: Abilities::new(player_ref.clone())?,
            jni_ref: player_ref,
        })
    }

     
    pub fn reset_last_attacked_ticks(&self) -> anyhow::Result<()> {
        self.call_method("resetAttackStrengthTicker", &[])
            .or_else(|_| self.call_method("resetLastAttackedTicks", &[]))?;
        Ok(())
    }

     
     
    pub fn is_swinging(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("swinging", FieldType::Boolean)?.z()?)
    }

     
     
     
     
     
     
    pub fn advance_swing(&self, extra_ticks: i32) -> anyhow::Result<()> {
        if extra_ticks == 0 || !self.is_swinging()? {
            return Ok(());
        }
        let duration = self.call_method("getCurrentSwingDuration", &[])?.i()?;
        if duration <= 0 {
            return Ok(());
        }
        let mut swing_time = self.get_field("swingTime", FieldType::Int)?.i()?;
        let mut ended = false;
        if extra_ticks < 0 {
            swing_time = (swing_time + extra_ticks).max(0);
        } else {
            for _ in 0..extra_ticks {
                swing_time += 1;
                if swing_time >= duration {
                    swing_time = 0;
                    ended = true;
                    break;
                }
            }
        }
        self.set_field("swingTime", FieldType::Int, JValue::Int(swing_time))?;
        if ended {
            self.set_field("swinging", FieldType::Boolean, JValue::from(false))?;
        }
        let fraction = swing_time as f32 / duration as f32;
        self.set_field("attackAnim", FieldType::Float, JValue::Float(fraction))?;
        Ok(())
    }

     
     
     
    pub fn freeze_movement(&self) -> anyhow::Result<()> {
        self.set_field("xxa", FieldType::Float, JValue::Float(0.0))?;
        self.set_field("zza", FieldType::Float, JValue::Float(0.0))?;
         
         
        self.call_method("setJumping", &[JValue::Bool(0)])?;
        self.call_method("setShiftKeyDown", &[JValue::Bool(0)])?;
        self.set_delta_movement(0.0, 0.0, 0.0)?;
        Ok(())
    }

     
     
     
     
     
    pub fn drop_selected(&self, all: bool) -> anyhow::Result<bool> {
        Ok(self.call_method("drop", &[JValue::Bool(all as u8)])?.z()?)
    }

    pub fn get_inventory(&self) -> anyhow::Result<Inventory> {
        mapping().in_frame(|| {
            let inventory = mapping()
                .call_method(
                    MinecraftClassType::Player,
                    self.jni_ref().as_obj(),
                    "getInventory",
                    &[],
                )?
                .l()?;
            Ok(Inventory::new(mapping().new_global_ref(inventory)?))
        })
    }

     
    pub fn experience_level(&self) -> anyhow::Result<i32> {
        self.in_frame(|| Ok(self.get_field("experienceLevel", FieldType::Int)?.i()?))
    }

     
    pub fn food_level(&self) -> anyhow::Result<i32> {
        self.in_frame(|| {
            let food = self.call_method("getFoodData", &[])?.l()?;
            Ok(mapping()
                .call_method(MinecraftClassType::FoodData, &food, "getFoodLevel", &[])?
                .i()?)
        })
    }

    pub fn container_menu(&self) -> anyhow::Result<AbstractContainerMenu> {
        mapping().in_frame(|| {
            let menu = mapping()
                .get_field(
                    MinecraftClassType::Player,
                    self.jni_ref().as_obj(),
                    "containerMenu",
                    FieldType::Object(MinecraftClassType::AbstractContainerMenu),
                )?
                .l()?;
            Ok(AbstractContainerMenu::new(mapping().new_global_ref(menu)?))
        })
    }

    pub fn player_info(&self) -> anyhow::Result<PlayerInfo> {
        mapping().in_frame(|| {
            let info = mapping()
                .get_field(
                    MinecraftClassType::LocalPlayer,
                    self.jni_ref().as_obj(),
                    "playerInfo",
                    FieldType::Object(MinecraftClassType::PlayerInfo),
                )?
                .l()?;
            if info.is_null() {
                anyhow::bail!("LocalPlayer.playerInfo is null");
            }
            Ok(PlayerInfo::new(mapping().new_global_ref(info)?))
        })
    }

    pub fn equipment_slot_for_item(&self, stack: &ItemStack) -> anyhow::Result<EquipmentSlot> {
        mapping().in_frame(|| {
            let slot = mapping()
                .call_method(
                    MinecraftClassType::LivingEntity,
                    self.jni_ref().as_obj(),
                    "getEquipmentSlotForItem",
                    &[JValue::Object(stack.jni_ref().as_obj())],
                )?
                .l()?;
            Ok(EquipmentSlot::new(mapping().new_global_ref(slot)?))
        })
    }
}

impl EntityRef for LocalPlayer {}
impl LivingEntityRef for LocalPlayer {}
impl PlayerRef for LocalPlayer {}

#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Abilities)]
pub struct Abilities {
    jni_ref: GlobalRef,
}

impl Abilities {
    pub fn new(player: GlobalRef) -> anyhow::Result<Self> {
        mapping().in_frame(|| {
            let jni_ref = mapping()
                .call_method(MinecraftClassType::Player, &player, "getAbilities", &[])?
                .l()?;
            Ok(Self {
                jni_ref: mapping().new_global_ref(jni_ref)?,
            })
        })
    }

    pub fn fly(&self, value: bool) -> anyhow::Result<()> {
        let value: jboolean = if value { 1 } else { 0 };
        self.set_field("flying", FieldType::Boolean, JValue::Bool(value))?;
        self.set_field("mayfly", FieldType::Boolean, JValue::Bool(value))?;
        Ok(())
    }

    pub fn is_flying(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("flying", FieldType::Boolean)?.z()?)
    }

    pub fn set_flying_speed(&self, speed: f32) -> anyhow::Result<()> {
        self.call_method("setFlyingSpeed", &[JValue::Float(speed)])?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_may_fly(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("mayfly", FieldType::Boolean)?.z()?)
    }

    pub fn set_may_fly(&self, value: bool) -> anyhow::Result<()> {
        let value: jboolean = if value { 1 } else { 0 };
        self.set_field("mayfly", FieldType::Boolean, JValue::Bool(value))?;
        Ok(())
    }

    pub fn set_invulnerable(&self, value: bool) -> anyhow::Result<()> {
        let value: jboolean = if value { 1 } else { 0 };
        self.set_field("invulnerable", FieldType::Boolean, JValue::Bool(value))?;
        Ok(())
    }

    pub fn get_invulnerable(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("invulnerable", FieldType::Boolean)?.z()?)
    }
}
