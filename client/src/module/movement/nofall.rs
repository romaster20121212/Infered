use crate::mapping::entity::EntityRef;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::net::packet::{Packet, PacketAction};

 
 
 
 
 
#[derive(Debug)]
pub struct NoFallModule {
    pub module: ModuleData,
}

impl NoFallModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoFall,
                description: "Prevents fall damage".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyN,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::packet::serverbound_move_player::ServerboundMovePlayerPacket;
    #[test]
    fn nofall_preserves_movement_and_collision_for_every_packet_variant() {
        let module = NoFallModule::new();
        for has_position in [false, true] {
            for has_rotation in [false, true] {
                let original = ServerboundMovePlayerPacket {
                    x: 12.0,
                    y: 64.0,
                    z: -9.0,
                    y_rot: 45.0,
                    x_rot: 20.0,
                    on_ground: false,
                    horizontal_collision: true,
                    has_position,
                    has_rotation,
                };
                let mut packet = Packet::ServerboundMovePlayer(original);
                assert_eq!(module.handle_packet(&mut packet), PacketAction::Forward);
                let Packet::ServerboundMovePlayer(actual) = packet else {
                    panic!("packet changed type")
                };
                assert!(actual.on_ground);
                assert_eq!(
                    ServerboundMovePlayerPacket {
                        on_ground: false,
                        ..actual
                    },
                    original
                );
            }
        }
    }
}

impl Module for NoFallModule {
    fn on_start(&self) -> anyhow::Result<()> {
        log::info!("[NoFall] enabled: outgoing movement ground-state protection");
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        if let Some(player) = crate::state::minecraft().player()? {
            player.reset_fall_distance()?;
        }
        Ok(())
    }

    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
         
         
        if let Packet::ServerboundMovePlayer(move_packet) = packet {
            move_packet.on_ground = true;
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
