 
 
 
 
 
 
 
 
 
 

use crate::mapping::java::iterable::Iterable;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JIntArray, JLongArray, JObject, JString, JValue};
use jni::sys::jlong;
use jni::JNIEnv;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

 
#[derive(Debug, MappedObject)]
#[mapped(class = LevelChunkSection)]
pub struct LevelChunkSection {
    jni_ref: GlobalRef,
}

impl LevelChunkSection {
     
    pub fn new(jni_ref: GlobalRef) -> LevelChunkSection {
        LevelChunkSection { jni_ref }
    }

     
    pub fn has_only_air(&self) -> anyhow::Result<bool> {
        self.in_frame(|| Ok(self.call_method("hasOnlyAir", &[])?.z()?))
    }

     
    pub fn states(&self) -> anyhow::Result<PalettedContainer> {
        self.in_frame(|| {
            let obj = self.call_method("getStates", &[])?.l()?;
            Ok(PalettedContainer::new(mapping().new_global_ref(obj)?))
        })
    }
}

 
#[derive(Debug, MappedObject)]
#[mapped(class = PalettedContainer)]
pub struct PalettedContainer {
    jni_ref: GlobalRef,
}

impl PalettedContainer {
     
    pub fn new(jni_ref: GlobalRef) -> PalettedContainer {
        PalettedContainer { jni_ref }
    }

     
     
    pub fn data(&self) -> anyhow::Result<PalettedContainerData> {
        self.in_frame(|| {
            let obj = self
                .get_field(
                    "data",
                    FieldType::Object(MinecraftClassType::PalettedContainerData),
                )?
                .l()?;
            Ok(PalettedContainerData::new(mapping().new_global_ref(obj)?))
        })
    }
}

 
#[derive(Debug, MappedObject)]
#[mapped(class = PalettedContainerData)]
pub struct PalettedContainerData {
    jni_ref: GlobalRef,
}

impl PalettedContainerData {
     
    pub fn new(jni_ref: GlobalRef) -> PalettedContainerData {
        PalettedContainerData { jni_ref }
    }

     
     
    pub fn palette(&self) -> anyhow::Result<GlobalRef> {
        self.in_frame(|| {
            let obj = self.call_method("palette", &[])?.l()?;
            mapping().new_global_ref(obj)
        })
    }

     
    pub fn storage(&self) -> anyhow::Result<GlobalRef> {
        self.in_frame(|| {
            let obj = self.call_method("storage", &[])?.l()?;
            mapping().new_global_ref(obj)
        })
    }
}

 
 
#[derive(Debug, Clone)]
pub struct Palette {
    jni_ref: GlobalRef,
}

impl Palette {
     
    pub fn new(jni_ref: GlobalRef) -> Palette {
        Palette { jni_ref }
    }

     
    pub fn size(&self) -> anyhow::Result<i32> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::Palette,
                self.jni_ref.as_obj(),
                "getSize",
                &[],
            )?
            .i()?)
    }

     
     
    pub fn value_for(&self, index: i32) -> anyhow::Result<JObject<'static>> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::Palette,
                self.jni_ref.as_obj(),
                "valueFor",
                &[JValue::Int(index)],
            )?
            .l()?)
    }
}

 
#[derive(Debug, Clone)]
pub struct BitStorage {
    jni_ref: GlobalRef,
}

impl BitStorage {
     
