 
 
 
 
 
 
 
 
 
 
 
 

use std::sync::{Arc, Mutex};

use rhai::{Dynamic, Engine, Map};

use crate::net::packet::clientbound_explode::ClientboundExplodePacket;
use crate::net::packet::clientbound_set_entity_motion::ClientboundSetEntityMotionPacket;
use crate::net::packet::serverbound_move_player::ServerboundMovePlayerPacket;
use crate::net::packet::serverbound_use_item::ServerboundUseItemPacket;
use crate::net::packet::Packet;

 
 
#[derive(Debug, Clone)]
pub struct Slot {
    pub direction: &'static str,
    pub kind: &'static str,
    pub fields: Map,
    pub cancelled: bool,
}

 
pub type PacketSlot = Arc<Mutex<Option<Slot>>>;

 
pub fn register(engine: &mut Engine, slot: PacketSlot) {
    let read = Arc::clone(&slot);
    engine.register_fn("packet_kind", move || {
        read.lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(|slot| slot.kind.to_string()))
            .unwrap_or_default()
    });
    let read = Arc::clone(&slot);
    engine.register_fn("packet_direction", move || {
        read.lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(|slot| slot.direction.to_string()))
            .unwrap_or_default()
    });
    let read = Arc::clone(&slot);
    engine.register_fn("packet_has", move |name: &str| {
        read.lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(|slot| slot.fields.contains_key(name)))
            .unwrap_or(false)
    });
    let read = Arc::clone(&slot);
    engine.register_fn("packet_field", move |name: &str| {
        read.lock()
            .ok()
            .and_then(|slot| slot.as_ref()?.fields.get(name).cloned())
            .and_then(|value| value.try_cast::<f64>())
            .unwrap_or(0.0)
    });
    let read = Arc::clone(&slot);
    engine.register_fn("packet_flag", move |name: &str| {
        read.lock()
            .ok()
            .and_then(|slot| slot.as_ref()?.fields.get(name).cloned())
            .and_then(|value| value.try_cast::<bool>())
            .unwrap_or(false)
    });
    let write = Arc::clone(&slot);
    engine.register_fn("packet_set", move |name: &str, value: f64| {
        if let Ok(mut guard) = write.lock() {
            if let Some(slot) = guard.as_mut() {
                slot.fields.insert(name.into(), Dynamic::from_float(value));
            }
        }
    });
    let write = Arc::clone(&slot);
    engine.register_fn("packet_set_flag", move |name: &str, value: bool| {
        if let Ok(mut guard) = write.lock() {
            if let Some(slot) = guard.as_mut() {
                slot.fields.insert(name.into(), Dynamic::from_bool(value));
            }
        }
    });
    let write = Arc::clone(&slot);
    engine.register_fn("packet_cancel", move || {
        if let Ok(mut guard) = write.lock() {
            if let Some(slot) = guard.as_mut() {
                slot.cancelled = true;
            }
        }
    });
}

 
 
pub fn dispatch(module: &super::ScriptedModule, packet: &mut Packet) -> bool {
    let Some(slot) = view(packet) else {
        return false;
    };
    let before = slot.fields.clone();

     
     
    {
        let mut guard = match module.packet_slot().lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = Some(slot);
    }

    let ran = module.call_script("on_packet");

    let slot = module
        .packet_slot()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());

    let Some(slot) = slot else {
        return false;
    };
    if !ran {
        return false;
    }
    if slot.cancelled {
        return true;
    }
     
     
    if !fields_equal(&slot.fields, &before) {
        write_back(packet, &slot.fields);
    }
    false
}

 
 
 
fn fields_equal(left: &Map, right: &Map) -> bool {
    left.len() == right.len()
        && left.iter().all(|(name, value)| {
            let Some(other) = right.get(name) else {
                return false;
            };
            if let (Some(a), Some(b)) = (
                value.clone().try_cast::<f64>(),
                other.clone().try_cast::<f64>(),
            ) {
                return a.to_bits() == b.to_bits();
            }
            if let (Some(a), Some(b)) = (
                value.clone().try_cast::<rhai::INT>(),
                other.clone().try_cast::<rhai::INT>(),
            ) {
                return a == b;
            }
            if let (Some(a), Some(b)) = (
                value.clone().try_cast::<bool>(),
                other.clone().try_cast::<bool>(),
            ) {
                return a == b;
            }
            false
        })
}

 
fn view(packet: &Packet) -> Option<Slot> {
    let (direction, kind, fields) = match packet {
        Packet::ServerboundMovePlayer(inner) => {
            ("outbound", "ServerboundMovePlayer", move_fields(*inner))
        }
        Packet::ServerboundUseItem(inner) => {
            ("outbound", "ServerboundUseItem", use_item_fields(*inner))
        }
        Packet::ClientboundSetEntityMotion(inner) => (
            "inbound",
            "ClientboundSetEntityMotion",
            motion_fields(*inner),
        ),
         
        Packet::ServerboundSwing(_) => ("outbound", "ServerboundSwing", Map::new()),
        Packet::ServerboundInteract(_) => ("outbound", "ServerboundInteract", Map::new()),
        Packet::ClientboundPlayerInfoUpdate(_) => {
            ("inbound", "ClientboundPlayerInfoUpdate", Map::new())
        }
        Packet::ClientboundPlayerPosition(_) => {
            ("inbound", "ClientboundPlayerPosition", Map::new())
        }
        Packet::ClientboundBlockUpdate(_) => ("inbound", "ClientboundBlockUpdate", Map::new()),
        Packet::ClientboundPlayerRotation(_) => {
            ("inbound", "ClientboundPlayerRotation", Map::new())
        }
        Packet::ClientboundExplode(inner) => {
            ("inbound", "ClientboundExplode", explosion_fields(inner))
        }
        Packet::ClientboundCustomPayload(_) => ("inbound", "ClientboundCustomPayload", Map::new()),
        Packet::ClientboundSystemChat(_) => ("inbound", "ClientboundSystemChat", Map::new()),
        Packet::ClientboundPlayerChat(_) => ("inbound", "ClientboundPlayerChat", Map::new()),
    };
    Some(Slot {
        direction,
        kind,
        fields,
        cancelled: false,
    })
}

 
fn explosion_fields(packet: &ClientboundExplodePacket) -> Map {
    Map::from_iter([
        ("x".into(), Dynamic::from_float(packet.center[0])),
        ("y".into(), Dynamic::from_float(packet.center[1])),
        ("z".into(), Dynamic::from_float(packet.center[2])),
         
         
         
        (
            "radius".into(),
            Dynamic::from_float(packet.radius.unwrap_or(0.0) as f64),
        ),
    ])
}

