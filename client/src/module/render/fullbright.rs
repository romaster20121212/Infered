use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;
use std::sync::Mutex;

 
 
const FULLBRIGHT_GAMMA: f64 = 16.0;
 
 
const DEFAULT_GAMMA: f64 = 0.5;

 
 
 
 
 
#[derive(Debug)]
pub struct FullbrightModule {
    pub module: ModuleData,
     
     
    original_gamma: Mutex<Option<f64>>,
}

impl FullbrightModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Fullbright,
                description: "Lights up the world to maximum brightness".to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
            original_gamma: Mutex::new(None),
        }
    }
}

fn scene_allows_fullbright() -> bool {
    let in_world = matches!(minecraft().world(), Ok(Some(_)));
    in_world
}

impl Module for FullbrightModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !scene_allows_fullbright() {
            return Ok(());
        }
        let gamma = minecraft().options()?.gamma()?;
        let current = gamma.get_double().unwrap_or(DEFAULT_GAMMA);
         
         
         
        let original = if current.is_finite() && (0.0..=1.0).contains(&current) {
            current
        } else {
            DEFAULT_GAMMA
        };
        *self.original_gamma.lock().unwrap() = Some(original);
        gamma.force_double(FULLBRIGHT_GAMMA)
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let Some(original) = self.original_gamma.lock().unwrap().take() else {
            return Ok(());
        };
        minecraft().options()?.gamma()?.force_double(original)
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        if !scene_allows_fullbright() {
            let mut applied = self.original_gamma.lock().unwrap();
            if applied.is_some() {
                let original = applied.take().unwrap_or(DEFAULT_GAMMA);
                let _ = minecraft().options()?.gamma()?.force_double(original);
            }
            return Ok(());
        }
        if self.original_gamma.lock().unwrap().is_none() {
            self.on_start()?;
        }
        minecraft()
            .options()?
            .gamma()?
            .force_double(FULLBRIGHT_GAMMA)
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        let mut original = self.original_gamma.lock().unwrap();
        if let Some(value) = original.take() {
            minecraft().options()?.gamma()?.force_double(value)?;
        } else if let Ok(option) = minecraft().options()?.gamma() {
             
            if option.get_double().unwrap_or(DEFAULT_GAMMA) > 1.0 {
                option.force_double(DEFAULT_GAMMA)?;
            }
        }
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