    pub fn new(jni_ref: GlobalRef) -> BitStorage {
        BitStorage { jni_ref }
    }

     
    pub fn bits(&self) -> anyhow::Result<i32> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::BitStorage,
                self.jni_ref.as_obj(),
                "getBits",
                &[],
            )?
            .i()?)
    }

     
    pub fn size(&self) -> anyhow::Result<i32> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::BitStorage,
                self.jni_ref.as_obj(),
                "getSize",
                &[],
            )?
            .i()?)
    }

     
     
    pub fn get(&self, index: i32) -> anyhow::Result<i32> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::BitStorage,
                self.jni_ref.as_obj(),
                "get",
                &[JValue::Int(index)],
            )?
            .i()?)
    }

     
    pub fn unpack_words(words: &[jlong], bits: i32) -> Vec<i32> {
        if bits <= 0 {
            return vec![0; 4096];
        }
        let values_per_long = 64 / (bits as usize);
        if values_per_long == 0 {
            return vec![0; 4096];
        }
        let mask = (1u64 << bits) - 1;
        let mut values = Vec::with_capacity(4096);
        for i in 0..4096 {
            let word_idx = i / values_per_long;
            let bit_offset = (i % values_per_long) * (bits as usize);
            if word_idx < words.len() {
                let word = words[word_idx] as u64;
                let value = ((word >> bit_offset) & mask) as i32;
                values.push(value);
            } else {
                values.push(0);
            }
        }
        values
    }

     
     
     
    pub fn unpack(&self) -> anyhow::Result<Option<Vec<i32>>> {
        let bits = self.bits()?;
        if bits <= 0 {
            return Ok(Some(Vec::new()));
        }

        if let Ok(Some((words, bits))) = self.raw_words() {
            if !words.is_empty() {
                return Ok(Some(Self::unpack_words(&words, bits)));
            }
        }

        mapping().in_frame(|| {
            let env = mapping().get_env()?;
            let array: JIntArray = match env.new_int_array(4096) {
                Ok(array) => array,
                Err(error) => {
                     
                     
                    let _ = env.exception_clear();
                    return Err(error.into());
                }
            };
            if mapping()
                .call_method(
                    MinecraftClassType::SimpleBitStorage,
                    self.jni_ref.as_obj(),
                    "unpack",
                    &[JValue::Object(array.as_ref())],
                )
                .is_err()
            {
                let _ = env.exception_clear();
                return Ok(None);
            }

            let mut values = vec![0_i32; 4096];
            if let Err(error) = env.get_int_array_region(&array, 0, &mut values) {
                let _ = env.exception_clear();
                return Err(error.into());
            }
            Ok(Some(values))
        })
    }

     
     
     
     
     
    pub fn raw_words(&self) -> anyhow::Result<Option<(Vec<jlong>, i32)>> {
        let bits = self.bits()?;
        if bits <= 0 {
            return Ok(Some((Vec::new(), bits)));
        }

        let raw = mapping().call_method(
            MinecraftClassType::SimpleBitStorage,
            self.jni_ref.as_obj(),
            "getRaw",
            &[],
        );
        let array = match raw {
            Ok(value) => value.l()?,
            Err(_) => {
                 
                 
                 
                if let Ok(env) = mapping().get_env() {
                    let _ = env.exception_clear();
                }
                return Ok(None);
            }
        };
        if array.is_null() {
            return Ok(None);
        }

        let words = mapping().in_frame(|| {
            let mut env = mapping().get_env()?;
            let array = JLongArray::from(array);
            let length = env.get_array_length(&array)? as usize;
            let elements =
                unsafe { env.get_array_elements(&array, jni::objects::ReleaseMode::NoCopyBack)? };
            let words: Vec<jlong> = elements.iter().copied().collect();
            debug_assert_eq!(words.len(), length);
            Ok(words)
        })?;
        Ok(Some((words, bits)))
    }
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
pub fn palette_slot_target(
    env: &mut JNIEnv,
    palette: &Palette,
    index: i32,
    ids: &[String],
    blocks: &[GlobalRef],
    cache: &mut HashMap<i32, Option<usize>>,
) -> anyhow::Result<Option<usize>> {
    if let Some(&known) = cache.get(&index) {
        return Ok(known);
    }
    let state = palette.value_for(index)?;
    let hit = state_target(env, state, ids, blocks)?;
    cache.insert(index, hit);
    Ok(hit)
}

 
 
struct StateTargetEntry {
    state: GlobalRef,
    matched: Option<String>,
}

 
 
