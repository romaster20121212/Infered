use crate::mapping::block_entity::BlockEntity;
use crate::mapping::chunk::LevelChunkSection;
use crate::mapping::entity::Entity;
use crate::mapping::java::iterable::Iterable;
use crate::mapping::math::BlockPos;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JObject, JString, JValue};

#[derive(Debug, MappedObject)]
#[mapped(class = Level)]
pub struct World {
    jni_ref: GlobalRef,
}

impl World {
     
    pub fn new(jni_ref: GlobalRef) -> World {
        World { jni_ref }
    }

     
    pub fn get_entities(&self) -> anyhow::Result<Vec<Entity>> {
        self.in_frame(|| {
            let iterable_obj = self.call_method("entitiesForRendering", &[])?.l()?;
            let iterable = Iterable::new(mapping().new_global_ref(iterable_obj)?);

            let iterator = iterable.iterator()?;
            let mut entities = Vec::new();
            while iterator.has_next()? {
                entities.push(Entity::new(iterator.next()?));
            }
            Ok(entities)
        })
    }

     
    pub fn get_entity(&self, id: i32) -> anyhow::Result<Option<Entity>> {
        self.in_frame(|| {
            let entity = self.call_method("getEntity", &[JValue::Int(id)])?.l()?;
            if entity.is_null() {
                return Ok(None);
            }
            Ok(Some(Entity::new(mapping().new_global_ref(entity)?)))
        })
    }

     
    pub fn is_block_air(&self, pos: &BlockPos) -> anyhow::Result<bool> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let pos_obj = pos.to_java(&mut env)?;
            let state = self
                .call_method("getBlockState", &[JValue::Object(&pos_obj)])?
                .l()?;
            Ok(mapping()
                .call_method(MinecraftClassType::BlockState, &state, "isAir", &[])?
                .z()?)
        })
    }

     
     
     
    pub fn block_shape_boxes(&self, pos: &BlockPos) -> anyhow::Result<Vec<[f64; 6]>> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let pos_obj = pos.to_java(&mut env)?;
            let state = self
                .call_method("getBlockState", &[JValue::Object(&pos_obj)])?
                .l()?;
            crate::mapping::chunk::shape_boxes(&mut env, &state, self.jni_ref().as_obj(), &pos_obj)
        })
    }

     
     
     
     
    pub fn block_collision_boxes(&self, pos: &BlockPos) -> anyhow::Result<Vec<[f64; 6]>> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let pos_obj = pos.to_java(&mut env)?;
            let state = self
                .call_method("getBlockState", &[JValue::Object(&pos_obj)])?
                .l()?;
            crate::mapping::chunk::collision_shape_boxes(
                &mut env,
                &state,
                self.jni_ref().as_obj(),
                &pos_obj,
            )
        })
    }

     
     
    pub fn get_min_y(&self) -> anyhow::Result<i32> {
        self.in_frame(|| Ok(self.call_method("getMinY", &[])?.i()?))
    }

     
     
    pub fn sections_count(&self) -> anyhow::Result<i32> {
        self.in_frame(|| Ok(self.call_method("getSectionsCount", &[])?.i()?))
    }

     
     
     
    pub fn block_registry_name(&self, pos: &BlockPos) -> anyhow::Result<String> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let pos_obj = pos.to_java(&mut env)?;
            let state = self
                .call_method("getBlockState", &[JValue::Object(&pos_obj)])?
                .l()?;
            let block = mapping()
                .call_method(MinecraftClassType::BlockState, &state, "getBlock", &[])?
                .l()?;
            let registry = crate::mapping::chunk::block_registry()?;
            let key = mapping()
                .call_method(
                    MinecraftClassType::Registry,
                    registry.as_obj(),
                    "getKey",
                    &[JValue::Object(&block)],
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

     
     
     
     
    pub fn block_state_int_property(
        &self,
        pos: &BlockPos,
        name: &str,
    ) -> anyhow::Result<Option<i32>> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let pos_obj = pos.to_java(&mut env)?;
            let state = self
                .call_method("getBlockState", &[JValue::Object(&pos_obj)])?
                .l()?;
            let properties = mapping()
                .call_method(
                    MinecraftClassType::StateHolder,
                    &state,
                    "getProperties",
                    &[],
                )?
                .l()?;
            if properties.is_null() {
                return Ok(None);
            }
             
             
             
            let iterable = Iterable::new(mapping().new_global_ref(properties)?);
            let iterator = iterable.iterator()?;
            while iterator.has_next()? {
                let property = iterator.next()?;
                let key = mapping()
                    .call_method(MinecraftClassType::Property, &property, "getName", &[])?
                    .l()?;
                if mapping().get_string(key)? != name {
                    continue;
                }
                let value = mapping()
                    .call_method(
                        MinecraftClassType::StateHolder,
                        &state,
                        "getValue",
                        &[JValue::Object(&property)],
                    )?
                    .l()?;
                return Ok(Some(
                    mapping()
                        .call_method(MinecraftClassType::Integer, &value, "intValue", &[])?
                        .i()?,
                ));
            }
            Ok(None)
        })
    }

     
     
    pub fn rain_level(&self) -> anyhow::Result<f32> {
        self.in_frame(|| {
            Ok(self
                .call_method("getRainLevel", &[JValue::Float(1.0)])?
                .f()?)
        })
    }

     
     
    pub fn set_rain_level(&self, level: f32) -> anyhow::Result<()> {
        self.call_method("setRainLevel", &[JValue::Float(level)])?;
        Ok(())
    }

     
    pub fn thunder_level(&self) -> anyhow::Result<f32> {
        self.in_frame(|| {
            Ok(self
                .call_method("getThunderLevel", &[JValue::Float(1.0)])?
                .f()?)
        })
    }

     
    pub fn set_thunder_level(&self, level: f32) -> anyhow::Result<()> {
        self.call_method("setThunderLevel", &[JValue::Float(level)])?;
        Ok(())
    }

     
     
    pub fn get_chunk(&self, x: i32, z: i32) -> anyhow::Result<Option<LevelChunk>> {
        self.in_frame(|| {
            let chunk = self
                .call_method("getChunk", &[JValue::Int(x), JValue::Int(z)])?
                .l()?;
            if chunk.is_null() {
                return Ok(None);
            }
            Ok(Some(LevelChunk::new(mapping().new_global_ref(chunk)?)))
        })
    }

     
     
     
     
     
     
     
    pub fn add_dust_particles(
        &self,
        r: u8,
        g: u8,
        b: u8,
        scale: f32,
        spawns: &[(f64, f64, f64, f64, f64, f64)],
    ) -> anyhow::Result<()> {
        if spawns.is_empty() {
            return Ok(());
        }

        let mut env = mapping().get_env()?;

         
         
        if env.push_local_frame(32).is_err() {
            let _ = env.exception_clear();
            return Err(anyhow::anyhow!(
                "Failed to push local frame for dust particles"
            ));
        }
        let result = (|| -> anyhow::Result<()> {
            let class_name =
                mapping().runtime_class_name(MinecraftClassType::DustParticleOptions)?;
            let jclass = mapping().resolve_class(&mut env, &class_name)?;
             
            let ctor = env.get_method_id(&jclass, "<init>", "(IF)V").map_err(|_| {
                let _ = env.exception_clear();
                anyhow::anyhow!("DustParticleOptions(int, float) not present")
            })?;
            let color = 0xFF000000u32 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
             
            let options = unsafe {
                env.new_object_unchecked(
                    &jclass,
                    ctor,
                    &[
                        JValue::Int(color as i32).as_jni(),
                        JValue::Float(scale).as_jni(),
                    ],
                )
            }
            .map_err(|_| {
                let _ = env.exception_clear();
                anyhow::anyhow!("failed to construct DustParticleOptions")
            })?;

            let args_list: Vec<Vec<JValue>> = spawns
                .iter()
                .map(|(x, y, z, xd, yd, zd)| {
                    vec![
                        JValue::Object(&options),
                        JValue::Double(*x),
                        JValue::Double(*y),
                        JValue::Double(*z),
                        JValue::Double(*xd),
                        JValue::Double(*yd),
                        JValue::Double(*zd),
                    ]
                })
                .collect();
            mapping().call_method_batch(
                MinecraftClassType::Level,
                self.jni_ref().as_obj(),
                "addParticle",
                &args_list,
            )
        })();
         
         
         
        let _ = unsafe { env.pop_local_frame(&JObject::null()) };
        result
    }

     
     
     
    pub fn play_explosion_sound(
        &self,
        center: [f64; 3],
        volume: f32,
        pitch: f32,
    ) -> anyhow::Result<()> {
        self.in_frame(|| {
            let sound = match mapping().get_static_field(
                MinecraftClassType::SoundEvents,
                "GENERIC_EXPLODE",
                FieldType::Object(MinecraftClassType::SoundEvent),
            ) {
                Ok(value) => value.l()?,
                Err(_) => {
                    let mut env = mapping().get_env()?;
                    let class_name =
                        mapping().runtime_class_name(MinecraftClassType::SoundEvents)?;
                    let jclass = mapping().resolve_class(&mut env, &class_name)?;
                    let field_name = mapping()
                        .runtime_field_name(MinecraftClassType::SoundEvents, "GENERIC_EXPLODE")?;
                    let holder_ref_sig = format!(
                        "L{}$class_6883;",
                        mapping().runtime_class_name(MinecraftClassType::Holder)?
                    );
                    let holder = match env.get_static_field(&jclass, &field_name, &holder_ref_sig) {
                        Ok(value) => value.l()?,
                        Err(_) => {
                            let _ = env.exception_clear();
                            let holder_sig =
                                FieldType::Object(MinecraftClassType::Holder).get_signature()?;
                            env.get_static_field(&jclass, &field_name, &holder_sig)
                                .map_err(|_| {
                                    let _ = env.exception_clear();
                                    anyhow::anyhow!("could not resolve SoundEvents.GENERIC_EXPLODE")
                                })?
                                .l()?
                        }
                    };
                    mapping()
                        .call_method(MinecraftClassType::Holder, &holder, "value", &[])?
                        .l()?
                }
            };
            let source = mapping()
                .get_static_field(
                    MinecraftClassType::SoundSource,
                    "BLOCKS",
                    FieldType::Object(MinecraftClassType::SoundSource),
                )?
                .l()?;
            self.call_method(
                "playLocalSound",
                &[
                    JValue::Double(center[0]),
                    JValue::Double(center[1]),
                    JValue::Double(center[2]),
                    JValue::Object(&sound),
                    JValue::Object(&source),
                    JValue::Float(volume),
                    JValue::Float(pitch),
                    JValue::Bool(0),
                ],
            )?;
            Ok(())
        })
    }
}

 
#[derive(Debug, MappedObject)]
#[mapped(class = LevelChunk)]
pub struct LevelChunk {
    jni_ref: GlobalRef,
}

impl LevelChunk {
     
    pub fn new(jni_ref: GlobalRef) -> LevelChunk {
        LevelChunk { jni_ref }
    }

     
    pub fn get_block_entities(&self) -> anyhow::Result<Vec<BlockEntity>> {
        self.in_frame(|| {
            let map = self.call_method("getBlockEntities", &[])?.l()?;
            if map.is_null() {
                return Ok(Vec::new());
            }

            let values = mapping()
                .call_method(MinecraftClassType::Map, &map, "values", &[])?
                .l()?;
            let iterable = Iterable::new(mapping().new_global_ref(values)?);
            let iterator = iterable.iterator()?;

            let mut block_entities = Vec::new();
            while iterator.has_next()? {
                block_entities.push(BlockEntity::new(iterator.next()?));
            }
            Ok(block_entities)
        })
    }

     
     
     
     
    pub fn get_section(&self, index: i32) -> anyhow::Result<Option<LevelChunkSection>> {
        self.in_frame(|| {
            let obj = self.call_method("getSection", &[JValue::Int(index)])?.l()?;
            if obj.is_null() {
                return Ok(None);
            }
            Ok(Some(LevelChunkSection::new(mapping().new_global_ref(obj)?)))
        })
    }
}
