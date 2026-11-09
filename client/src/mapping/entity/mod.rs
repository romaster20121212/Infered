 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::component::Component;
use crate::mapping::entity::living::LivingEntity;
use crate::mapping::math::Vec3;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::JString;
use jni::objects::{GlobalRef, JValue};

pub mod living;
pub mod mob;
pub mod player;

 
 
 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Entity)]
pub struct Entity {
    jni_ref: GlobalRef,
}

 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = ItemEntity)]
pub struct ItemEntity {
    jni_ref: GlobalRef,
}

impl EntityRef for ItemEntity {}

 
 
 
 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Projectile)]
pub struct Projectile {
    jni_ref: GlobalRef,
}

impl EntityRef for Projectile {}

 
 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = FishingHook)]
pub struct FishingHook {
    jni_ref: GlobalRef,
}

impl FishingHook {
     
    pub fn new(jni_ref: GlobalRef) -> FishingHook {
        FishingHook { jni_ref }
    }

     
     
    pub fn is_biting(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("biting", FieldType::Boolean)?.z()?)
    }

     
    pub fn player_owner(&self) -> anyhow::Result<Option<crate::mapping::entity::player::Player>> {
        self.in_frame(|| {
            let owner = self.call_method("getPlayerOwner", &[])?.l()?;
            if owner.is_null() {
                return Ok(None);
            }
            Ok(Some(crate::mapping::entity::player::Player::new(
                mapping().new_global_ref(owner)?,
            )))
        })
    }
}

impl EntityRef for FishingHook {}

 
 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = EndCrystal)]
pub struct EndCrystal {
    jni_ref: GlobalRef,
}

impl EntityRef for EndCrystal {}

impl Entity {
     
    pub fn new(jni_ref: GlobalRef) -> Entity {
        Entity { jni_ref }
    }
}