static STATE_TARGETS: OnceLock<Mutex<HashMap<i32, Vec<StateTargetEntry>>>> = OnceLock::new();

fn state_targets() -> &'static Mutex<HashMap<i32, Vec<StateTargetEntry>>> {
    STATE_TARGETS.get_or_init(|| Mutex::new(HashMap::new()))
}

 
 
 
pub fn clear_state_target_cache() {
    if let Ok(mut cache) = state_targets().lock() {
        cache.clear();
    }
}

 
 
fn state_target(
    env: &mut JNIEnv,
    state: JObject<'static>,
    ids: &[String],
    blocks: &[GlobalRef],
) -> anyhow::Result<Option<usize>> {
    let hash = object_identity(env, &state)?;

     
    if let Ok(cache) = state_targets().lock() {
        if let Some(entries) = cache.get(&hash) {
            for entry in entries {
                if env.is_same_object(&state, entry.state.as_obj())? {
                    return Ok(entry
                        .matched
                        .as_ref()
                        .and_then(|id| ids.iter().position(|candidate| candidate == id)));
                }
            }
        }
    }

     
    let block = state_block(&state)?;
    let mut matched: Option<String> = None;
    for (position, target) in blocks.iter().enumerate() {
        if env.is_same_object(&block, target.as_obj())? {
            matched = ids.get(position).cloned();
            break;
        }
    }

    if let Ok(mut cache) = state_targets().lock() {
        cache.entry(hash).or_default().push(StateTargetEntry {
            state: mapping().new_global_ref(state)?,
            matched: matched.clone(),
        });
    }
    Ok(matched
        .as_ref()
        .and_then(|id| ids.iter().position(|candidate| candidate == id)))
}

 
 
 
fn object_identity(env: &mut JNIEnv, object: &JObject) -> anyhow::Result<i32> {
    match env.call_static_method(
        "java/lang/System",
        "identityHashCode",
        "(Ljava/lang/Object;)I",
        &[JValue::Object(object)],
    ) {
        Ok(value) => Ok(value.i()?),
        Err(error) => {
            let _ = env.exception_clear();
            Err(error.into())
        }
    }
}

 
pub fn palette_identity(env: &mut JNIEnv, palette: &Palette) -> anyhow::Result<i32> {
    object_identity(env, palette.jni_ref.as_obj())
}

 
struct PaletteTargetEntry {
     
     
    palette: GlobalRef,
     
     
    slots: HashMap<i32, Option<usize>>,
     
     
     
     
    no_target_size: Option<i32>,
}

 
 
 
 
 
 
static PALETTE_TARGETS: OnceLock<Mutex<HashMap<i32, PaletteTargetEntry>>> = OnceLock::new();

const PALETTE_TARGET_CACHE_MAX: usize = 4096;

fn palette_targets() -> &'static Mutex<HashMap<i32, PaletteTargetEntry>> {
    PALETTE_TARGETS.get_or_init(|| Mutex::new(HashMap::new()))
}

 
 
 
pub fn clear_palette_target_cache() {
    if let Ok(mut cache) = palette_targets().lock() {
        cache.clear();
    }
}

 
 
 
pub fn palette_target_cache_get(
    palette: &Palette,
    hash: i32,
) -> Option<(HashMap<i32, Option<usize>>, Option<i32>)> {
    let cache = palette_targets().lock().ok()?;
    let entry = cache.get(&hash)?;
    let env = mapping().get_env().ok()?;
    let same = env
        .is_same_object(palette.jni_ref.as_obj(), entry.palette.as_obj())
        .unwrap_or(false);
    if !same {
        return None;
    }
    Some((entry.slots.clone(), entry.no_target_size))
}

 
 
