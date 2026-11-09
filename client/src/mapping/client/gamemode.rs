use crate::mapping::block::Direction;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::Entity;
use crate::mapping::math::{BlockPos, Vec3};
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};

#[derive(Debug, Clone, MappedObject)]
#[mapped(class = MultiPlayerGameMode)]
pub struct MultiPlayerGameMode {
    jni_ref: GlobalRef,
}

impl MultiPlayerGameMode {
    pub fn new(jni_ref: GlobalRef) -> Self {
        Self { jni_ref }
    }

     
     
     
    pub fn set_carried_index(&self, slot: i32) -> anyhow::Result<()> {
        self.set_field("carriedIndex", FieldType::Int, JValue::Int(slot))
    }

     
    pub fn attack(&self, player: &LocalPlayer, target: &Entity) -> anyhow::Result<()> {
        self.call_method(
            "attack",
            &[
                JValue::Object(player.jni_ref().as_obj()),
                JValue::Object(target.jni_ref().as_obj()),
            ],
        )?;
        Ok(())
    }

     
     
    fn container_click(
        &self,
        container_id: i32,
        slot: i32,
        button: i32,
        action: &str,
        player: &LocalPlayer,
    ) -> anyhow::Result<()> {
        self.in_frame(|| {
            let input = mapping()
                .get_static_field(
                    MinecraftClassType::ContainerInput,
                    action,
                    FieldType::Object(MinecraftClassType::ContainerInput),
                )?
                .l()?;
            self.call_method(
                "handleContainerInput",
                &[
                    JValue::Int(container_id),
                    JValue::Int(slot),
                    JValue::Int(button),
                    JValue::Object(&input),
                    JValue::Object(player.jni_ref().as_obj()),
                ],
            )?;
            Ok(())
        })
    }

     
     
     
    pub fn use_item(&self, player: &LocalPlayer) -> anyhow::Result<bool> {
        self.in_frame(|| {
            let hand = mapping()
                .get_static_field(
                    MinecraftClassType::InteractionHand,
                    "MAIN_HAND",
                    FieldType::Object(MinecraftClassType::InteractionHand),
                )?
                .l()?;
            let result = self
                .call_method(
                    "useItem",
                    &[
                        JValue::Object(player.jni_ref().as_obj()),
                        JValue::Object(&hand),
                    ],
                )?
                .l()?;
             
            let consumes = mapping()
                .call_method(
                    MinecraftClassType::InteractionResult,
                    &result,
                    "consumesAction",
                    &[],
                )?
                .z()?;
            Ok(consumes)
        })
    }

     
    pub fn release_using_item(&self, player: &LocalPlayer) -> anyhow::Result<()> {
        self.call_method(
            "releaseUsingItem",
            &[JValue::Object(player.jni_ref().as_obj())],
        )?;
        Ok(())
    }

     
     
    pub fn container_throw(
        &self,
        container_id: i32,
        slot: i32,
        all: bool,
        player: &LocalPlayer,
    ) -> anyhow::Result<()> {
        self.container_click(container_id, slot, if all { 1 } else { 0 }, "THROW", player)
    }

     
     
    pub fn container_swap(
        &self,
        container_id: i32,
        slot: i32,
        hotbar_button: i32,
        player: &LocalPlayer,
    ) -> anyhow::Result<()> {
        self.container_click(container_id, slot, hotbar_button, "SWAP", player)
    }

     
     
     
    pub fn container_pickup(
        &self,
        container_id: i32,
        slot: i32,
        player: &LocalPlayer,
    ) -> anyhow::Result<()> {
        self.container_click(container_id, slot, 0, "PICKUP", player)
    }

     
     
    pub fn container_quick_move(
        &self,
        container_id: i32,
        slot: i32,
        player: &LocalPlayer,
    ) -> anyhow::Result<()> {
        self.container_click(container_id, slot, 0, "QUICK_MOVE", player)
    }

     
     
    pub fn destroy_block_pos(&self) -> anyhow::Result<Option<BlockPos>> {
        self.in_frame(|| {
            let pos = self
                .get_field(
                    "destroyBlockPos",
                    FieldType::Object(MinecraftClassType::BlockPos),
                )?
                .l()?;
            if pos.is_null() {
                return Ok(None);
            }
            Ok(Some(BlockPos::read(&pos)?))
        })
    }

     
     
    pub fn destroy_progress(&self) -> anyhow::Result<f32> {
        self.in_frame(|| Ok(self.get_field("destroyProgress", FieldType::Float)?.f()?))
    }

     
    fn destroy_block(&self, method: &str, pos: &BlockPos, face: Direction) -> anyhow::Result<()> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let pos_obj = pos.to_java(&mut env)?;
            let face_obj = face.to_java()?;
            self.call_method(
                method,
                &[JValue::Object(&pos_obj), JValue::Object(&face_obj)],
            )?;
            Ok(())
        })
    }

     
     
     
    pub fn start_destroy_block(&self, pos: &BlockPos, face: Direction) -> anyhow::Result<()> {
        self.destroy_block("startDestroyBlock", pos, face)
    }

     
    pub fn continue_destroy_block(&self, pos: &BlockPos, face: Direction) -> anyhow::Result<()> {
        self.destroy_block("continueDestroyBlock", pos, face)
    }

     
     
    pub fn stop_destroy_block(&self) -> anyhow::Result<()> {
        self.call_method("stopDestroyBlock", &[])?;
        Ok(())
    }

     
     
     
    pub fn place_block_on(
        &self,
        player: &LocalPlayer,
        anchor: &BlockPos,
        face: Direction,
        hit: Vec3,
    ) -> anyhow::Result<()> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let hit_obj = hit.to_java(&mut env)?;
            let anchor_obj = anchor.to_java(&mut env)?;
            let face_obj = face.to_java()?;
            let block_hit_class = mapping().resolve_class(
                &mut env,
                &mapping().runtime_class_name(MinecraftClassType::BlockHitResult)?,
            )?;
            let block_hit = {
                let hit_desc = format!(
                    "L{};",
                    mapping().runtime_class_name(MinecraftClassType::Vec3)?
                );
                let face_desc = format!(
                    "L{};",
                    mapping().runtime_class_name(MinecraftClassType::Direction)?
                );
                let pos_desc = format!(
                    "L{};",
                    mapping().runtime_class_name(MinecraftClassType::BlockPos)?
                );
                let signature = format!("({hit_desc}{face_desc}{pos_desc}Z)V");
                env.new_object(
                    &block_hit_class,
                    &signature,
                    &[
                        JValue::Object(&hit_obj),
                        JValue::Object(&face_obj),
                        JValue::Object(&anchor_obj),
                        JValue::Bool(0),
                    ],
                )?
            };
            let hand = mapping()
                .get_static_field(
                    MinecraftClassType::InteractionHand,
                    "MAIN_HAND",
                    FieldType::Object(MinecraftClassType::InteractionHand),
                )?
                .l()?;
            self.call_method(
                "useItemOn",
                &[
                    JValue::Object(player.jni_ref().as_obj()),
                    JValue::Object(&hand),
                    JValue::Object(&block_hit),
                ],
            )?;
            Ok(())
        })
    }
}
