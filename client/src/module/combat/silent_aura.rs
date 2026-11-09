use crate::mapping::entity::EntityRef;
use crate::mapping::{FieldType, MappedObject};
use crate::module::combat::aura::{AuraTarget, BaseAura};
use crate::module::combat::rotation::{align_yaw, silent_rotation};
use crate::module::{KeyboardKey, Module, ModuleData, ModuleId, ModuleSetting};
use crate::net::packet::{Packet, PacketAction};
use crate::state::minecraft;
use jni::objects::JValue;
use std::sync::Mutex;

 
 
 
 
 
 
 
 
 
const AIMBOT_MAX_SPEED: f32 = 20.0;

 
 
 
 
const NOT_OFFERED: &[&str] = &[
    "Actual Mouse",
    "Auto Jump",
    "Auto Walk",
    "Walk Range",
    "Adaptive Strafe",
];

 
 
 
fn pin_aim_speed(module: &mut ModuleData) {
    if let Some(ModuleSetting::Slider {
        value, min, max, ..
    }) = module.get_setting_mut("Speed")
    {
        *value = AIMBOT_MAX_SPEED;
        *min = AIMBOT_MAX_SPEED;
        *max = AIMBOT_MAX_SPEED;
    }
}

 
fn offered(setting: &ModuleSetting) -> bool {
    let name = match setting {
        ModuleSetting::Toggle { name, .. }
        | ModuleSetting::Slider { name, .. }
        | ModuleSetting::Choice { name, .. }
        | ModuleSetting::Color { name, .. }
        | ModuleSetting::BlockList { name, .. }
        | ModuleSetting::Text { name, .. } => name,
    };
    !NOT_OFFERED.contains(&name.as_str())
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct SilentAuraModule {
    pub aura: BaseAura,
     
     
    wire_yaw: Mutex<Option<f32>>,
}

impl SilentAuraModule {
    pub fn new() -> Self {
        let mut aura = BaseAura::new(
            ModuleId::SilentAura,
            "Attacks players with a silent aim the camera never shows".to_string(),
            KeyboardKey::KeyNone,
            AuraTarget::Players,
            true,
        );
        aura.module.settings.retain(offered);
        pin_aim_speed(&mut aura.module);
        Self {
            aura,
            wire_yaw: Mutex::new(None),
        }
    }

    fn silent_aim(&self) -> bool {
        self.aura
            .module
            .get_setting("Silent Aim")
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(false)
    }

     
     
     
    fn guard_mining(&self) -> anyhow::Result<()> {
        let game = minecraft();
        if !self.silent_aim()
            || !self.aura.holds_silent_aim()
            || silent_rotation().is_none()
            || game.current_screen().is_open()
            || game.player()?.is_none()
        {
            return Ok(());
        }
        prevent_vanilla_mining()
    }

     
     
     
    fn hand_back_yaw(&self) -> anyhow::Result<()> {
        let last = *self.wire_yaw.lock().unwrap_or_else(|p| p.into_inner());
        if let (Some(last), Some(player)) = (last, minecraft().player()?) {
            let yaw = player.get_yaw()?;
            let aligned = align_yaw(yaw, last);
            if yaw != aligned {
                player.set_rotation(aligned, player.get_pitch()?)?;
            }
        }
        Ok(())
    }
}

fn prevent_vanilla_mining() -> anyhow::Result<()> {
    let game = minecraft();
    let Some(mode) = game.game_mode()? else {
        return Ok(());
    };
     
     
    mode.stop_destroy_block()?;
     
     
     
    if game.hit_block()?.is_none() {
        return Ok(());
    }
     
     
     
     
    let delay = game.get_field("missTime", FieldType::Int)?.i()?;
    game.set_field("missTime", FieldType::Int, JValue::Int(delay.max(1)))
}

 
 
 
fn continuous_yaw(last: &mut Option<f32>, yaw: f32) -> f32 {
    let aligned = last.map_or(yaw, |reference| align_yaw(yaw, reference));
    *last = Some(aligned);
    aligned
}

impl Module for SilentAuraModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
         
        *self.wire_yaw.lock().unwrap_or_else(|p| p.into_inner()) = minecraft()
            .player()?
            .and_then(|player| player.get_yaw().ok());
        self.aura.on_start()
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.aura.on_stop()?;
        let result = self.hand_back_yaw();
        *self.wire_yaw.lock().unwrap_or_else(|p| p.into_inner()) = None;
        result
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        self.aura.on_tick()
    }

    fn on_input_phase(&self) -> anyhow::Result<()> {
         
         
         
         
         
         
         
        self.guard_mining()?;
         
         
         
        let use_key = minecraft().options()?.key_use()?;
        if use_key.is_down()? || use_key.get_field("clickCount", FieldType::Int)?.i()? > 0 {
            return Ok(());
        }
        self.aura.on_input_phase()
    }

    fn on_render(&self) {
        self.aura.on_render()
    }

    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        let action = self.aura.handle_packet(packet);
        match packet {
            Packet::ServerboundMovePlayer(snapshot) if snapshot.has_rotation => {
                let mut last = self.wire_yaw.lock().unwrap_or_else(|p| p.into_inner());
                 
                 
                 
                snapshot.y_rot = continuous_yaw(&mut last, snapshot.y_rot);
            }
            Packet::ClientboundPlayerPosition(_) | Packet::ClientboundPlayerRotation(_) => {
                 
                *self.wire_yaw.lock().unwrap_or_else(|p| p.into_inner()) = None;
            }
            _ => {}
        }
        action
    }

    fn get_module_data(&self) -> &ModuleData {
        self.aura.get_module_data()
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        self.aura.get_module_data_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::combat::rotation::wrap_degrees;
    use crate::net::packet::serverbound_move_player::ServerboundMovePlayerPacket;

    fn module_without_packet_fallback() -> SilentAuraModule {
        let mut module = SilentAuraModule::new();
        if let Some(ModuleSetting::Toggle { value, .. }) =
            module.get_module_data_mut().get_setting_mut("Silent Aim")
        {
            *value = false;
        }
        module
    }

    #[test]
    fn aim_speed_is_pinned_to_aimbots_maximum() {
        let module = SilentAuraModule::new();
        let Some(ModuleSetting::Slider {
            value, min, max, ..
        }) = module.get_module_data().get_setting("Speed")
        else {
            panic!("Silent Aura must expose a Speed slider");
        };
        assert_eq!(*value, 20.0);
        assert_eq!(*min, 20.0);
        assert_eq!(*max, 20.0);
         
         
        assert_eq!(7.0_f32.clamp(*min, *max), 20.0);
    }

    #[test]
    fn faster_aim_keeps_the_silent_aura_settings_unchanged() {
        let module = SilentAuraModule::new();
        let names: Vec<&str> = module
            .get_module_data()
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(
            names,
            vec![
                "Speed",
                "Cooldown",
                "Shield Check",
                "Prediction",
                "Combat Debug",
                "Silent Aim",
                "Visual Silent",
            ]
        );
         
         
         
        for name in [
            "Delay (ms)",
            "Random Delay",
            "Min Delay (ms)",
            "Max Delay (ms)",
            "Reaction Delay (ms)",
            "Hit Chance (%)",
        ] {
            assert!(
                module.get_module_data().get_setting(name).is_none(),
                "Silent Aura must not gate hits on {name}"
            );
        }
    }

    fn movement(yaw: f32, has_rotation: bool) -> Packet {
        Packet::ServerboundMovePlayer(ServerboundMovePlayerPacket {
            x: 12.0,
            y: 64.0,
            z: -9.0,
            y_rot: yaw,
            x_rot: 17.0,
            on_ground: true,
            horizontal_collision: false,
            has_position: true,
            has_rotation,
        })
    }

    #[test]
    fn consecutive_hits_cross_the_canonical_seam_without_a_full_turn() {
        for (input, expected) in [
            ([178.0, 179.0, -179.0], [178.0, 179.0, 181.0]),
            ([-178.0, -179.0, 179.0], [-178.0, -179.0, -181.0]),
        ] {
            let mut last = None;
            let output = input.map(|yaw| continuous_yaw(&mut last, yaw));
            assert_eq!(output, expected);
             
             
            let previous_delta = output[1] - output[0];
            let delta = output[2] - output[1];
            let flagged =
                output[2].abs() < 360.0 && delta.abs() > 320.0 && previous_delta.abs() < 30.0;
            assert!(!flagged);
            assert_eq!(delta.abs(), 2.0);
        }
    }

    #[test]
    fn wire_yaw_stays_continuous_through_camera_returns_and_many_turns() {
        let mut last = Some(719.0);
        for yaw in [1.0, 179.0, -179.0, 0.0, -179.0, 179.0, 719.0, -721.0] {
            let before = last.unwrap();
            let output = continuous_yaw(&mut last, yaw);
            assert!((output - before).abs() <= 180.0);
            assert!(wrap_degrees(output - yaw).abs() < 1e-4);
        }
    }

    #[test]
    fn yaw_rebase_preserves_all_other_movement_fields() {
        let module = module_without_packet_fallback();
        *module.wire_yaw.lock().unwrap() = Some(179.0);
        let mut packet = movement(-179.0, true);
        let mut expected = packet.clone();
        let Packet::ServerboundMovePlayer(ref mut snapshot) = expected else {
            unreachable!()
        };
        snapshot.y_rot = 181.0;
        assert_eq!(module.handle_packet(&mut packet), PacketAction::Forward);
        assert_eq!(packet, expected);
        assert_eq!(*module.wire_yaw.lock().unwrap(), Some(181.0));
    }

    #[test]
    fn packets_without_look_do_not_change_the_wire_baseline_or_grow_a_look() {
        let module = module_without_packet_fallback();
        *module.wire_yaw.lock().unwrap() = Some(179.0);
        let mut packet = movement(0.0, false);
        let original = packet.clone();
        assert_eq!(module.handle_packet(&mut packet), PacketAction::Forward);
        assert_eq!(packet, original);
        assert_eq!(*module.wire_yaw.lock().unwrap(), Some(179.0));
    }

    #[test]
    fn wire_history_is_local_to_each_silent_aura_instance() {
        let first = module_without_packet_fallback();
        let second = module_without_packet_fallback();
        let mut packet = movement(179.0, true);
        first.handle_packet(&mut packet);
        assert_eq!(*first.wire_yaw.lock().unwrap(), Some(179.0));
        assert_eq!(*second.wire_yaw.lock().unwrap(), None);
        assert!(!first.aura.module.enabled);
        assert_eq!(first.aura.module.id, ModuleId::SilentAura);
    }
}
