 
 
 
 
 
 
 
 
 
 
 

use std::sync::Mutex;

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

#[derive(Debug)]
pub struct InvisibleShieldModule {
    pub module: ModuleData,
     
    applied: Mutex<Option<(bool, bool)>>,
}

impl InvisibleShieldModule {
    pub fn new() -> Self {
        let toggle = |name: &str| ModuleSetting::Toggle {
            name: name.into(),
            value: true,
        };
        Self {
            module: ModuleData {
                id: ModuleId::InvisibleShield,
                description:
                    "Hides the shield you are holding in first person, leaving the bare arm. \
                     Purely local: the server still sees the shield."
                        .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![toggle("Main Hand"), toggle("Off Hand")],
            },
            applied: Mutex::new(None),
        }
    }

     
    fn hands(&self) -> (bool, bool) {
        (
            self.toggle("Main Hand", true),
            self.toggle("Off Hand", true),
        )
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

     
     
     
    fn push(&self, main_hand: bool, off_hand: bool, enabled: bool) {
        match crate::camera_bridge::set_shield_hide(main_hand, off_hand, enabled) {
            Ok(()) => {
                let mut applied = self.applied.lock().unwrap_or_else(|p| p.into_inner());
                *applied = enabled.then_some((main_hand, off_hand));
            }
            Err(error) => log::debug!("Invisible Shield: {error}"),
        }
    }
}

impl Module for InvisibleShieldModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::transform_installed(crate::jvmti::SHIELD_TRANSFORM_PARAMS.log_label) {
            log::warn!(
                "Invisible Shield: the view model hook is not installed on this build, so the \
                 shield cannot be hidden"
            );
        }
        let (main_hand, off_hand) = self.hands();
        self.push(main_hand, off_hand, true);
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        self.push(false, false, false);
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.push(false, false, false);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let hands = self.hands();
        {
            let applied = self.applied.lock().unwrap_or_else(|p| p.into_inner());
            if *applied == Some(hands) {
                return Ok(());
            }
        }
        self.push(hands.0, hands.1, true);
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
