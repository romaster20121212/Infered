 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::smoke::{set_explosion_puffs, start_source, stop_source, DrawPuff};
use crate::mapping::client::world::World;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::net::packet::{Packet, PacketAction};
use std::sync::Mutex;

 
 
 
 
const SUPPRESS_TICKS: u32 = 2;
 
 
const MAX_BURSTS: usize = 4;
 
const TICK_BUDGET: usize = 192;
 
const TICK_SECONDS: f32 = 1.0 / 20.0;
 
const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653;
 
const FLASH_FRACTION: f32 = 0.18;
 
const SPARK_GRAVITY: f64 = 1.85;

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spawn {
    pub pos: [f64; 3],
    pub motion: [f64; 3],
}

 
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
     
    pub scale: f32,
    pub spawns: Vec<Spawn>,
}

 
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Frame {
    pub core: Option<Layer>,
    pub sparks: Option<Layer>,
    pub smoke: Option<Layer>,
}

 
fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

 
 
fn shock_ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(4)
}

 
 
 
 
fn shell_point(index: usize, count: usize, radius: f64, phase: f64, flatten: f64) -> [f64; 3] {
    let count = count.max(1) as f64;
    let i = index as f64 + 0.5;
    let y = 1.0 - 2.0 * (i / count);
    let ring = (1.0 - y * y).max(0.0).sqrt();
    let theta = i * GOLDEN_ANGLE + phase;
    let (sin, cos) = theta.sin_cos();
    [
        radius * ring * cos,
        radius * y * (1.0 - flatten.clamp(0.0, 1.0)),
        radius * ring * sin,
    ]
}

 
fn outward(point: [f64; 3], speed: f64) -> [f64; 3] {
    let length = point[0].hypot(point[1]).hypot(point[2]);
    if length < 1e-6 {
        return [0.0, speed * 0.25, 0.0];
    }
    [
        point[0] / length * speed,
        point[1] / length * speed,
        point[2] / length * speed,
    ]
}

 
fn place(center: [f64; 3], point: [f64; 3]) -> [f64; 3] {
    [
        center[0] + point[0],
        center[1] + point[1],
        center[2] + point[2],
    ]
}

 
 
 
 
