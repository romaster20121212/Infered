use crate::mapping::client::chunk_batch::boosted_nanos_per_chunk;
use crate::mapping::client::view_distance::singleplayer_player_list;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
 
 
const VANILLA_BASELINE_CHUNKS_PER_TICK: f64 = 3.5;

 
 
 
const MAX_RADIUS_CHUNKS: f32 = 64.0;

 
 
const RADIUS_UNSET: f32 = 0.0;

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadiusPlan {
     
    None,
     
     
    Unavailable,
     
    Apply(i32),
}

 
 
 
 
 
 
 
 
 
 
pub fn plan_radius(requested_chunks: f32, supported: bool) -> RadiusPlan {
    if !requested_chunks.is_finite() || requested_chunks <= RADIUS_UNSET {
        return RadiusPlan::None;
    }
    let requested = requested_chunks.round() as i32;
    if requested <= 0 {
        return RadiusPlan::None;
    }
    if !supported {
        return RadiusPlan::Unavailable;
    }
    RadiusPlan::Apply(requested)
}

 
 
 
 
 
 
 
 
pub fn baseline_chunks_per_tick(live: Option<f64>) -> Option<f64> {
    match live {
        Some(rate) if rate.is_finite() && rate > 0.0 => Some(rate),
        _ => Some(VANILLA_BASELINE_CHUNKS_PER_TICK),
    }
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct ChunkBoostModule {
    pub module: ModuleData,
     
    baseline: Mutex<Option<f64>>,
     
     
    original: Mutex<Option<OriginalRadius>>,
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OriginalRadius {
    client_render_distance: i32,
    server_render_distance: i32,
    server_view_distance: i32,
    server_simulation_distance: i32,
}

impl ChunkBoostModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::ChunkBoost,
                description: "Loads chunks faster, and further in singleplayer".to_string(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Speed".to_string(),
                        value: 10.0,
                        min: 1.0,
                        max: 10.0,
                    },
                    ModuleSetting::Slider {
                        name: "Radius".to_string(),
                        value: RADIUS_UNSET,
                        min: RADIUS_UNSET,
                        max: MAX_RADIUS_CHUNKS,
                    },
                ],
            },
            baseline: Mutex::new(None),
            original: Mutex::new(None),
        }
    }

    fn speed_multiplier(&self) -> f64 {
        self.module
            .get_setting("Speed")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(10.0) as f64
    }

    fn radius_chunks(&self) -> f32 {
        self.module
            .get_setting("Radius")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(RADIUS_UNSET)
    }

     
    fn capture_and_apply(&self) -> anyhow::Result<()> {
        self.capture_baseline();
        self.apply_radius()
    }

    fn capture_baseline(&self) {
        let live = minecraft()
            .chunk_batch_calculator()
            .ok()
            .flatten()
            .and_then(|calculator| calculator.desired_chunks_per_tick().ok());
        if let Some(baseline) = baseline_chunks_per_tick(live) {
            *self.baseline.lock().unwrap() = Some(baseline);
        }
    }

     
    fn apply_radius(&self) -> anyhow::Result<()> {
         
        let player_list = singleplayer_player_list().unwrap_or(None);
        let options = minecraft().options()?;
        let RadiusPlan::Apply(chunks) = plan_radius(self.radius_chunks(), player_list.is_some())
        else {
            return Ok(());
        };
        let Some(player_list) = player_list else {
            return Ok(());
        };

         
         
         
        let mut original = self.original.lock().unwrap();
        if original.is_none() {
            *original = Some(OriginalRadius {
                client_render_distance: options.render_distance()?.get_int().unwrap_or(0),
                server_render_distance: options.server_render_distance().unwrap_or(0),
                server_view_distance: player_list.view_distance().unwrap_or(0),
                server_simulation_distance: player_list.simulation_distance().unwrap_or(0),
            });
        }
        drop(original);

         
         
         
        player_list.set_view_distance(chunks)?;
        player_list.set_simulation_distance(chunks)?;
        options.set_server_render_distance(chunks)?;
        options.render_distance()?.force_int(chunks)?;
        Ok(())
    }

     
    fn restore_radius(&self) -> anyhow::Result<()> {
        let Some(original) = self.original.lock().unwrap().take() else {
            return Ok(());
        };
        let options = match minecraft().options() {
            Ok(options) => options,
            Err(error) => {
                 
                *self.original.lock().unwrap() = Some(original);
                return Err(error);
            }
        };
        if let Ok(render_distance) = options.render_distance() {
            let _ = render_distance.force_int(original.client_render_distance);
        }
        let _ = options.set_server_render_distance(original.server_render_distance);
        if let Ok(Some(player_list)) = singleplayer_player_list() {
            let _ = player_list.set_view_distance(original.server_view_distance);
            let _ = player_list.set_simulation_distance(original.server_simulation_distance);
        }
        Ok(())
    }
}

impl Module for ChunkBoostModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.capture_and_apply()
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.baseline.lock().unwrap().take();
        self.restore_radius()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
         
        let Some(baseline) = *self.baseline.lock().unwrap() else {
            self.capture_baseline();
            return Ok(());
        };
        if let Ok(Some(calculator)) = minecraft().chunk_batch_calculator() {
            let current = calculator.aggregated_nanos_per_chunk().unwrap_or(f64::NAN);
            if let Some(wanted) =
                boosted_nanos_per_chunk(current, baseline, self.speed_multiplier())
            {
                let _ = calculator.force_nanos_per_chunk(wanted);
            }
        }

         
         
        self.apply_radius()
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.on_stop()
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
    fn radius_is_inert_when_unset() {
        assert_eq!(plan_radius(0.0, true), RadiusPlan::None);
        assert_eq!(plan_radius(-1.0, true), RadiusPlan::None);
        assert_eq!(plan_radius(f32::NAN, true), RadiusPlan::None);
    }

    #[test]
    fn radius_needs_singleplayer() {
        assert_eq!(plan_radius(24.0, false), RadiusPlan::Unavailable);
        assert_eq!(plan_radius(24.0, true), RadiusPlan::Apply(24));
    }

    #[test]
    fn radius_is_not_pinned_by_the_stale_singleplayer_clamp() {
         
         
         
        assert_eq!(plan_radius(48.0, true), RadiusPlan::Apply(48));
        assert_eq!(plan_radius(12.0, true), RadiusPlan::Apply(12));
    }

    #[test]
    fn radius_rounds_to_whole_chunks() {
        assert_eq!(plan_radius(24.4, true), RadiusPlan::Apply(24));
        assert_eq!(plan_radius(24.6, true), RadiusPlan::Apply(25));
    }

    #[test]
    fn baseline_prefers_the_live_rate() {
        assert_eq!(baseline_chunks_per_tick(Some(9.0)), Some(9.0));
         
        assert_eq!(
            baseline_chunks_per_tick(Some(0.0)),
            Some(VANILLA_BASELINE_CHUNKS_PER_TICK)
        );
        assert_eq!(
            baseline_chunks_per_tick(Some(f64::NAN)),
            Some(VANILLA_BASELINE_CHUNKS_PER_TICK)
        );
        assert_eq!(
            baseline_chunks_per_tick(None),
            Some(VANILLA_BASELINE_CHUNKS_PER_TICK)
        );
    }
}
