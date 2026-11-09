 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::net::packet::{Packet, PacketAction};

 
 
 
 
fn action_for(forced_rotation: bool) -> PacketAction {
    if forced_rotation {
        PacketAction::Cancel
    } else {
        PacketAction::Forward
    }
}

 
fn is_forced_rotation(packet: &Packet) -> bool {
    matches!(packet, Packet::ClientboundPlayerRotation(_))
}

 
#[derive(Debug)]
pub struct NoRotateModule {
    pub module: ModuleData,
}

impl NoRotateModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoRotate,
                description: "Ignores server-forced view rotations and keeps your aim".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for NoRotateModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        action_for(is_forced_rotation(packet))
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_ships_off_and_setting_free() {
        let module = NoRotateModule::new();
        assert_eq!(module.module.id, ModuleId::NoRotate);
        assert_eq!(module.module.category, ModuleCategory::Movement);
        assert!(!module.module.enabled);
        assert!(module.module.settings.is_empty());
    }

    #[test]
    fn a_server_forced_rotation_is_dropped() {
        assert_eq!(action_for(true), PacketAction::Cancel);
    }

    #[test]
    fn everything_else_still_passes_through() {
        assert_eq!(
            action_for(false),
            PacketAction::Forward,
            "position corrections and outbound movement must never be cancelled"
        );
    }
}
