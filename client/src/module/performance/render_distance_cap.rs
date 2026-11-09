 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
const DEFAULT_CAP_CHUNKS: f32 = 8.0;
 
const MIN_CAP_CHUNKS: f32 = 2.0;
 
const MAX_CAP_CHUNKS: f32 = 32.0;

 
 
 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Clamp {
    original: i32,
    applied: i32,
}

#[derive(Debug)]
pub struct RenderDistanceCapModule {
    pub module: ModuleData,
    clamp: Mutex<Option<Clamp>>,
}

impl RenderDistanceCapModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::RenderDistanceCap,
                description: "Caps the render distance to draw fewer chunks. Only lowers \
                              the option, and restores it when disabled."
                    .to_string(),
                category: ModuleCategory::Performance,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Distance".to_string(),
                    value: DEFAULT_CAP_CHUNKS,
                    min: MIN_CAP_CHUNKS,
                    max: MAX_CAP_CHUNKS,
                }],
            },
            clamp: Mutex::new(None),
        }
    }

    fn cap_chunks(&self) -> i32 {
        self.module
            .get_setting("Distance")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(DEFAULT_CAP_CHUNKS)
            .round() as i32
    }

     
    fn apply(&self) -> anyhow::Result<()> {
        let cap = self.cap_chunks();
        let render_distance = minecraft().options()?.render_distance()?;
        let current = render_distance.get_int()?;
        if current <= cap {
            return Ok(());
        }
        {
            let mut clamp = self.clamp.lock().unwrap();
             
            let original = clamp.map_or(current, |c| c.original);
            *clamp = Some(Clamp {
                original,
                applied: cap,
            });
        }
        render_distance.force_int(cap)
    }

     
    fn restore(&self) -> anyhow::Result<()> {
        let Some(clamp) = *self.clamp.lock().unwrap() else {
            return Ok(());
        };
        let render_distance = minecraft().options()?.render_distance()?;
        if render_distance.get_int()? == clamp.applied {
            render_distance.force_int(clamp.original)?;
        }
        *self.clamp.lock().unwrap() = None;
        Ok(())
    }

     
    fn sync(&self) -> anyhow::Result<()> {
        if matches!(minecraft().world(), Ok(Some(_))) {
            self.apply()
        } else {
            self.restore()
        }
    }
}

impl Module for RenderDistanceCapModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.sync()
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.restore()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        self.sync()
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.restore()
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