impl EntityRef for Entity {}

 
 
 
pub trait EntityRef: MappedObject + Sized {
     
     
    fn as_living(&self) -> Option<LivingEntity> {
        self.instance_of::<LivingEntity>()
            .then(|| LivingEntity::new(self.jni_ref().clone()))
    }

     
    fn id(&self) -> anyhow::Result<i32> {
        Ok(self.call_method("getId", &[])?.i()?)
    }

     
    fn get_position(&self) -> anyhow::Result<Vec3> {
        self.in_frame(|| {
            let vec3 = self.call_method("position", &[])?.l()?;
            Vec3::read(&vec3)
        })
    }

     
    fn get_eye_position(&self) -> anyhow::Result<Vec3> {
        self.in_frame(|| {
            let vec3 = self.call_method("getEyePosition", &[])?.l()?;
            Vec3::read(&vec3)
        })
    }

     
    fn get_yaw(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getYRot", &[])?.f()?)
    }

     
    fn get_pitch(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getXRot", &[])?.f()?)
    }

     
    fn distance_to_sqr(&self, x: f64, y: f64, z: f64) -> anyhow::Result<f64> {
        Ok(self
            .call_method(
                "distanceToSqr",
                &[JValue::Double(x), JValue::Double(y), JValue::Double(z)],
            )?
            .d()?)
    }

     
    fn bb_width(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getBbWidth", &[])?.f()?)
    }

     
    fn bb_height(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getBbHeight", &[])?.f()?)
    }

     
    fn is_sprinting(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isSprinting", &[])?.z()?)
    }

     
    fn set_sprinting(&self, value: bool) -> anyhow::Result<()> {
        self.call_method("setSprinting", &[JValue::from(value)])?;
        Ok(())
    }

     
    fn on_ground(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("onGround", &[])?.z()?)
    }

     
    fn jump_from_ground(&self) -> anyhow::Result<()> {
        self.call_method("jumpFromGround", &[])?;
        Ok(())
    }

     
    fn is_in_water(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isInWater", &[])?.z()?)
    }

     
    fn is_in_lava(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isInLava", &[])?.z()?)
    }

     
    fn on_climbable(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("onClimbable", &[])?.z()?)
    }

     
    fn get_delta_movement(&self) -> anyhow::Result<Vec3> {
        self.in_frame(|| {
            let vec3 = self.call_method("getDeltaMovement", &[])?.l()?;
            Vec3::read(&vec3)
        })
    }

     
    fn set_delta_movement(&self, x: f64, y: f64, z: f64) -> anyhow::Result<()> {
        self.call_method(
            "setDeltaMovement",
            &[JValue::Double(x), JValue::Double(y), JValue::Double(z)],
        )?;
        Ok(())
    }

     
    fn set_pos(&self, x: f64, y: f64, z: f64) -> anyhow::Result<()> {
        self.call_method(
            "setPos",
            &[JValue::Double(x), JValue::Double(y), JValue::Double(z)],
        )?;
        Ok(())
    }

     
    fn is_shift_key_down(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isShiftKeyDown", &[])?.z()?)
    }

     
     
    fn set_no_physics(&self, value: bool) -> anyhow::Result<()> {
        self.set_field("noPhysics", FieldType::Boolean, JValue::from(value))?;
        Ok(())
    }

     
    fn set_invulnerable(&self, value: bool) -> anyhow::Result<()> {
        self.call_method("setInvulnerable", &[JValue::from(value)])?;
        Ok(())
    }

     
     
     
     
    fn set_rotation(&self, yaw: f32, pitch: f32) -> anyhow::Result<()> {
        self.call_method("setYRot", &[JValue::Float(yaw)])?;
        self.call_method("setXRot", &[JValue::Float(pitch)])?;
        self.set_field("yRotO", FieldType::Float, JValue::Float(yaw))?;
        self.set_field("xRotO", FieldType::Float, JValue::Float(pitch))?;
        Ok(())
    }

     
    fn get_fall_distance(&self) -> anyhow::Result<f64> {
         
         
        match self.get_field("fallDistance", FieldType::Float) {
            Ok(value) => Ok(value.f()? as f64),
            Err(_) => Ok(self.get_field("fallDistance", FieldType::Double)?.d()?),
        }
    }

     
     
    fn discard(&self) -> anyhow::Result<()> {
        Ok(self.call_method("discard", &[])?.v()?)
    }

     
     
    fn is_removed(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isRemoved", &[])?.z()?)
    }

     
    fn reset_fall_distance(&self) -> anyhow::Result<()> {
        Ok(self.call_method("resetFallDistance", &[])?.v()?)
    }

     
    fn get_name(&self) -> anyhow::Result<Component> {
        self.in_frame(|| {
            let component = self.call_method("getName", &[])?.l()?;
            Ok(Component::new(mapping().new_global_ref(component)?))
        })
    }

     
    fn get_tick_count(&self) -> anyhow::Result<i32> {
        Ok(self.get_field("tickCount", FieldType::Int)?.i()?)
    }

     
    fn is_invisible(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isInvisible", &[])?.z()?)
    }

     
    fn is_spectator(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isSpectator", &[])?.z()?)
    }
}

 
 
pub trait LivingEntityRef: EntityRef {
     