fn burst_frame(
    center: [f64; 3],
    radius: f32,
    age: f32,
    duration: f32,
    density: f32,
    core: bool,
    sparks: bool,
    smoke: bool,
) -> Frame {
    let duration = if duration > 0.0 {
        duration
    } else {
        TICK_SECONDS
    };
    let t = (age / duration).clamp(0.0, 1.0);
    let wave = ease_out(t) as f64;
    let shock = shock_ease(t) as f64;
     
     
    let fade = (1.0 - t * t).clamp(0.0, 1.0);
    let phase = t as f64 * 6.5;
    let mut frame = Frame::default();

    if core {
        let mut spawns = Vec::new();
         
         
         
        if t < FLASH_FRACTION {
            let flash = ((density * 0.85).round() as usize).max(4);
            let flash_radius = (radius as f64) * 0.22 * wave;
            for index in 0..flash {
                let point = shell_point(index, flash, flash_radius, phase, 0.0);
                spawns.push(Spawn {
                    pos: place(center, point),
                    motion: outward(point, 0.24 * (radius as f64) / duration as f64),
                });
            }
        }
         
         
         
        let speed = (radius as f64) * 0.58 / duration as f64;
        let shell = ((density * (0.35 + 0.65 * fade)).round() as usize).max(1);
        let shell_radius = (radius as f64) * wave;
        for index in 0..shell {
            let point = shell_point(index, shell, shell_radius, phase, 0.28);
            let radial = outward(point, speed);
            let swirl = 0.14 * speed * (1.0 - t as f64);
            let horiz = point[0].hypot(point[2]).max(1e-6);
            let tx = -point[2] / horiz * swirl;
            let tz = point[0] / horiz * swirl;
            spawns.push(Spawn {
                pos: place(center, point),
                motion: [radial[0] + tx, radial[1], radial[2] + tz],
            });
        }
        let crown = ((density * 0.35 * fade).round() as usize).max(1);
        for index in 0..crown {
            let angle = index as f64 * GOLDEN_ANGLE + phase;
            let (sin, cos) = angle.sin_cos();
            let point = [shell_radius * cos, 0.0, shell_radius * sin];
            spawns.push(Spawn {
                pos: place(center, point),
                motion: outward(point, speed * 1.15),
            });
        }
        frame.core = Some(Layer {
            scale: 0.34 * (1.0 - 0.42 * t),
            spawns,
        });
    }

    if sparks {
        let mut spawns = Vec::new();
        let speed = (radius as f64) * 0.68 / duration as f64;
         
         
        let ring = ((density * 0.55 * fade).round() as usize).max(1);
        let ring_radius = (radius as f64) * shock * 1.08;
        for index in 0..ring {
            let angle = index as f64 * GOLDEN_ANGLE + phase * 1.3;
            let (sin, cos) = angle.sin_cos();
            let point = [ring_radius * cos, 0.0, ring_radius * sin];
            spawns.push(Spawn {
                pos: place(center, point),
                motion: outward(point, speed * 1.35),
            });
        }
         
         
        let embers = ((density * 0.45 * fade).round() as usize).max(1);
        let arc_reach = (radius as f64) * (0.85 * wave);
        let time = t as f64;
        let arc_lift = (radius as f64) * 0.42 * (time * (1.0 - 0.85 * time)).max(0.0);
        for index in 0..embers {
            let frac = (index as f64 + 0.5) / embers as f64;
            let angle = index as f64 * GOLDEN_ANGLE * 1.5 + phase * 0.9;
            let (sin, cos) = angle.sin_cos();
            let r = arc_reach * (0.45 + 0.55 * frac);
            let y = arc_lift * (0.5 + 0.5 * (1.0 - frac));
            let point = [r * cos, y, r * sin];
            let clamped = {
                let d = point[0].hypot(point[1]).hypot(point[2]);
                let limit = (radius as f64) * 1.08;
                if d > limit && d > 1e-6 {
                    let s = limit / d;
                    [point[0] * s, point[1] * s, point[2] * s]
                } else {
                    point
                }
            };
            let out = outward([clamped[0], 0.0, clamped[2]], speed * 1.1);
            let vy = speed * (0.55 - SPARK_GRAVITY * time);
            spawns.push(Spawn {
                pos: place(center, clamped),
                motion: [out[0], vy, out[2]],
            });
        }
        frame.sparks = Some(Layer {
            scale: 0.22 * (1.0 - 0.35 * t),
            spawns,
        });
    }

    if smoke {
         
         
         
        let speed = (radius as f64) * 0.20 / duration as f64;
        let lift = 0.12 + 0.28 * wave;
        let count = ((density * 0.72 * (0.5 + 0.5 * fade)).round() as usize).max(1);
        let cloud_radius = (radius as f64) * 0.70 * wave;
        let column_drift = (radius as f64) * 0.16 * (t as f64 * t as f64);
        let mut spawns = Vec::with_capacity(count);
        for index in 0..count {
            let mut point = shell_point(index, count, cloud_radius, phase * 0.65, 0.18);
            point[1] += column_drift;
            let motion = outward(point, speed);
            spawns.push(Spawn {
                pos: place(center, point),
                motion: [motion[0], motion[1] + lift, motion[2]],
            });
        }
        frame.smoke = Some(Layer {
            scale: 0.58 * (0.55 + 0.45 * t),
            spawns,
        });
    }

    frame
}

 
 
fn boom_pitch(center: [f64; 3], radius: f32) -> f32 {
    let hash = ((center[0] * 12.9898 + center[1] * 78.233 + center[2] * 37.719).sin() * 43_758.55)
        .fract()
        .abs() as f32;
    let size_bias = ((8.0 / radius.max(2.0)).clamp(0.75, 1.15) - 1.0) * 0.15;
    (0.72 + hash * 0.22 + size_bias).clamp(0.55, 1.15)
}

 
#[derive(Debug, Clone, Copy)]
struct Burst {
    center: [f64; 3],
    radius: f32,
    duration: f32,
     
    age: f32,
     
     
    needs_sound: bool,
}

#[derive(Debug, Default)]
struct ExplosionState {
    bursts: Vec<Burst>,
     
