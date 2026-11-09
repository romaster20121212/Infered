 
 
 
 
 
 
 
 
 

use std::sync::Mutex;

use crate::camera_bridge::{ItemView, ITEM_VIEW_IDENTITY};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
const MIN_SCALE: f32 = 0.25;
const MAX_SCALE: f32 = 2.0;

 
 
const DEFAULT_SCALE: f32 = 0.75;

 
const MAX_OFFSET: f32 = 1.0;

 
const MAX_ANGLE: f32 = 180.0;

 
 
const MAIN_NAMES: [&str; 7] = [
    "Main Hand",
    "Main X",
    "Main Y",
    "Main Z",
    "Main Pitch",
    "Main Yaw",
    "Main Roll",
];
const OFF_NAMES: [&str; 7] = [
    "Off Hand",
    "Off X",
    "Off Y",
    "Off Z",
    "Off Pitch",
    "Off Yaw",
    "Off Roll",
];

 
const BOUNDS: [(f32, f32, f32); 7] = [
    (MIN_SCALE, MAX_SCALE, DEFAULT_SCALE),
    (-MAX_OFFSET, MAX_OFFSET, 0.0),
    (-MAX_OFFSET, MAX_OFFSET, 0.0),
    (-MAX_OFFSET, MAX_OFFSET, 0.0),
    (-MAX_ANGLE, MAX_ANGLE, 0.0),
    (-MAX_ANGLE, MAX_ANGLE, 0.0),
    (-MAX_ANGLE, MAX_ANGLE, 0.0),
];

#[derive(Debug)]
pub struct ItemScaleModule {
    pub module: ModuleData,
     
    applied: Mutex<Option<(ItemView, ItemView)>>,
}

impl ItemScaleModule {
    pub fn new() -> Self {
        let sliders = |names: &[&str; 7]| {
            names
                .iter()
                .zip(BOUNDS)
                .map(|(name, (min, max, default))| ModuleSetting::Slider {
                    name: (*name).into(),
                    value: default,
                    min,
                    max,
                })
                .collect::<Vec<_>>()
        };
        let mut settings = sliders(&MAIN_NAMES);
        settings.extend(sliders(&OFF_NAMES));
        Self {
            module: ModuleData {
                id: ModuleId::ItemScale,
                description:
                    "Changes the first-person view model for each hand: its size, its offset \
                     and its rotation. The defaults are vanilla except the scale."
                        .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings,
            },
            applied: Mutex::new(None),
        }
    }

     
    fn views(&self) -> (ItemView, ItemView) {
        (self.view(&MAIN_NAMES), self.view(&OFF_NAMES))
    }

    fn view(&self, names: &[&str; 7]) -> ItemView {
        std::array::from_fn(|index| self.value(names[index], BOUNDS[index]))
    }

    fn value(&self, name: &str, (min, max, default): (f32, f32, f32)) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(min, max))
            .unwrap_or(default)
    }

     
     
     
    fn push(&self, main_hand: ItemView, off_hand: ItemView, enabled: bool) {
        match crate::camera_bridge::set_item_view(main_hand, off_hand, enabled) {
            Ok(()) => {
                let mut applied = self.applied.lock().unwrap_or_else(|p| p.into_inner());
                 
                 
                *applied = enabled.then_some((main_hand, off_hand));
            }
            Err(error) => log::debug!("Item View: {error}"),
        }
    }
}

impl Module for ItemScaleModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::transform_installed(crate::jvmti::ITEM_SCALE_TRANSFORM_PARAMS.log_label) {
            log::warn!(
                "Item View: the view model hook is not installed on this build, so the held \
                 item cannot be changed"
            );
        }
        let (main_hand, off_hand) = self.views();
        self.push(main_hand, off_hand, true);
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        self.push(ITEM_VIEW_IDENTITY, ITEM_VIEW_IDENTITY, false);
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
         
        self.push(ITEM_VIEW_IDENTITY, ITEM_VIEW_IDENTITY, false);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let (main_hand, off_hand) = self.views();
        let applied = *self.applied.lock().unwrap_or_else(|p| p.into_inner());
        if applied == Some((main_hand, off_hand)) {
            return Ok(());
        }
        self.push(main_hand, off_hand, true);
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