    fn get_health(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getHealth", &[])?.f()?)
    }

     
     
     
    fn get_active_effects(&self) -> anyhow::Result<Vec<(String, i32, i32)>> {
        self.in_frame(|| {
            let collection = mapping()
                .call_method(
                    MinecraftClassType::LivingEntity,
                    self.jni_ref().as_obj(),
                    "getActiveEffects",
                    &[],
                )?
                .l()?;
            let iterable = crate::mapping::java::iterable::Iterable::new(
                mapping().new_global_ref(collection)?,
            );
            let iterator = iterable.iterator()?;
            let mut effects = Vec::new();
            while iterator.has_next()? {
                let effect = iterator.next()?;
                let row =
                    mapping().in_frame(|| -> anyhow::Result<Option<(String, i32, i32)>> {
                        let amplifier = mapping()
                            .call_method(
                                MinecraftClassType::MobEffectInstance,
                                effect.as_obj(),
                                "getAmplifier",
                                &[],
                            )?
                            .i()?;
                        let duration = mapping()
                            .call_method(
                                MinecraftClassType::MobEffectInstance,
                                effect.as_obj(),
                                "getDuration",
                                &[],
                            )?
                            .i()?;
                         
                         
                        let holder = mapping()
                            .call_method(
                                MinecraftClassType::MobEffectInstance,
                                effect.as_obj(),
                                "getEffect",
                                &[],
                            )?
                            .l()?;
                        let registered = mapping().call_method(
                            MinecraftClassType::Holder,
                            &holder,
                            "getRegisteredName",
                            &[],
                        );
                        let name = if let Ok(registered) = registered {
                            mapping().get_string(registered.l()?)?
                        } else {
                            let key = mapping()
                                .call_method(MinecraftClassType::Holder, &holder, "unwrapKey", &[])?
                                .l()?;
                            let present = mapping()
                                .call_method(MinecraftClassType::Optional, &key, "isPresent", &[])?
                                .z()?;
                            if !present {
                                return Ok(None);
                            }
                            let resource_key = mapping()
                                .call_method(MinecraftClassType::Optional, &key, "get", &[])?
                                .l()?;
                            let location = mapping()
                                .call_method(
                                    MinecraftClassType::ResourceKey,
                                    &resource_key,
                                    "location",
                                    &[],
                                )?
                                .l()?;
                            let full = mapping()
                                .call_method(
                                    MinecraftClassType::ResourceLocation,
                                    &location,
                                    "toString",
                                    &[],
                                )?
                                .l()?;
                            let full_jstr = JString::from(full);
                            let java_str = mapping().get_env()?.get_string(&full_jstr)?;
                            String::from(java_str)
                        };
                        let short = name.strip_prefix("minecraft:").unwrap_or(&name).to_string();
                        Ok(Some((short, amplifier, duration)))
                    })?;
                if let Some(row) = row {
                    effects.push(row);
                }
            }
            effects.sort_by(|a, b| a.0.cmp(&b.0));
            Ok(effects)
        })
    }

     
    fn get_max_health(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getMaxHealth", &[])?.f()?)
    }

     
    fn get_armor_value(&self) -> anyhow::Result<i32> {
        Ok(self.call_method("getArmorValue", &[])?.i()?)
    }

     
     
     
    fn get_absorption(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getAbsorptionAmount", &[])?.f()?)
    }

     
    fn get_main_hand_item(&self) -> anyhow::Result<crate::mapping::inventory::ItemStack> {
        self.in_frame(|| {
            let stack = mapping()
                .call_method(
                    MinecraftClassType::LivingEntity,
                    self.jni_ref().as_obj(),
                    "getMainHandItem",
                    &[],
                )?
                .l()?;
            Ok(crate::mapping::inventory::ItemStack::new(
                mapping().new_global_ref(stack)?,
            ))
        })
    }

     
    fn get_offhand_item(&self) -> anyhow::Result<crate::mapping::inventory::ItemStack> {
        self.in_frame(|| {
            let slot = mapping()
                .get_static_field(
                    MinecraftClassType::EquipmentSlot,
                    "OFFHAND",
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
            Ok(crate::mapping::inventory::ItemStack::new(
                mapping().new_global_ref(stack)?,
            ))
        })
    }

     
     
     
    fn is_alive(&self) -> anyhow::Result<bool> {
        Ok(!self.call_method("isDeadOrDying", &[])?.z()?)
    }

     
    fn hurt_time(&self) -> anyhow::Result<i32> {
        mapping().in_frame(|| {
            Ok(mapping()
                .get_field(
                    MinecraftClassType::LivingEntity,
                    self.jni_ref().as_obj(),
                    "hurtTime",
                    FieldType::Int,
                )?
                .i()?)
        })
    }

     
     
    fn is_using_item(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isUsingItem", &[])?.z()?)
    }

     
     
    fn get_ticks_using_item(&self) -> anyhow::Result<i32> {
        Ok(self.call_method("getTicksUsingItem", &[])?.i()?)
    }

     
     
     
    fn is_blocking(&self) -> anyhow::Result<bool> {
        match self.call_method("isBlocking", &[]) {
            Ok(result) => Ok(result.z()?),
            Err(_) => self.is_using_item(),
        }
    }

     
     
    fn move_input(&self) -> anyhow::Result<(f32, f32)> {
        let strafe = self.get_field("xxa", FieldType::Float)?.f()?;
        let forward = self.get_field("zza", FieldType::Float)?.f()?;
        Ok((strafe, forward))
    }

     
    fn is_fall_flying(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isFallFlying", &[])?.z()?)
    }

     
     
    fn has_move_input(&self) -> anyhow::Result<bool> {
        let (strafe, forward) = self.move_input()?;
        Ok(strafe != 0.0 || forward != 0.0)
    }

     
     
    fn get_speed(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getSpeed", &[])?.f()?)
    }

     
    fn is_jumping(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("jumping", FieldType::Boolean)?.z()?)
    }

     
    fn swing(&self) -> anyhow::Result<()> {
        self.in_frame(|| {
            let hand = mapping()
                .get_static_field(
                    MinecraftClassType::InteractionHand,
                    "MAIN_HAND",
                    FieldType::Object(MinecraftClassType::InteractionHand),
                )?
                .l()?;
            self.call_method("swing", &[JValue::Object(&hand)])?;
            Ok(())
        })
    }

     
     
