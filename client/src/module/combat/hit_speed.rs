 
 
 
 
 
 
 

use std::sync::Mutex;

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;

const SPEED_SETTING: &str = "Speed";

 
 
const MIN_SPEED: f32 = 0.1;
const MAX_SPEED: f32 = 5.0;
const DEFAULT_SPEED: f32 = 2.0;

#[derive(Debug)]
pub struct HitSpeedModule {
    pub module: ModuleData,
     
    carry: Mutex<f32>,
}

impl HitSpeedModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::HitSpeed,
                description:
                    "Speeds up or slows down your attack swing so hits animate faster or slower"
                        .to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: SPEED_SETTING.into(),
                    value: DEFAULT_SPEED,
                    min: MIN_SPEED,
                    max: MAX_SPEED,
                }],
            },
            carry: Mutex::new(0.0),
        }
    }

     
    fn speed(&self) -> f32 {
        self.module
            .get_setting(SPEED_SETTING)
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(MIN_SPEED, MAX_SPEED))
            .unwrap_or(DEFAULT_SPEED)
    }
}

 
 
 
fn take_whole_steps(carry: &mut f32, speed: f32) -> i32 {
    *carry += speed - 1.0;
    let whole = carry.trunc();
    *carry -= whole;
    whole as i32
}

impl Module for HitSpeedModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *self.carry.lock().unwrap_or_else(|p| p.into_inner()) = 0.0;
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.carry.lock().unwrap_or_else(|p| p.into_inner()) = 0.0;
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        let mut carry = self.carry.lock().unwrap_or_else(|p| p.into_inner());
        if !player.is_swinging()? {
             
            *carry = 0.0;
            return Ok(());
        }
        let steps = take_whole_steps(&mut carry, self.speed());
        player.advance_swing(steps)
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
    use super::take_whole_steps;

    #[test]
    fn vanilla_speed_takes_no_extra_steps() {
        let mut carry = 0.0;
        assert_eq!(take_whole_steps(&mut carry, 1.0), 0);
        assert_eq!(carry, 0.0);
    }

    #[test]
    fn whole_speed_takes_one_extra_step_per_tick() {
        let mut carry = 0.0;
        assert_eq!(take_whole_steps(&mut carry, 2.0), 1);
        assert_eq!(take_whole_steps(&mut carry, 2.0), 1);
        assert_eq!(carry, 0.0);
    }

    #[test]
    fn fractional_speed_averages_out_over_ticks() {
         
        let mut carry = 0.0;
        let total: i32 = (0..10).map(|_| take_whole_steps(&mut carry, 1.5)).sum();
        assert_eq!(total, 5);
    }

    #[test]
    fn slower_speed_holds_back_whole_steps() {
         
        let mut carry = 0.0;
        let steps: Vec<i32> = (0..4).map(|_| take_whole_steps(&mut carry, 0.5)).collect();
        assert_eq!(steps, vec![0, -1, 0, -1]);
        assert_eq!(carry, 0.0);
    }

    #[test]
    fn slow_speed_still_moves_the_swing() {
         
        let mut carry = 0.0;
        let net: i32 = (0..4).map(|_| 1 + take_whole_steps(&mut carry, 0.25)).sum();
        assert_eq!(net, 1);
    }
}
