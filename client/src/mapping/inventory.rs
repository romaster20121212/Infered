 
 

use crate::mapping::java::iterable::Iterable;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JString, JValue};

 
#[derive(Debug, MappedObject)]
#[mapped(class = Inventory)]
pub struct Inventory {
    jni_ref: GlobalRef,
}

impl Inventory {
     
    pub const SLOT_OFFHAND: i32 = 40;

     
    pub fn new(jni_ref: GlobalRef) -> Inventory {
        Inventory { jni_ref }
    }

     
     
     
    pub fn menu_slot(inventory_slot: i32) -> i32 {
        if inventory_slot < 9 {
            inventory_slot + 36
        } else {
            inventory_slot
        }
    }

     
    pub fn get_item(&self, slot: i32) -> anyhow::Result<ItemStack> {
        self.in_frame(|| {
            let stack = self.call_method("getItem", &[JValue::Int(slot)])?.l()?;
            Ok(ItemStack::new(mapping().new_global_ref(stack)?))
        })
    }

     
    pub fn get_selected(&self) -> anyhow::Result<i32> {
        Ok(self.get_field("selected", FieldType::Int)?.i()?)
    }

     
     
    pub fn set_selected(&self, slot: i32) -> anyhow::Result<()> {
        self.set_field("selected", FieldType::Int, JValue::Int(slot))?;
        Ok(())
    }
}

 
#[derive(Debug, MappedObject)]
#[mapped(class = ItemStack)]
pub struct ItemStack {
    jni_ref: GlobalRef,
}

impl ItemStack {
     
    pub fn new(jni_ref: GlobalRef) -> ItemStack {
        ItemStack { jni_ref }
    }

     
    pub fn is_empty(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isEmpty", &[])?.z()?)
    }

     
    pub fn count(&self) -> anyhow::Result<i32> {
        Ok(self.call_method("getCount", &[])?.i()?)
    }

     
     
     
    pub fn is_food(&self) -> anyhow::Result<bool> {
        if self.is_empty()? {
            return Ok(false);
        }
        self.in_frame(|| {
            for name in ["FOOD", "CONSUMABLE"] {
                let component = mapping()
                    .get_static_field(
                        MinecraftClassType::DataComponents,
                        name,
                        FieldType::Object(MinecraftClassType::DataComponentType),
                    )?
                    .l()?;
                if !self
                    .call_method("has", &[JValue::Object(&component)])?
                    .z()?
                {
                    return Ok(false);
                }
            }
            Ok(true)
        })
    }

     
    pub fn is_healing_potion(&self) -> anyhow::Result<bool> {
        if self.is_empty()? || self.get_item()?.registry_name()? != "minecraft:splash_potion" {
            return Ok(false);
        }
        self.in_frame(|| {
            let component = mapping()
                .get_static_field(
                    MinecraftClassType::DataComponents,
                    "POTION_CONTENTS",
                    FieldType::Object(MinecraftClassType::DataComponentType),
                )?
                .l()?;
            let contents = self
                .call_method("get", &[JValue::Object(&component)])?
                .l()?;
            if contents.is_null() {
                return Ok(false);
            }
            let effects = mapping()
                .call_method(
                    MinecraftClassType::PotionContents,
                    &contents,
                    "getAllEffects",
                    &[],
                )?
                .l()?;
            let iter = Iterable::new(mapping().new_global_ref(effects)?).iterator()?;
            while iter.has_next()? {
                let effect = iter.next()?;
                let holder = mapping()
                    .call_method(
                        MinecraftClassType::MobEffectInstance,
                        effect.as_obj(),
                        "getEffect",
                        &[],
                    )?
                    .l()?;
                let name = mapping()
                    .call_method(
                        MinecraftClassType::Holder,
                        &holder,
                        "getRegisteredName",
                        &[],
                    )?
                    .l()?;
                let name = mapping().get_string(name)?;
                if name.strip_prefix("minecraft:").unwrap_or(&name) == "instant_health" {
                    return Ok(true);
                }
            }
            Ok(false)
        })
    }

     
    pub fn get_damage_value(&self) -> anyhow::Result<i32> {
        Ok(self.call_method("getDamageValue", &[])?.i()?)
    }

     
    pub fn get_max_damage(&self) -> anyhow::Result<i32> {
        Ok(self.call_method("getMaxDamage", &[])?.i()?)
    }

     
    pub fn get_item(&self) -> anyhow::Result<Item> {
        self.in_frame(|| {
            let item = self.call_method("getItem", &[])?.l()?;
            Ok(Item::new(mapping().new_global_ref(item)?))
        })
    }

     
    pub fn get_display_name(&self) -> anyhow::Result<String> {
        self.in_frame(|| {
            let component = self.call_method("getHoverName", &[])?.l()?;
            let component =
                crate::mapping::component::Component::new(mapping().new_global_ref(component)?);
            component.get_string()
        })
    }
}

 
 
 
#[derive(Debug, MappedObject)]
#[mapped(class = Item)]
pub struct Item {
    jni_ref: GlobalRef,
}

