 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::input::GUI_OPEN;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::math::BlockPos;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use std::sync::atomic::Ordering;
use std::sync::Mutex;

 
 
 
const SUPPORT_DEPTH: f64 = 0.5;

 
 
 
const FOOT_CLEARANCE: f64 = 0.001;

 
 
 
 
const NOMINAL_STEP: f64 = 0.1;

 
 
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Feet {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dx: f64,
    pub dz: f64,
    pub half_width: f64,
}

 
 
 
 
pub(crate) fn intended_step(strafe: f32, forward: f32, yaw_degrees: f32, speed: f64) -> (f64, f64) {
    let speed = speed.max(NOMINAL_STEP);
    let (sin, cos) = (yaw_degrees as f64).to_radians().sin_cos();
    let x = strafe as f64 * speed;
    let z = forward as f64 * speed;
    (x * cos - z * sin, z * cos + x * sin)
}

 
pub(crate) fn predicted(feet: &Feet, lookahead: f64) -> (f64, f64) {
    (feet.x + feet.dx * lookahead, feet.z + feet.dz * lookahead)
}

 
 
 
const CORNER_SAMPLE: f64 = 0.01;

 
 
 
 
 
pub(crate) fn support_probes(feet: &Feet, lookahead: f64) -> [[f64; 6]; 4] {
    let (x, z) = predicted(feet, lookahead);
    let w = feet.half_width;
    let depth = feet.y - SUPPORT_DEPTH;
    let top = feet.y - FOOT_CLEARANCE;
    let corner = |cx: f64, cz: f64| {
        [
            cx - CORNER_SAMPLE,
            depth,
            cz - CORNER_SAMPLE,
            cx + CORNER_SAMPLE,
            top,
            cz + CORNER_SAMPLE,
        ]
    };
    [
        corner(x - w, z - w),
        corner(x + w, z - w),
        corner(x - w, z + w),
        corner(x + w, z + w),
    ]
}

 
 
pub(crate) fn overlaps(a: [f64; 6], b: [f64; 6]) -> bool {
    a[0] < b[3] && a[3] > b[0] && a[1] < b[4] && a[4] > b[1] && a[2] < b[5] && a[5] > b[2]
}

 
 
pub(crate) fn fully_supported<F, const N: usize>(probes: [[f64; 6]; N], mut boxes_at: F) -> bool
where
    F: FnMut(i32, i32, i32) -> Vec<[f64; 6]>,
{
    probes.iter().all(|probe| supported(*probe, &mut boxes_at))
}

 
pub(crate) fn supported<F>(probe: [f64; 6], mut boxes_at: F) -> bool
where
    F: FnMut(i32, i32, i32) -> Vec<[f64; 6]>,
{
     
     
    let x_low = probe[0].floor() as i32;
    let x_high = (probe[3] - f64::EPSILON).floor() as i32;
    let y_low = probe[1].floor() as i32;
    let y_high = (probe[4] - f64::EPSILON).floor() as i32;
    let z_low = probe[2].floor() as i32;
    let z_high = (probe[5] - f64::EPSILON).floor() as i32;

    for bx in x_low..=x_high {
        for bz in z_low..=z_high {
            for by in y_low..=y_high {
                for local in boxes_at(bx, by, bz) {
                    let world = [
                        local[0] + bx as f64,
                        local[1] + by as f64,
                        local[2] + bz as f64,
                        local[3] + bx as f64,
                        local[4] + by as f64,
                        local[5] + bz as f64,
                    ];
                    if overlaps(world, probe) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

 
 
#[derive(Debug, Default)]
struct SafeWalkState {
     
    held: bool,
     
    cooldown: u32,
}

 
 
fn sneak_module_enabled() -> bool {
    client()
        .modules
        .get(ModuleId::Sneak)
        .and_then(|handle| {
            handle
                .try_lock()
                .ok()
                .map(|module| module.get_module_data().enabled)
        })
        .unwrap_or(false)
}

 
 
#[derive(Debug)]
pub struct SafeWalkModule {
    pub module: ModuleData,
    state: Mutex<SafeWalkState>,
}

impl SafeWalkModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::SafeWalk,
                description: "Crouches at edges by holding the sneak key".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Lookahead (ticks)".to_string(),
                        value: 1.0,
                        min: 0.5,
                        max: 2.0,
                    },
                    ModuleSetting::Slider {
                        name: "Release Delay (ticks)".to_string(),
                        value: 4.0,
                        min: 0.0,
                        max: 10.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Only On Ground".to_string(),
                        value: true,
                    },
                ],
            },
            state: Mutex::new(SafeWalkState::default()),
        }
    }

    fn toggle(&self, name: &str) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

    fn slider(&self, name: &str, fallback: f64) -> f64 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .map(|value| value as f64)
            .filter(|value| value.is_finite())
            .unwrap_or(fallback)
    }

    fn ticks(&self, name: &str) -> u32 {
        self.slider(name, 0.0).round().max(0.0) as u32
    }

     
    fn edge_ahead(
        &self,
        player: &crate::mapping::entity::player::LocalPlayer,
    ) -> anyhow::Result<bool> {
         
        let (strafe, forward) = player.move_input()?;
        if strafe == 0.0 && forward == 0.0 {
            return Ok(false);
        }
        let mc = minecraft();
        let Some(world) = mc.world()? else {
            return Ok(false);
        };
        let position = player.get_position()?;
        let motion = player.get_delta_movement()?;
        let speed = (motion.x() * motion.x() + motion.z() * motion.z()).sqrt();
        let (dx, dz) = intended_step(strafe, forward, player.get_yaw()?, speed);

        let feet = Feet {
            x: position.x(),
            y: position.y(),
            z: position.z(),
            dx,
            dz,
            half_width: player.bb_width()? as f64 / 2.0,
        };
        let probes = support_probes(&feet, self.slider("Lookahead (ticks)", 1.0));
         
        let mut cache: std::collections::HashMap<(i32, i32, i32), Vec<[f64; 6]>> =
            std::collections::HashMap::new();
        Ok(!fully_supported(probes, |x, y, z| {
            cache
                .entry((x, y, z))
                .or_insert_with(|| {
                    world
                        .block_collision_boxes(&BlockPos::new(x, y, z))
                        .unwrap_or_default()
                })
                .clone()
        }))
    }

     
    fn apply(&self, held: &mut bool, want: bool) -> anyhow::Result<()> {
        let key = minecraft().options()?.key_shift()?;
        if want {
             
            if !*held && !key.is_down()? {
                key.set_down(true)?;
                *held = true;
            }
        } else if *held {
            key.set_down(false)?;
            *held = false;
        }
        Ok(())
    }

     
    fn stand_down(&self, state: &mut SafeWalkState) -> anyhow::Result<()> {
        self.apply(&mut state.held, false)?;
        state.cooldown = 0;
        Ok(())
    }
}