    fn get_attribute_value(&self, attribute: &str) -> anyhow::Result<Option<f64>> {
        self.in_frame(|| {
            let attribute_holder = mapping()
                .get_static_field(
                    MinecraftClassType::Attributes,
                    attribute,
                    FieldType::Object(MinecraftClassType::Holder),
                )
                .ok()
                .and_then(|value| value.l().ok());
            let Some(attribute_holder) = attribute_holder else {
                return Ok(None);
            };
            if attribute_holder.is_null() {
                return Ok(None);
            }
            let instance = self
                .call_method("getAttribute", &[JValue::Object(&attribute_holder)])?
                .l()?;
            if instance.is_null() {
                return Ok(None);
            }
            let value = mapping()
                .call_method(
                    MinecraftClassType::AttributeInstance,
                    &instance,
                    "getValue",
                    &[],
                )?
                .d()?;
            Ok(Some(value))
        })
    }

     
    fn attribute_modifier_amount(&self, attribute: &str, id: &str) -> anyhow::Result<Option<f64>> {
        self.in_frame(|| {
            let holder = mapping()
                .get_static_field(
                    MinecraftClassType::Attributes,
                    attribute,
                    FieldType::Object(MinecraftClassType::Holder),
                )?
                .l()?;
            let instance = self
                .call_method("getAttribute", &[JValue::Object(&holder)])?
                .l()?;
            if instance.is_null() {
                return Ok(None);
            }
            let env = mapping().get_env()?;
            let text = env.new_string(id)?;
            let resource = mapping()
                .call_static_method(
                    MinecraftClassType::ResourceLocation,
                    "parse",
                    &[JValue::Object(&text)],
                )?
                .l()?;
            let modifier = mapping()
                .call_method(
                    MinecraftClassType::AttributeInstance,
                    &instance,
                    "getModifier",
                    &[JValue::Object(&resource)],
                )?
                .l()?;
            if modifier.is_null() {
                return Ok(None);
            }
            Ok(Some(
                mapping()
                    .call_method(
                        MinecraftClassType::AttributeModifier,
                        &modifier,
                        "amount",
                        &[],
                    )?
                    .d()?,
            ))
        })
    }

     
     
     
    fn add_attribute_modifier(&self, attribute: &str, id: &str, amount: f64) -> anyhow::Result<()> {
        self.in_frame(|| {
            let attribute_holder = mapping()
                .get_static_field(
                    MinecraftClassType::Attributes,
                    attribute,
                    FieldType::Object(MinecraftClassType::Holder),
                )?
                .l()?;
            let instance = self
                .call_method("getAttribute", &[JValue::Object(&attribute_holder)])?
                .l()?;
            if instance.is_null() {
                anyhow::bail!("entity has no '{attribute}' attribute");
            }

             
            let mut env = mapping().get_env()?;
            let jid = env.new_string(id)?;
            let resource = mapping()
                .call_static_method(
                    MinecraftClassType::ResourceLocation,
                    "parse",
                    &[JValue::Object(&jid)],
                )?
                .l()?;

             
             
            let has = mapping()
                .call_method(
                    MinecraftClassType::AttributeInstance,
                    &instance,
                    "hasModifier",
                    &[JValue::Object(&resource)],
                )?
                .z()?;
            if has {
                mapping().call_method(
                    MinecraftClassType::AttributeInstance,
                    &instance,
                    "removeModifier",
                    &[JValue::Object(&resource)],
                )?;
            }

            let operation = mapping()
                .get_static_field(
                    MinecraftClassType::AttributeModifierOperation,
                    "ADD_VALUE",
                    FieldType::Object(MinecraftClassType::AttributeModifierOperation),
                )?
                .l()?;
            let modifier_class = mapping().resolve_class(
                &mut env,
                &mapping().runtime_class_name(MinecraftClassType::AttributeModifier)?,
            )?;
            let modifier = env.new_object(
                &modifier_class,
                format!(
                    "(L{};DL{};)V",
                    mapping().runtime_class_name(MinecraftClassType::ResourceLocation)?,
                    mapping().runtime_class_name(MinecraftClassType::AttributeModifierOperation)?,
                ),
                &[
                    JValue::Object(&resource),
                    JValue::Double(amount),
                    JValue::Object(&operation),
                ],
            )?;
            mapping().call_method(
                MinecraftClassType::AttributeInstance,
                &instance,
                "addTransientModifier",
                &[JValue::Object(&modifier)],
            )?;
            Ok(())
        })
    }

     
     
