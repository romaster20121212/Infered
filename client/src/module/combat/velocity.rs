use crate::mapping::entity::EntityRef;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::net::packet::clientbound_set_entity_motion::ClientboundSetEntityMotionPacket;
use crate::net::packet::{Packet, PacketAction};
use crate::state::minecraft;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

 
const UNKNOWN_ID: i32 = i32::MIN;

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct VelocityModule {
    pub module: ModuleData,
     
     
     
    local_id: AtomicI32,
     
     
    horizontal_bits: std::sync::atomic::AtomicU32,
     
    horizontal_collision: AtomicBool,
}

impl VelocityModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Velocity,
                description: "Holds knockback back while hitting a wall".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyV,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Horizontal".to_string(),
                    value: 0.0,
                    min: 0.0,
                    max: 100.0,
                }],
            },
            local_id: AtomicI32::new(UNKNOWN_ID),
            horizontal_bits: std::sync::atomic::AtomicU32::new(0.0_f32.to_bits()),
            horizontal_collision: AtomicBool::new(false),
        }
    }

     
    fn factor(&self, name: &str) -> f64 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(0.0) as f64
            / 100.0
    }

     
     
    fn snapshot_factors(&self) {
        self.horizontal_bits.store(
            (self.factor("Horizontal") as f32).to_bits(),
            Ordering::Relaxed,
        );
    }
}

impl Module for VelocityModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.horizontal_collision.store(false, Ordering::Relaxed);
        self.snapshot_factors();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.local_id.store(UNKNOWN_ID, Ordering::Relaxed);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        let id = match minecraft().player()? {
            Some(player) => player.id()?,
            None => UNKNOWN_ID,
        };
        self.local_id.store(id, Ordering::Relaxed);
        self.snapshot_factors();
        Ok(())
    }

    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        match packet {
            Packet::ServerboundMovePlayer(movement) => {
                self.horizontal_collision
                    .store(movement.horizontal_collision, Ordering::Relaxed);
            }
            Packet::ClientboundSetEntityMotion(motion) => {
                if motion.entity_id == self.local_id.load(Ordering::Relaxed)
                    && self.horizontal_collision.load(Ordering::Relaxed)
                {
                    let horizontal =
                        f32::from_bits(self.horizontal_bits.load(Ordering::Relaxed)) as f64;
                    scale_knockback(motion, horizontal);
                }
            }
            _ => {}
        }
        PacketAction::Forward
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 
 
fn scale_knockback(motion: &mut ClientboundSetEntityMotionPacket, horizontal: f64) {
    motion.x *= horizontal;
    motion.z *= horizontal;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::packet::serverbound_move_player::ServerboundMovePlayerPacket;

    const LOCAL_ID: i32 = 7;

    fn motion_packet(entity_id: i32) -> Packet {
        Packet::ClientboundSetEntityMotion(ClientboundSetEntityMotionPacket {
            entity_id,
            x: 1.0,
            y: 0.5,
            z: -1.0,
        })
    }

    fn motion_of(packet: &Packet) -> (f64, f64, f64) {
        match packet {
            Packet::ClientboundSetEntityMotion(motion) => (motion.x, motion.y, motion.z),
            other => panic!("expected a motion packet, got {other:?}"),
        }
    }

    fn movement_packet(horizontal_collision: bool) -> Packet {
        Packet::ServerboundMovePlayer(ServerboundMovePlayerPacket {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
            horizontal_collision,
            has_position: true,
            has_rotation: false,
        })
    }

     
    fn armed(horizontal: f32) -> VelocityModule {
        let mut module = VelocityModule::new();
        if let Some(ModuleSetting::Slider { value, .. }) =
            module.module.get_setting_mut("Horizontal")
        {
            *value = horizontal;
        }
        module.local_id.store(LOCAL_ID, Ordering::Relaxed);
        module.snapshot_factors();
        module
    }

     
     
    #[test]
    fn knockback_passes_through_without_a_wall_collision() {
        let module = armed(0.0);
        let mut packet = motion_packet(LOCAL_ID);
        assert_eq!(module.handle_packet(&mut packet), PacketAction::Forward);
        assert_eq!(motion_of(&packet), (1.0, 0.5, -1.0));
    }

     
     
    #[test]
    fn a_wall_collision_lets_the_horizontal_share_through() {
        let module = armed(50.0);
        let mut movement = movement_packet(true);
        module.handle_packet(&mut movement);
        let mut packet = motion_packet(LOCAL_ID);
        module.handle_packet(&mut packet);
        assert_eq!(motion_of(&packet), (0.5, 0.5, -0.5));
    }

     
     
    #[test]
    fn a_clean_movement_packet_closes_the_gate() {
        let module = armed(0.0);
        let mut colliding = movement_packet(true);
        module.handle_packet(&mut colliding);
        let mut clean = movement_packet(false);
        module.handle_packet(&mut clean);
        let mut packet = motion_packet(LOCAL_ID);
        module.handle_packet(&mut packet);
        assert_eq!(motion_of(&packet), (1.0, 0.5, -1.0));
    }

    #[test]
    fn other_entities_keep_their_motion() {
        let module = armed(0.0);
        let mut movement = movement_packet(true);
        module.handle_packet(&mut movement);
        let mut packet = motion_packet(LOCAL_ID + 1);
        module.handle_packet(&mut packet);
        assert_eq!(motion_of(&packet), (1.0, 0.5, -1.0));
    }

    #[test]
    fn nothing_is_touched_before_the_local_id_is_known() {
        let module = armed(0.0);
        let mut movement = movement_packet(true);
        module.handle_packet(&mut movement);
        module.local_id.store(UNKNOWN_ID, Ordering::Relaxed);
        let mut packet = motion_packet(LOCAL_ID);
        module.handle_packet(&mut packet);
        assert_eq!(motion_of(&packet), (1.0, 0.5, -1.0));
    }
}