impl Module for SafeWalkModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if let Ok(mut state) = self.state.lock() {
            *state = SafeWalkState::default();
        }
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
        self.stand_down(&mut state)
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
        let mc = minecraft();

        let Some(player) = mc.player()? else {
            return self.stand_down(&mut state);
        };
         
        if mc.current_screen().is_open() || GUI_OPEN.load(Ordering::Relaxed) {
            return self.stand_down(&mut state);
        }
        if self.toggle("Only On Ground") && !player.on_ground()? {
            return self.stand_down(&mut state);
        }
         
         
        if sneak_module_enabled() {
            return self.stand_down(&mut state);
        }
         
        if player.abilities.is_flying()? {
            return self.stand_down(&mut state);
        }

        let edge = self.edge_ahead(&player)?;
        if edge {
            state.cooldown = self.ticks("Release Delay (ticks)");
        }
        let cooling = if edge {
            true
        } else if state.cooldown > 0 {
            state.cooldown -= 1;
            true
        } else {
            false
        };

        self.apply(&mut state.held, cooling)
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

     
    fn floor_at(y: f64) -> impl FnMut(i32, i32, i32) -> Vec<[f64; 6]> {
        move |_x, by, _z| {
            if (by as f64) < y {
                vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
            } else {
                Vec::new()
            }
        }
    }

    fn feet(x: f64, y: f64, z: f64, dx: f64, dz: f64) -> Feet {
        Feet {
            x,
            y,
            z,
            dx,
            dz,
            half_width: 0.3,
        }
    }

    #[test]
    fn the_intended_step_follows_the_camera() {
         
        let (x, z) = intended_step(0.0, 1.0, 0.0, 1.0);
        assert!(x.abs() < 1e-9 && (z - 1.0).abs() < 1e-9);
        let (x, z) = intended_step(1.0, 0.0, 0.0, 1.0);
        assert!((x - 1.0).abs() < 1e-9 && z.abs() < 1e-9);
         
        let (x, z) = intended_step(0.0, 1.0, 90.0, 1.0);
        assert!((x + 1.0).abs() < 1e-9 && z.abs() < 1e-9);
    }

    #[test]
    fn a_player_who_has_not_built_up_speed_still_steps_nominally() {
        let (x, z) = intended_step(0.0, 1.0, 0.0, 0.0);
        assert!(x.abs() < 1e-9);
        assert!((z - NOMINAL_STEP).abs() < 1e-9);
    }

    #[test]
    fn predicted_feet_follow_the_last_motion() {
        let f = feet(0.0, 64.0, 0.0, 0.2, -0.1);
        assert_eq!(predicted(&f, 1.0), (0.2, -0.1));
        assert_eq!(predicted(&f, 2.0), (0.4, -0.2));
    }

    #[test]
    fn the_probes_sit_under_each_corner_and_below_the_feet() {
        let f = feet(1.0, 64.0, 2.0, 0.0, 0.0);
        let probes = support_probes(&f, 1.0);
         
        assert!((probes[0][0] - (0.7 - CORNER_SAMPLE)).abs() < 1e-9);
        assert!((probes[1][3] - (1.3 + CORNER_SAMPLE)).abs() < 1e-9);
        assert!((probes[2][2] - (2.3 - CORNER_SAMPLE)).abs() < 1e-9);
        assert!((probes[3][5] - (2.3 + CORNER_SAMPLE)).abs() < 1e-9);
        for probe in probes {
            assert!(probe[4] < 64.0, "a probe must not reach the feet plane");
            assert!(probe[1] >= 64.0 - SUPPORT_DEPTH - 1e-9);
        }
    }

    #[test]
    fn overlapping_boxes_need_real_volume() {
        let a = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        assert!(overlaps(a, [0.5, 0.5, 0.5, 1.5, 1.5, 1.5]));
         
        assert!(!overlaps(a, [1.0, 0.0, 0.0, 2.0, 1.0, 1.0]));
        assert!(!overlaps(a, [0.0, 1.0, 0.0, 1.0, 2.0, 1.0]));
    }

    #[test]
    fn standing_on_floor_is_supported() {
        let f = feet(0.5, 64.0, 0.5, 0.15, 0.0);
        assert!(fully_supported(support_probes(&f, 1.0), floor_at(64.0)));
    }

    #[test]
    fn walking_towards_an_edge_loses_support() {
         
         
        let mut floor = |bx: i32, _by: i32, _bz: i32| {
            if bx < 2 {
                vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
            } else {
                Vec::new()
            }
        };
        let here = support_probes(&feet(1.6, 64.0, 0.5, 0.0, 0.0), 1.0);
        assert!(fully_supported(here, &mut floor));
        let ahead = support_probes(&feet(1.75, 64.0, 0.5, 0.2, 0.0), 1.0);
        assert!(!fully_supported(ahead, &mut floor));
    }

    #[test]
    fn a_plant_is_not_floor() {
         
         
        let mut world = |bx: i32, _by: i32, _bz: i32| {
            if bx < 1 {
                vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
            } else {
                Vec::new()
            }
        };
        let probes = support_probes(&feet(1.3, 64.0, 0.5, 0.3, 0.0), 1.0);
        assert!(!fully_supported(probes, &mut world));
    }

    #[test]
    fn a_slab_top_counts_as_support() {
         
         
        let mut slab = |_bx: i32, by: i32, _bz: i32| {
            if by == 63 {
                vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
            } else if by == 64 {
                vec![[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]]
            } else {
                Vec::new()
            }
        };
         
        let probes = support_probes(&feet(0.5, 64.5, 0.5, 0.0, 0.0), 1.0);
        assert!(fully_supported(probes, &mut slab));
    }

    #[test]
    fn a_wall_beside_the_feet_is_not_floor() {
         
         
        let mut wall = |bx: i32, by: i32, _bz: i32| {
            if bx == 1 && by == 64 {
                vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
            } else {
                Vec::new()
            }
        };
        let probes = support_probes(&feet(0.75, 64.0, 0.5, 0.0, 0.0), 1.0);
        assert!(!fully_supported(probes, &mut wall));
    }

    #[test]
    fn a_surface_just_below_the_probe_is_not_floor() {
         
         
        let mut ledge = |_bx: i32, by: i32, _bz: i32| {
            if by == 63 {
                vec![[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]]
            } else {
                Vec::new()
            }
        };
        let probes = support_probes(&feet(0.5, 64.0, 0.5, 0.0, 0.0), 1.0);
        assert!(!fully_supported(probes, &mut ledge));
    }

    #[test]
    fn default_settings_are_a_short_anticipation_with_a_release_tail() {
        let module = SafeWalkModule::new();
        assert_eq!(module.ticks("Release Delay (ticks)"), 4);
        assert_eq!(module.slider("Lookahead (ticks)", 0.0), 1.0);
        assert!(module.toggle("Only On Ground"));
    }
}
