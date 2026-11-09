 

use std::time::Duration;

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

#[derive(Debug)]
pub struct ClientOptimization {
    module: ModuleData,
}

impl ClientOptimization {
    fn new(id: ModuleId, description: &str, name: &str, value: f32, min: f32, max: f32) -> Self {
        Self {
            module: ModuleData {
                id,
                description: description.into(),
                category: ModuleCategory::Performance,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: name.into(),
                    value,
                    min,
                    max,
                }],
            },
        }
    }
}

impl Module for ClientOptimization {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn on_tick(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }
    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

pub fn modules() -> [ClientOptimization; 5] {
    [
        ClientOptimization::new(ModuleId::EspSnapshotOptimizer,
            "Refreshes Infered's entity ESP snapshots less often; boxes may trail moving targets.",
            "Refresh ms", 150.0, 66.0, 500.0),
        ClientOptimization::new(ModuleId::BlockScanBudget,
            "Spreads Infered's Block ESP and Xray scans over smaller frame-time slices; scans take longer.",
            "Budget ms", 0.5, 0.25, 2.0),
        ClientOptimization::new(ModuleId::NametagGearOptimizer,
            "Reads nametag equipment less often; gear changes appear later.",
            "Refresh ms", 1000.0, 200.0, 3000.0),
        ClientOptimization::new(ModuleId::RadarScanOptimizer,
            "Scans radar contacts less often; moving contacts may lag.",
            "Refresh ms", 250.0, 100.0, 1000.0),
        ClientOptimization::new(ModuleId::TrajectoryOptimizer,
            "Limits trajectory prediction recalculation; aim and impact previews may lag.",
            "Refresh ms", 100.0, 16.0, 500.0),
    ]
}

 
pub(crate) fn override_duration(id: ModuleId, name: &str, min: f32, max: f32) -> Option<Duration> {
    let client = crate::state::try_client()?;
    if !client.modules.is_active() {
        return None;
    }
    let handle = client.modules.get(id)?;
    let module = handle.try_lock().ok()?;
    let data = module.get_module_data();
    if !data.enabled {
        return None;
    }
    let value = data
        .get_setting(name)
        .and_then(ModuleSetting::get_slider_value)?;
    value
        .is_finite()
        .then(|| Duration::from_secs_f64(f64::from(value.clamp(min, max)) / 1000.0))
}

pub(crate) fn duration(
    id: ModuleId,
    name: &str,
    fallback: Duration,
    min: f32,
    max: f32,
) -> Duration {
    override_duration(id, name, min, max).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn client_optimizers_are_distinct_and_disabled_by_default() {
        let modules = modules();
        let mut ids = HashSet::new();
        for module in modules {
            assert!(ids.insert(module.module.id));
            assert_eq!(module.module.category, ModuleCategory::Performance);
            assert!(!module.module.enabled);
            assert_eq!(module.module.settings.len(), 1);
        }
        assert_eq!(ids.len(), 5);
    }
}