    suppress_ticks: u32,
     
     
    gpu: bool,
}

#[derive(Debug)]
pub struct ExplosionModule {
    pub module: ModuleData,
    state: Mutex<ExplosionState>,
}

impl ExplosionModule {
    pub fn new() -> Self {
        let slider = |name: &str, value: f32, min: f32, max: f32| ModuleSetting::Slider {
            name: name.into(),
            value,
            min,
            max,
        };
        Self {
            module: ModuleData {
                id: ModuleId::Explosion,
                description:
                    "Replaces server explosions completely with a custom shockwave, sparks, and smoke burst"
                        .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    slider("Radius", 8.0, 2.0, 32.0),
                    slider("Duration", 1.1, 0.4, 2.5),
                    slider("Density", 14.0, 2.0, 40.0),
                    ModuleSetting::Color {
                        name: "Core Color".into(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Color {
                        name: "Spark Color".into(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Color {
                        name: "Smoke Color".into(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Toggle {
                        name: "Core Burst".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Shockwave & Sparks".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Smoke Cloud".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Play Sound".into(),
                        value: true,
                    },
                    slider("Sound Volume", 4.0, 0.0, 6.0),
                    ModuleSetting::Toggle {
                        name: "Hide Vanilla".into(),
                        value: true,
                    },
                ],
            },
            state: Mutex::new(ExplosionState::default()),
        }
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(fallback)
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

    fn color(&self, name: &str, fallback: [f32; 4]) -> [f32; 4] {
        match self.module.get_setting(name) {
            Some(ModuleSetting::Color { value, .. }) => std::array::from_fn(|i| {
                let component = value[i];
                if component.is_finite() {
                    component.clamp(0.0, 1.0)
                } else {
                    fallback[i]
                }
            }),
            _ => fallback,
        }
    }

     
     
    fn lower_gate(&self) {
        if let Err(error) = crate::camera_bridge::set_explosion_suppression(false) {
            log::debug!("Custom explosion gate release: {error}");
        }
    }

     
     
    fn raise_gate(&self) -> bool {
        match crate::camera_bridge::set_explosion_suppression(true) {
            Ok(()) => true,
            Err(error) => {
                log::warn!("Custom explosion cannot hide the vanilla explosion: {error}");
                false
            }
        }
    }

     
     
    fn push_puffs(
        layer: &Layer,
        color: [f32; 4],
        look: (f32, f32, f32),
        budget: &mut usize,
        out: &mut Vec<DrawPuff>,
    ) {
        let (radius_scale, alpha, light) = look;
        let take = layer.spawns.len().min(*budget);
        *budget -= take;
        for (index, spawn) in layer.spawns[..take].iter().enumerate() {
            out.push(DrawPuff {
                pos: spawn.pos,
                radius: layer.scale * radius_scale,
                seed: (index as f32 * 0.618_034).fract(),
                alpha: alpha * color[3],
                light,
                tint: [color[0], color[1], color[2]],
                blast: true,
            });
        }
    }

     
    fn spawn_layer(world: &World, layer: &Layer, color: [f32; 4], budget: &mut usize) {
        if *budget == 0 || layer.spawns.is_empty() {
            return;
        }
        let take = layer.spawns.len().min(*budget);
        *budget -= take;
        let batch: Vec<(f64, f64, f64, f64, f64, f64)> = layer.spawns[..take]
            .iter()
            .map(|spawn| {
                (
                    spawn.pos[0],
                    spawn.pos[1],
                    spawn.pos[2],
                    spawn.motion[0],
                    spawn.motion[1],
                    spawn.motion[2],
                )
            })
            .collect();
        let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        if let Err(error) = world.add_dust_particles(
            channel(color[0]),
            channel(color[1]),
            channel(color[2]),
            layer.scale.max(0.02),
            &batch,
        ) {
            log::debug!("Custom explosion could not spawn particles: {error}");
        }
    }
}

impl Module for ExplosionModule {
    fn on_start(&self) -> anyhow::Result<()> {
        let gpu = match start_source(ModuleId::Explosion) {
            Ok(()) => true,
            Err(error) => {
                log::warn!("Custom explosion falls back to dust particles: {error}");
                false
            }
        };
        let mut state = self.state.lock().unwrap();
        *state = ExplosionState {
            gpu,
            ..ExplosionState::default()
        };
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let gpu = {
            let mut state = self.state.lock().unwrap();
            state.bursts.clear();
            state.suppress_ticks = 0;
            std::mem::take(&mut state.gpu)
        };
        self.lower_gate();
        set_explosion_puffs(Vec::new());
        if gpu {
            stop_source(ModuleId::Explosion)?;
        }
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let density = self.slider("Density", 14.0);
        let core = self.toggle("Core Burst", true);
        let sparks = self.toggle("Shockwave & Sparks", true);
        let smoke = self.toggle("Smoke Cloud", true);
        let play_sound = self.toggle("Play Sound", true);
        let sound_volume = self.slider("Sound Volume", 4.0).clamp(0.0, 6.0);
        let core_color = self.color("Core Color", [1.0, 0.0, 0.0, 1.0]);
        let spark_color = self.color("Spark Color", [1.0, 0.0, 0.0, 1.0]);
        let smoke_color = self.color("Smoke Color", [1.0, 0.0, 0.0, 1.0]);

        let Some(world) = crate::state::minecraft().world()? else {
             
             
             
            {
                let mut state = self.state.lock().unwrap();
                state.bursts.clear();
                state.suppress_ticks = 0;
            }
            self.lower_gate();
            set_explosion_puffs(Vec::new());
            return Ok(());
        };

        let mut state = self.state.lock().unwrap();
        let window_open = state.suppress_ticks > 0;
        if window_open {
             
             
             
             
            self.lower_gate();
        }

        let gpu = state.gpu;
        let mut budget = TICK_BUDGET;
        let mut puffs = Vec::new();
        for burst in &mut state.bursts {
            if burst.needs_sound {
                burst.needs_sound = false;
                if play_sound && sound_volume > 0.0 {
                    let pitch = boom_pitch(burst.center, burst.radius);
                    if let Err(error) =
                        world.play_explosion_sound(burst.center, sound_volume, pitch)
                    {
                        log::debug!("Custom explosion could not play sound: {error}");
                    }
                }
            }
            burst.age += TICK_SECONDS;
            if burst.age >= burst.duration {
                continue;
            }
            let frame = burst_frame(
                burst.center,
                burst.radius,
                burst.age,
                burst.duration,
                density,
                core,
                sparks,
                smoke,
            );
            if gpu {
                 
                 
                let t = (burst.age / burst.duration).clamp(0.0, 1.0);
                let fade = 1.0 - t * t;
                if let Some(layer) = &frame.core {
                    Self::push_puffs(
                        layer,
                        core_color,
                        (2.2, 0.95 * fade, 1.0),
                        &mut budget,
                        &mut puffs,
                    );
                }
                if let Some(layer) = &frame.sparks {
                    Self::push_puffs(
                        layer,
                        spark_color,
                        (1.4, fade, 1.0),
                        &mut budget,
                        &mut puffs,
                    );
                }
                if let Some(layer) = &frame.smoke {
                    Self::push_puffs(
                        layer,
                        smoke_color,
                        (1.9, 0.5 * fade, 0.55),
                        &mut budget,
                        &mut puffs,
                    );
                }
                continue;
            }
            if let Some(layer) = &frame.core {
                Self::spawn_layer(&world, layer, core_color, &mut budget);
            }
            if let Some(layer) = &frame.sparks {
                Self::spawn_layer(&world, layer, spark_color, &mut budget);
            }
            if let Some(layer) = &frame.smoke {
                Self::spawn_layer(&world, layer, smoke_color, &mut budget);
            }
        }
        if gpu {
             
             
            set_explosion_puffs(puffs);
        }
        state.bursts.retain(|burst| burst.age < burst.duration);

        if window_open {
            state.suppress_ticks -= 1;
            if state.suppress_ticks > 0 && !self.raise_gate() {
                state.suppress_ticks = 0;
            }
        }
        Ok(())
    }

    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        let Packet::ClientboundExplode(explosion) = packet else {
            return PacketAction::Forward;
        };
        let hide = self.toggle("Hide Vanilla", true);
         
         
        let radius = explosion
            .radius
            .filter(|radius| radius.is_finite() && *radius >= 1.0)
            .unwrap_or_else(|| self.slider("Radius", 8.0));
        let center = explosion.center;
        let duration = self.slider("Duration", 1.1);

        {
            let mut state = self.state.lock().unwrap();
            let gated = hide && self.raise_gate();
            state.bursts.push(Burst {
                center,
                radius,
                duration,
                age: 0.0,
                 
                needs_sound: gated,
            });
            if state.bursts.len() > MAX_BURSTS {
                state.bursts.remove(0);
            }
            state.suppress_ticks = if gated { SUPPRESS_TICKS } else { 0 };
        }
        if !hide {
            self.lower_gate();
        }

         
         
        PacketAction::Forward
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

    const CENTER: [f64; 3] = [100.0, 64.0, -30.0];

    fn frame_at(age: f32) -> Frame {
        burst_frame(CENTER, 8.0, age, 1.0, 12.0, true, true, true)
    }

     
     
    #[test]
    fn puffs_follow_the_layer_and_respect_the_budget() {
        let frame = frame_at(0.3);
        let layer = frame.smoke.expect("smoke");
        let mut budget = 5;
        let mut out = Vec::new();
        ExplosionModule::push_puffs(
            &layer,
            [0.1, 0.2, 0.3, 0.5],
            (2.0, 0.5, 0.55),
            &mut budget,
            &mut out,
        );
        assert_eq!(out.len(), 5, "budget must cap the puffs");
        assert_eq!(budget, 0);
        for puff in &out {
            assert!((puff.radius - layer.scale * 2.0).abs() < 1e-6);
            assert!((puff.alpha - 0.25).abs() < 1e-6);
            assert_eq!(puff.tint, [0.1, 0.2, 0.3]);
            assert!((0.0..1.0).contains(&puff.seed));
        }
    }

     
    #[test]
    fn every_particle_stays_inside_the_configured_radius() {
        let duration = 1.0;
        for step in 0..=20 {
            let age = step as f32 * duration / 20.0;
            let frame = burst_frame(CENTER, 8.0, age, duration, 12.0, true, true, true);
            for layer in [&frame.core, &frame.sparks, &frame.smoke]
                .into_iter()
                .flatten()
            {
                for spawn in &layer.spawns {
                    let dx = spawn.pos[0] - CENTER[0];
                    let dy = spawn.pos[1] - CENTER[1];
                    let dz = spawn.pos[2] - CENTER[2];
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                    assert!(
                        distance <= 8.0 * 1.1 + 0.001,
                        "spawn {distance} blocks out at age {age}"
                    );
                }
            }
        }
    }

     
    #[test]
    fn the_wave_expands_smoothly_to_the_radius() {
        let radius = |age: f32| {
            let frame = burst_frame(CENTER, 8.0, age, 1.0, 12.0, true, false, false);
            let core = frame.core.expect("core layer");
            core.spawns
                .iter()
                .map(|spawn| {
                    let dx = spawn.pos[0] - CENTER[0];
                    let dy = spawn.pos[1] - CENTER[1];
                    let dz = spawn.pos[2] - CENTER[2];
                    (dx * dx + dy * dy + dz * dz).sqrt()
                })
                .fold(0.0_f64, f64::max)
        };
        let mut previous = 0.0;
        for step in 0..=10 {
            let age = step as f32 / 10.0;
            let reach = radius(age);
            assert!(reach >= previous - 1e-6, "the wave shrank at age {age}");
            previous = reach;
        }
         
        assert!(previous >= 8.0 * 0.98, "the wave only reached {previous}");
    }

     
    #[test]
    fn the_core_flash_happens_first_and_nothing_stays_behind() {
        let first = frame_at(0.01).core.expect("core").spawns.len();
        let late = frame_at(0.9).core.expect("core").spawns.len();
        assert!(first > late, "the flash must be denser than the tail");
        let flash = frame_at(0.02);
        let core = flash.core.expect("core");
         
        let nearest = core
            .spawns
            .iter()
            .map(|spawn| (spawn.pos[0] - CENTER[0]).abs())
            .fold(f64::MAX, f64::min);
        assert!(nearest < 4.0, "no spawn near the centre: {nearest}");

         
        let done = burst_frame(CENTER, 8.0, 1.0, 1.0, 12.0, true, true, true);
        for layer in [&done.core, &done.sparks, &done.smoke]
            .into_iter()
            .flatten()
        {
            for spawn in &layer.spawns {
                let dx = spawn.pos[0] - CENTER[0];
                let dz = spawn.pos[2] - CENTER[2];
                assert!(dx.hypot(dz) <= 8.0 * 1.1 + 0.001);
            }
        }
    }

     
     
    #[test]
    fn shockwave_leads_the_shell_and_sparks_arc_under_gravity() {
        let early = burst_frame(CENTER, 8.0, 0.15, 1.0, 14.0, true, true, false);
        let core_early = early.core.expect("core");
        let sparks_early = early.sparks.expect("sparks");

        let max_horiz = |layer: &Layer| {
            layer
                .spawns
                .iter()
                .map(|spawn| (spawn.pos[0] - CENTER[0]).hypot(spawn.pos[2] - CENTER[2]))
                .fold(0.0_f64, f64::max)
        };
        assert!(
            max_horiz(&sparks_early) > max_horiz(&core_early),
            "shockwave ring must outrun the main core shell"
        );

        let late = burst_frame(CENTER, 8.0, 0.85, 1.0, 14.0, false, true, false)
            .sparks
            .expect("late sparks");
        let early_vy: f64 = sparks_early.spawns.iter().map(|s| s.motion[1]).sum::<f64>()
            / sparks_early.spawns.len() as f64;
        let late_vy: f64 =
            late.spawns.iter().map(|s| s.motion[1]).sum::<f64>() / late.spawns.len() as f64;
        assert!(
            early_vy > late_vy && late_vy < 0.0,
            "debris sparks must climb early ({early_vy}) and fall late ({late_vy})"
        );
    }

     
    #[test]
    fn the_smoke_trails_behind_and_lifts() {
        let frame = burst_frame(CENTER, 8.0, 0.5, 1.0, 12.0, true, true, true);
        let core = frame.core.expect("core");
        let smoke = frame.smoke.expect("smoke");
        let reach = |layer: &Layer| {
            layer
                .spawns
                .iter()
                .map(|spawn| {
                    let dx = spawn.pos[0] - CENTER[0];
                    let dz = spawn.pos[2] - CENTER[2];
                    dx.hypot(dz)
                })
                .fold(0.0_f64, f64::max)
        };
        assert!(reach(&smoke) < reach(&core), "smoke must trail the core");
        let lift: f64 = smoke.spawns.iter().map(|spawn| spawn.motion[1]).sum();
        assert!(lift > 0.0, "the cloud must rise");
        assert!(smoke.scale > core.scale, "soot is fatter than sparks");
    }

     
    #[test]
    fn the_layers_are_independent() {
        let off = burst_frame(CENTER, 8.0, 0.3, 1.0, 12.0, false, false, false);
        assert!(off.core.is_none() && off.sparks.is_none() && off.smoke.is_none());
        let core_only = burst_frame(CENTER, 8.0, 0.3, 1.0, 12.0, true, false, false);
        assert!(
            core_only.core.is_some() && core_only.sparks.is_none() && core_only.smoke.is_none()
        );
        let sparks_only = burst_frame(CENTER, 8.0, 0.3, 1.0, 12.0, false, true, false);
        assert!(
            sparks_only.core.is_none()
                && sparks_only.sparks.is_some()
                && sparks_only.smoke.is_none()
        );
        let smoke_only = burst_frame(CENTER, 8.0, 0.3, 1.0, 12.0, false, false, true);
        assert!(
            smoke_only.core.is_none() && smoke_only.sparks.is_none() && smoke_only.smoke.is_some()
        );
    }

     
     
    #[test]
    fn every_tick_of_the_animation_spawns_a_bounded_amount() {
        for step in 0..10 {
            let age = step as f32 * 0.1;
            let frame = burst_frame(CENTER, 8.0, age, 1.0, 40.0, true, true, true);
            let total = frame
                .core
                .iter()
                .chain(frame.sparks.iter())
                .chain(frame.smoke.iter())
                .map(|layer| layer.spawns.len())
                .sum::<usize>();
            assert!(total > 0, "no particles at age {age}");
             
             
            assert!(total <= 40 * 5, "{total} particles in one tick");
        }
    }

     
    #[test]
    fn a_nonsense_duration_still_produces_finite_particles() {
        for duration in [0.0, -1.0, f32::NAN] {
            let frame = burst_frame(CENTER, 8.0, 0.05, duration, 12.0, true, true, true);
            for layer in [&frame.core, &frame.sparks, &frame.smoke]
                .into_iter()
                .flatten()
            {
                assert!(layer.scale.is_finite() && layer.scale > 0.0);
                for spawn in &layer.spawns {
                    for value in spawn.pos.iter().chain(spawn.motion.iter()) {
                        assert!(value.is_finite(), "non-finite particle at {duration}");
                    }
                }
            }
        }
    }

     
    #[test]
    fn boom_pitch_is_bounded_and_varies_with_position() {
        let p1 = boom_pitch(CENTER, 8.0);
        let p2 = boom_pitch([104.5, 65.0, -28.0], 8.0);
        assert!((0.55..=1.15).contains(&p1));
        assert!((0.55..=1.15).contains(&p2));
        assert!((p1 - p2).abs() > 1e-4);
    }

     
     
    #[test]
    fn the_module_exposes_the_expected_settings() {
        let module = ExplosionModule::new();
        let data = module.get_module_data();
        assert_eq!(data.id, ModuleId::Explosion);
        assert!(!data.enabled, "a cosmetic effect starts off");
        let names: Vec<&str> = data.settings.iter().map(|setting| setting.name()).collect();
        assert_eq!(
            names,
            vec![
                "Radius",
                "Duration",
                "Density",
                "Core Color",
                "Spark Color",
                "Smoke Color",
                "Core Burst",
                "Shockwave & Sparks",
                "Smoke Cloud",
                "Play Sound",
                "Sound Volume",
                "Hide Vanilla",
            ]
        );
        let core = module.color("Core Color", [0.0; 4]);
        assert!(
            core[0] > 0.9 && core[1] < 0.3 && core[2] < 0.3,
            "core is red"
        );
        let spark = module.color("Spark Color", [0.0; 4]);
        assert!(
            spark[0] > 0.9 && spark[1] < 0.3 && spark[2] < 0.3,
            "sparks are red"
        );
        let smoke = module.color("Smoke Color", [1.0; 4]);
        assert!(
            smoke[0] > 0.9 && smoke[1] < 0.3 && smoke[2] < 0.3,
            "smoke is red"
        );
        assert_eq!(
            data.get_setting("Hide Vanilla")
                .and_then(|s| s.get_toggle_value()),
            Some(true)
        );
        assert_eq!(
            data.get_setting("Play Sound")
                .and_then(|s| s.get_toggle_value()),
            Some(true)
        );
    }

     
    #[test]
    fn the_gate_window_is_short_and_finite() {
        assert!(
            SUPPRESS_TICKS >= 1 && SUPPRESS_TICKS <= 4,
            "window out of range"
        );
        let mut state = ExplosionState {
            suppress_ticks: SUPPRESS_TICKS,
            ..ExplosionState::default()
        };
        let mut ticks = 0;
        while state.suppress_ticks > 0 {
            state.suppress_ticks -= 1;
            ticks += 1;
            assert!(ticks <= 10, "the gate window never closed");
        }
        assert_eq!(ticks, SUPPRESS_TICKS);
    }

     
    #[test]
    fn bursts_are_capped_and_expire() {
        let mut state = ExplosionState::default();
        for _ in 0..(MAX_BURSTS + 3) {
            state.bursts.push(Burst {
                center: CENTER,
                radius: 8.0,
                duration: 1.0,
                age: 0.0,
                needs_sound: false,
            });
            if state.bursts.len() > MAX_BURSTS {
                state.bursts.remove(0);
            }
        }
        assert_eq!(state.bursts.len(), MAX_BURSTS);
        for burst in &mut state.bursts {
            burst.age = 1.0;
        }
        state.bursts.retain(|burst| burst.age < burst.duration);
        assert!(state.bursts.is_empty());
    }
}