fn move_fields(packet: ServerboundMovePlayerPacket) -> Map {
    Map::from_iter([
        ("x".into(), Dynamic::from_float(packet.x)),
        ("y".into(), Dynamic::from_float(packet.y)),
        ("z".into(), Dynamic::from_float(packet.z)),
        ("yaw".into(), Dynamic::from_float(packet.y_rot as f64)),
        ("pitch".into(), Dynamic::from_float(packet.x_rot as f64)),
        ("on_ground".into(), Dynamic::from_bool(packet.on_ground)),
        (
            "collision".into(),
            Dynamic::from_bool(packet.horizontal_collision),
        ),
    ])
}

fn use_item_fields(packet: ServerboundUseItemPacket) -> Map {
    Map::from_iter([
        ("yaw".into(), Dynamic::from_float(packet.y_rot as f64)),
        ("pitch".into(), Dynamic::from_float(packet.x_rot as f64)),
        ("sequence".into(), Dynamic::from_int(packet.sequence.into())),
        ("main_hand".into(), Dynamic::from_bool(packet.main_hand)),
    ])
}

fn motion_fields(packet: ClientboundSetEntityMotionPacket) -> Map {
    Map::from_iter([
        ("id".into(), Dynamic::from_int(packet.entity_id.into())),
        ("x".into(), Dynamic::from_float(packet.x)),
        ("y".into(), Dynamic::from_float(packet.y)),
        ("z".into(), Dynamic::from_float(packet.z)),
    ])
}

 
fn write_back(packet: &mut Packet, fields: &Map) {
    fn number(fields: &Map, name: &str) -> f64 {
        fields
            .get(name)
            .and_then(|value| value.clone().try_cast::<f64>())
            .unwrap_or(0.0)
    }
    fn flag(fields: &Map, name: &str) -> bool {
        fields
            .get(name)
            .and_then(|value| value.clone().try_cast::<bool>())
            .unwrap_or(false)
    }

    match packet {
        Packet::ServerboundMovePlayer(inner) => {
            inner.x = number(fields, "x");
            inner.y = number(fields, "y");
            inner.z = number(fields, "z");
            inner.y_rot = number(fields, "yaw") as f32;
            inner.x_rot = number(fields, "pitch") as f32;
            inner.on_ground = flag(fields, "on_ground");
            inner.horizontal_collision = flag(fields, "collision");
        }
        Packet::ServerboundUseItem(inner) => {
            inner.y_rot = number(fields, "yaw") as f32;
            inner.x_rot = number(fields, "pitch") as f32;
        }
        Packet::ClientboundSetEntityMotion(inner) => {
            inner.x = number(fields, "x");
            inner.y = number(fields, "y");
            inner.z = number(fields, "z");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> Map {
        Map::from_iter([
            ("x".into(), Dynamic::from_float(1.5)),
            ("id".into(), Dynamic::from_int(42)),
            ("on_ground".into(), Dynamic::from_bool(true)),
            ("nan".into(), Dynamic::from_float(f64::NAN)),
        ])
    }

    #[test]
    fn packet_fields_unchanged_values_do_not_request_rebuild() {
        let original = fields();
        assert!(fields_equal(&original, &original.clone()));
        assert!(fields_equal(&Map::new(), &Map::new()));
    }

    #[test]
    fn packet_fields_detect_value_key_and_type_changes() {
        let original = fields();
        for (key, value) in [
            ("x", Dynamic::from_float(2.0)),
            ("id", Dynamic::from_int(43)),
            ("on_ground", Dynamic::from_bool(false)),
            ("id", Dynamic::from_float(42.0)),
            ("new", Dynamic::from_float(1.0)),
        ] {
            let mut changed = original.clone();
            changed.insert(key.into(), value);
            assert!(!fields_equal(&original, &changed), "changed field {key}");
        }
        let mut changed = original.clone();
        changed.remove("x");
        assert!(!fields_equal(&original, &changed));
    }
}
