 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::input;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use std::sync::Mutex;

 
const MIN_ZOOM_FOV: f32 = 10.0;
 
const MAX_ZOOM_FOV: f32 = 70.0;
 
const DEFAULT_ZOOM_FOV: f32 = 30.0;

 
 
 
 
const ZOOM_KEYS: [(KeyboardKey, &str); 6] = [
    (KeyboardKey::KeyLAlt, "Left Alt"),
    (KeyboardKey::KeyC, "C"),
    (KeyboardKey::KeyV, "V"),
    (KeyboardKey::KeyB, "B"),
    (KeyboardKey::KeyX, "X"),
    (KeyboardKey::KeyZ, "Z"),
];
const DEFAULT_KEY_INDEX: usize = 0;

 
 
fn zoom_key(index: usize) -> KeyboardKey {
    ZOOM_KEYS
        .get(index)
        .map(|(key, _)| *key)
        .unwrap_or(KeyboardKey::KeyLAlt)
}

 
 
fn zoom_fov(configured: f32) -> i32 {
    if !configured.is_finite() {
        return DEFAULT_ZOOM_FOV as i32;
    }
    configured.clamp(MIN_ZOOM_FOV, MAX_ZOOM_FOV).round() as i32
}

 
 
 
 
fn should_zoom(
    enabled: bool,
    active: bool,
    focused: bool,
    menu_open: bool,
    key_down: bool,
) -> bool {
    enabled && active && focused && !menu_open && key_down
}

#[derive(Debug, Default)]
struct ZoomState {
     
    original: Option<i32>,
}

#[derive(Debug)]
pub struct ZoomModule {
    pub module: ModuleData,
    state: Mutex<ZoomState>,
}

impl ZoomModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Zoom,
                description: "Hold a key to narrow the field of view like a spyglass; \
                              releasing it restores your own FOV."
                    .to_string(),
                category: ModuleCategory::Misc,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Zoom FOV".to_string(),
                        value: DEFAULT_ZOOM_FOV,
                        min: MIN_ZOOM_FOV,
                        max: MAX_ZOOM_FOV,
                    },
                    ModuleSetting::Choice {
                        name: "Key".to_string(),
                        value: DEFAULT_KEY_INDEX,
                        options: ZOOM_KEYS
                            .iter()
                            .map(|(_, label)| label.to_string())
                            .collect(),
                    },
                ],
            },
            state: Mutex::new(ZoomState::default()),
        }
    }

    fn configured_fov(&self) -> f32 {
        self.module
            .get_setting("Zoom FOV")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(DEFAULT_ZOOM_FOV)
    }

    fn key(&self) -> KeyboardKey {
        match self.module.get_setting("Key") {
            Some(ModuleSetting::Choice { value, .. }) => zoom_key(*value),
            _ => zoom_key(DEFAULT_KEY_INDEX),
        }
    }

     
     
    fn sync(&self, zoomed: bool) -> anyhow::Result<()> {
        let fov = minecraft().options()?.fov()?;
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if zoomed {
            if state.original.is_none() {
                state.original = Some(fov.get_int()?);
            }
            fov.force_int(zoom_fov(self.configured_fov()))
        } else if let Some(original) = state.original.take() {
            fov.force_int(original)
        } else {
            Ok(())
        }
    }

     
     
    fn update(&self) {
        let zoomed = should_zoom(
            self.module.enabled,
            client().modules.is_active(),
            input::game_window_focused(),
            input::menu_open(),
            input::key_down(self.key() as i32),
        );
        if let Err(error) = self.sync(zoomed) {
            log::warn!("[Zoom] could not update the FOV: {error}");
        }
    }
}

impl Module for ZoomModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap_or_else(|p| p.into_inner()) = ZoomState::default();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.sync(false)
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        self.update();
        Ok(())
    }

    fn on_render(&self) {
        self.update();
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.sync(false)
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
    fn ships_armed_off_with_a_zoom_slider_and_a_key_picker() {
        let module = ZoomModule::new();
        assert_eq!(module.module.id, ModuleId::Zoom);
        assert_eq!(module.module.category, ModuleCategory::Misc);
        assert!(!module.module.enabled);
        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(|setting| setting.name())
            .collect();
        assert_eq!(names, vec!["Zoom FOV", "Key"]);
        assert_eq!(module.configured_fov(), 30.0);
        assert_eq!(module.key(), KeyboardKey::KeyLAlt);
        assert!(module.state.lock().unwrap().original.is_none());
    }

    #[test]
    fn the_zoom_fov_is_clamped_rounded_and_never_nan() {
        assert_eq!(zoom_fov(30.0), 30);
        assert_eq!(zoom_fov(29.6), 30);
        assert_eq!(zoom_fov(5.0), 10);
        assert_eq!(zoom_fov(400.0), 70);
        assert_eq!(zoom_fov(f32::NAN), 30);
    }

    #[test]
    fn the_key_dropdown_covers_every_option_in_order() {
        for (index, (key, label)) in ZOOM_KEYS.iter().enumerate() {
            assert_eq!(zoom_key(index), *key, "{label} maps to its key");
            assert!(!label.is_empty());
        }
        assert_eq!(zoom_key(99), KeyboardKey::KeyLAlt);
    }

    #[test]
    fn the_zoom_requires_every_gate() {
        assert!(should_zoom(true, true, true, false, true));
        assert!(!should_zoom(false, true, true, false, true), "module off");
        assert!(
            !should_zoom(true, false, true, false, true),
            "not in a world"
        );
        assert!(
            !should_zoom(true, true, false, false, true),
            "window unfocused"
        );
        assert!(
            !should_zoom(true, true, true, true, true),
            "a screen is open"
        );
        assert!(!should_zoom(true, true, true, false, false), "key released");
    }
}
