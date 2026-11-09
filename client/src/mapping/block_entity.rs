 

use crate::mapping::math::BlockPos;
use crate::mapping::{MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;

 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = BlockEntity)]
pub struct BlockEntity {
    jni_ref: GlobalRef,
}

impl BlockEntity {
     
    pub fn new(jni_ref: GlobalRef) -> BlockEntity {
        BlockEntity { jni_ref }
    }

     
    pub fn get_block_pos(&self) -> anyhow::Result<BlockPos> {
        self.in_frame(|| {
            let pos = self.call_method("getBlockPos", &[])?.l()?;
            BlockPos::read(&pos)
        })
    }

     
     
     
    pub fn is_container(&self) -> bool {
        const KINDS: [MinecraftClassType; 8] = [
            MinecraftClassType::ChestBlockEntity,
            MinecraftClassType::EnderChestBlockEntity,
            MinecraftClassType::BarrelBlockEntity,
            MinecraftClassType::ShulkerBoxBlockEntity,
            MinecraftClassType::HopperBlockEntity,
            MinecraftClassType::AbstractFurnaceBlockEntity,
            MinecraftClassType::DropperBlockEntity,
            MinecraftClassType::DispenserBlockEntity,
        ];
        self.in_frame(|| {
            Ok(KINDS.iter().any(|&kind| {
                mapping()
                    .is_instance_of(kind, self.jni_ref().as_obj())
                    .unwrap_or(false)
            }))
        })
        .unwrap_or(false)
    }

     
    pub fn is_spawner(&self) -> bool {
        self.in_frame(|| {
            mapping().is_instance_of(
                MinecraftClassType::MobSpawnerBlockEntity,
                self.jni_ref().as_obj(),
            )
        })
        .unwrap_or(false)
    }
}