pub fn palette_target_cache_put(
    palette: &Palette,
    hash: i32,
    slots: HashMap<i32, Option<usize>>,
    no_target_size: Option<i32>,
) {
    let Ok(mut cache) = palette_targets().lock() else {
        return;
    };
     
     
    let same = cache.get(&hash).is_some_and(|entry| {
        mapping()
            .get_env()
            .ok()
            .and_then(|env| {
                env.is_same_object(palette.jni_ref.as_obj(), entry.palette.as_obj())
                    .ok()
            })
            .unwrap_or(false)
    });
    if !same {
        if cache.len() >= PALETTE_TARGET_CACHE_MAX {
            cache.clear();
        }
        cache.insert(
            hash,
            PaletteTargetEntry {
                palette: palette.jni_ref.clone(),
                slots: HashMap::new(),
                no_target_size: None,
            },
        );
    }
    let entry = cache.get_mut(&hash).expect("entry just ensured");
    entry.slots.extend(slots);
    if no_target_size.is_some() {
        entry.no_target_size = no_target_size;
    }
}

 
 
pub fn state_block(state: &JObject) -> anyhow::Result<JObject<'static>> {
    Ok(mapping()
        .call_method(MinecraftClassType::BlockStateBase, state, "getBlock", &[])?
        .l()?)
}

 
fn axes() -> anyhow::Result<&'static [GlobalRef; 3]> {
    static AXES: std::sync::OnceLock<Option<[GlobalRef; 3]>> = std::sync::OnceLock::new();
    let cached = AXES.get_or_init(|| {
        mapping()
            .in_frame(|| {
                let axis = |name: &str| -> anyhow::Result<GlobalRef> {
                    let obj = mapping()
                        .get_static_field(
                            MinecraftClassType::DirectionAxis,
                            name,
                            FieldType::Object(MinecraftClassType::DirectionAxis),
                        )?
                        .l()?;
                    mapping().new_global_ref(obj)
                };
                Ok([axis("X")?, axis("Y")?, axis("Z")?])
            })
            .ok()
    });
    cached
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Direction.Axis constants unavailable"))
}

 
 
 
 
pub fn shape_boxes(
    env: &mut JNIEnv,
    state: &JObject,
    world: &JObject,
    pos: &JObject,
) -> anyhow::Result<Vec<[f64; 6]>> {
    shape_boxes_of(env, state, world, pos, "getShape")
}

 
 
 
 
 
pub fn collision_shape_boxes(
    env: &mut JNIEnv,
    state: &JObject,
    world: &JObject,
    pos: &JObject,
) -> anyhow::Result<Vec<[f64; 6]>> {
    shape_boxes_of(env, state, world, pos, "getCollisionShape")
}

 
 
fn shape_boxes_of(
    env: &mut JNIEnv,
    state: &JObject,
    world: &JObject,
    pos: &JObject,
    method: &str,
) -> anyhow::Result<Vec<[f64; 6]>> {
    let shape = mapping()
        .call_method(
            MinecraftClassType::BlockStateBase,
            state,
            method,
            &[JValue::Object(world), JValue::Object(pos)],
        )?
        .l()?;
    if shape.is_null()
        || mapping()
            .call_method(MinecraftClassType::VoxelShape, &shape, "isEmpty", &[])?
            .z()?
    {
        return Ok(Vec::new());
    }

    let axes = axes()?;
    let list = mapping()
        .call_method(MinecraftClassType::VoxelShape, &shape, "toAabbs", &[])?
        .l()?;
    let count = env.call_method(&list, "size", "()I", &[])?.i()?.max(0);
    let mut boxes = Vec::with_capacity(count as usize);
    for index in 0..count {
        let aabb = env
            .call_method(&list, "get", "(I)Ljava/lang/Object;", &[JValue::Int(index)])?
            .l()?;
        let mut bounds = [0.0f64; 6];
        for (axis_index, axis) in axes.iter().enumerate() {
            bounds[axis_index] = mapping()
                .call_method(
                    MinecraftClassType::Aabb,
                    &aabb,
                    "min",
                    &[JValue::Object(axis.as_obj())],
                )?
                .d()?;
            bounds[axis_index + 3] = mapping()
                .call_method(
                    MinecraftClassType::Aabb,
                    &aabb,
                    "max",
                    &[JValue::Object(axis.as_obj())],
                )?
                .d()?;
        }
        boxes.push(bounds);
    }
    Ok(boxes)
}

 
struct StateShapeEntry {
    state: GlobalRef,
    boxes: Vec<[f64; 6]>,
}

 
 
 
 