    fn remove_attribute_modifier(&self, attribute: &str, id: &str) -> anyhow::Result<()> {
        self.in_frame(|| {
            let attribute_holder = mapping()
                .get_static_field(
                    MinecraftClassType::Attributes,
                    attribute,
                    FieldType::Object(MinecraftClassType::Holder),
                )
                .ok()
                .and_then(|value| value.l().ok());
            let Some(attribute_holder) = attribute_holder else {
                return Ok(());
            };
            if attribute_holder.is_null() {
                return Ok(());
            }
            let instance = self
                .call_method("getAttribute", &[JValue::Object(&attribute_holder)])?
                .l()?;
            if instance.is_null() {
                return Ok(());
            }
            let env = mapping().get_env()?;
            let jid = env.new_string(id)?;
            let resource = mapping()
                .call_static_method(
                    MinecraftClassType::ResourceLocation,
                    "parse",
                    &[JValue::Object(&jid)],
                )?
                .l()?;
            let has = mapping()
                .call_method(
                    MinecraftClassType::AttributeInstance,
                    &instance,
                    "hasModifier",
                    &[JValue::Object(&resource)],
                )?
                .z()?;
            if has {
                mapping().call_method(
                    MinecraftClassType::AttributeInstance,
                    &instance,
                    "removeModifier",
                    &[JValue::Object(&resource)],
                )?;
            }
            Ok(())
        })
    }
}

 
 
pub trait PlayerRef: LivingEntityRef {
     
     
    fn attack_strength_scale(&self) -> anyhow::Result<f32> {
        self.attack_strength_scale_at(0.5)
    }

     
    fn attack_strength_scale_at(&self, tick_offset: f32) -> anyhow::Result<f32> {
        Ok(self
            .call_method("getAttackStrengthScale", &[JValue::Float(tick_offset)])?
            .f()?)
    }
}