impl Item {
     
    pub fn new(jni_ref: GlobalRef) -> Item {
        Item { jni_ref }
    }

     
     
     
    pub fn is_block_item(&self) -> bool {
        self.instance_of::<BlockItem>()
    }

     
     
    pub fn registry_name(&self) -> anyhow::Result<String> {
        self.in_frame(|| {
            let registry = crate::mapping::chunk::item_registry()?;
            let key = mapping()
                .call_method(
                    MinecraftClassType::Registry,
                    registry.as_obj(),
                    "getKey",
                    &[JValue::Object(self.jni_ref().as_obj())],
                )?
                .l()?;
            let name = mapping()
                .call_method(MinecraftClassType::ResourceLocation, &key, "toString", &[])?
                .l()?;
            let name_jstr = JString::from(name);
            let java_str = mapping().get_env()?.get_string(&name_jstr)?;
            Ok(String::from(java_str))
        })
    }
}

 
 
 
#[derive(Debug, MappedObject)]
#[mapped(class = BlockItem)]
pub struct BlockItem {
    jni_ref: GlobalRef,
}

impl BlockItem {
     
    pub fn new(jni_ref: GlobalRef) -> BlockItem {
        BlockItem { jni_ref }
    }
}

 
#[derive(Debug, MappedObject)]
#[mapped(class = AbstractContainerMenu)]
pub struct AbstractContainerMenu {
    jni_ref: GlobalRef,
}

impl AbstractContainerMenu {
     
    pub fn new(jni_ref: GlobalRef) -> AbstractContainerMenu {
        AbstractContainerMenu { jni_ref }
    }

     
    pub fn container_id(&self) -> anyhow::Result<i32> {
        Ok(self.get_field("containerId", FieldType::Int)?.i()?)
    }

     
     
    pub fn get_items(&self) -> anyhow::Result<Vec<ItemStack>> {
        self.in_frame(|| {
            let list = self.call_method("getItems", &[])?.l()?;
            let iterable = Iterable::new(mapping().new_global_ref(list)?);
            let iterator = iterable.iterator()?;
            let mut items = Vec::new();
            while iterator.has_next()? {
                items.push(ItemStack::new(iterator.next()?));
            }
            Ok(items)
        })
    }
}

 
#[derive(Debug, MappedObject)]
#[mapped(class = EquipmentSlot)]
pub struct EquipmentSlot {
    jni_ref: GlobalRef,
}

impl EquipmentSlot {
     
    pub fn new(jni_ref: GlobalRef) -> EquipmentSlot {
        EquipmentSlot { jni_ref }
    }

     
    pub fn is_armor(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isArmor", &[])?.z()?)
    }

     
     
    pub fn index(&self, base: i32) -> anyhow::Result<i32> {
        Ok(self.call_method("getIndex", &[JValue::Int(base)])?.i()?)
    }
}

 
pub fn totem_of_undying() -> anyhow::Result<Item> {
    mapping().in_frame(|| {
        let item = mapping()
            .get_static_field(
                MinecraftClassType::Items,
                "TOTEM_OF_UNDYING",
                FieldType::Object(MinecraftClassType::Item),
            )?
            .l()?;
        Ok(Item::new(mapping().new_global_ref(item)?))
    })
}