static STATE_SHAPES: OnceLock<Mutex<HashMap<i32, Vec<StateShapeEntry>>>> = OnceLock::new();

 
 
const STATE_SHAPE_CACHE_MAX: usize = 32_768;

fn state_shapes() -> &'static Mutex<HashMap<i32, Vec<StateShapeEntry>>> {
    STATE_SHAPES.get_or_init(|| Mutex::new(HashMap::new()))
}

 
 
 
 
 
 
 
 
pub fn block_shape(
    env: &mut JNIEnv,
    state: &JObject,
    world: &JObject,
    pos: &JObject,
) -> anyhow::Result<Vec<[f64; 6]>> {
    let hash = object_identity(env, state)?;
    if let Ok(cache) = state_shapes().lock() {
        if let Some(entries) = cache.get(&hash) {
            for entry in entries {
                if env.is_same_object(state, entry.state.as_obj())? {
                    return Ok(entry.boxes.clone());
                }
            }
        }
    }
    let boxes = shape_boxes(env, state, world, pos)?;
    if let Ok(mut cache) = state_shapes().lock() {
        if cache.len() >= STATE_SHAPE_CACHE_MAX {
            cache.clear();
        }
        cache.entry(hash).or_default().push(StateShapeEntry {
            state: env.new_global_ref(state)?,
            boxes: boxes.clone(),
        });
    }
    Ok(boxes)
}

 
 
 
pub fn block_registry() -> anyhow::Result<GlobalRef> {
    mapping().in_frame(|| {
        let obj = mapping()
            .get_static_field(
                MinecraftClassType::BuiltInRegistries,
                "BLOCK",
                FieldType::Object(MinecraftClassType::DefaultedRegistry),
            )
            .or_else(|_| {
                mapping().get_static_field(
                    MinecraftClassType::BuiltInRegistries,
                    "BLOCK",
                    FieldType::Object(MinecraftClassType::Registry),
                )
            })?
            .l()?;
        mapping().new_global_ref(obj)
    })
}

 
 
pub fn item_registry() -> anyhow::Result<GlobalRef> {
    mapping().in_frame(|| {
        let obj = mapping()
            .get_static_field(
                MinecraftClassType::BuiltInRegistries,
                "ITEM",
                FieldType::Object(MinecraftClassType::DefaultedRegistry),
            )
            .or_else(|_| {
                mapping().get_static_field(
                    MinecraftClassType::BuiltInRegistries,
                    "ITEM",
                    FieldType::Object(MinecraftClassType::Registry),
                )
            })?
            .l()?;
        mapping().new_global_ref(obj)
    })
}

 
 
 
pub fn registered_block_ids() -> anyhow::Result<Vec<String>> {
    let registry = block_registry()?;
    mapping().in_frame(|| {
        let set = mapping()
            .call_method(
                MinecraftClassType::Registry,
                registry.as_obj(),
                "keySet",
                &[],
            )?
            .l()?;
        let iterable = Iterable::new(mapping().new_global_ref(set)?);
        let iterator = iterable.iterator()?;

        let mut env = mapping().get_env()?;
        let mut ids = Vec::new();
        while iterator.has_next()? {
            let resource = iterator.next()?;
            let text = env
                .call_method(resource.as_obj(), "toString", "()Ljava/lang/String;", &[])?
                .l()?;
            ids.push(env.get_string(&JString::from(text))?.to_str()?.to_string());
        }
        ids.sort();
        ids.dedup();
        Ok(ids)
    })
}
