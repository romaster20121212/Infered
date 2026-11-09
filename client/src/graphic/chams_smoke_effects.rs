 
use super::chams_smoke::{finite, DrawPuff};
use crate::module::{ModuleData, ModuleSetting};

pub const MAX_ORBS: usize = 24;
pub const MAX_GLOBAL: usize = 32;
 
pub fn glow_pulse(time: f32) -> f32 {
    0.85 + 0.15 * (time * std::f32::consts::TAU / 3.0).sin()
}
#[derive(Clone)]
pub(super) struct Config {
    pub global: bool,
    pub radius: f32,
    pub opacity: f32,
    pub density: f32,
    pub fade_speed: f32,
    pub orbs: bool,
    pub count: usize,
    pub size: f32,
    pub color: [f32; 4],
    pub strength: f32,
    pub flicker: bool,
    pub wind: f32,
    pub direction: f32,
}
impl Config {
    pub fn read(data: &ModuleData) -> Self {
        let toggle = |name, fallback| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(fallback)
        };
        let slider = |name, fallback, min, max| {
            finite(
                data.get_setting(name)
                    .and_then(ModuleSetting::get_slider_value)
                    .unwrap_or(fallback),
                fallback,
                min,
                max,
            )
        };
        Self {
            global: toggle("AuraGlobalSmoke", true),
            radius: slider("AuraSmokeRadius", 1.5, 0.5, 8.0),
            opacity: slider("AuraSmokeOpacity", 0.055, 0.005, 0.2),
            density: slider("AuraSmokeDensity", 12.0, 1.0, 32.0),
            fade_speed: slider("AuraSmokeFadeSpeed", 2.0, 0.5, 8.0),
            orbs: toggle("Orbs", data.id == crate::module::ModuleId::Aura),
            count: slider("OrbCount", 12.0, 1.0, MAX_ORBS as f32).round() as usize,
            size: slider(
                "OrbSize",
                if data.id == crate::module::ModuleId::Aura {
                    0.18
                } else {
                    0.08
                },
                0.05,
                if data.id == crate::module::ModuleId::Aura {
                    0.30
                } else {
                    0.15
                },
            ),
            color: match data.get_setting("OrbColor") {
                Some(ModuleSetting::Color { value, .. }) => {
                    std::array::from_fn(|i| finite(value[i], [1.0, 0.0, 0.0, 1.0][i], 0.0, 1.0))
                }
                _ => [1.0, 0.0, 0.0, 1.0],
            },
            strength: slider(
                "OrbGlowStrength",
                if data.id == crate::module::ModuleId::Aura {
                    1.5
                } else {
                    0.6
                },
                0.0,
                2.0,
            ),
            flicker: toggle("OrbFlicker", true),
            wind: slider(
                if data.id == crate::module::ModuleId::Aura {
                    "Wind Speed"
                } else {
                    "Wind"
                },
                0.25,
                0.0,
                8.0,
            ),
            direction: slider("Wind Direction", 0.0, 0.0, 360.0).to_radians(),
        }
    }
}
#[derive(Clone)]
pub(super) struct Orb {
    pub pos: [f64; 3],
    origin: [f64; 3],
    velocity: [f64; 3],
    age: f32,
    life: f32,
    seed: f32,
}
 
pub(super) struct DrawOrb {
    pub pos: [f64; 3],
    pub size: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}
impl Orb {
    fn advance(&mut self, dt: f32) {
        self.age += finite(dt, 0.0, 0.0, 0.1);
        let t = self.age as f64;
        let seed = self.seed as f64;
        self.pos = std::array::from_fn(|i| self.origin[i] + self.velocity[i] * t);
         
        self.pos[0] += 0.45 * ((t * 0.9 + seed).sin() - seed.sin());
        self.pos[2] += 0.45 * ((t * 0.73 + seed * 1.7).sin() - (seed * 1.7).sin());
        self.pos[1] += 0.16 * ((t * 1.2 + seed * 0.4).sin() - (seed * 0.4).sin());
    }
    pub fn snapshot(&self, cfg: &Config, time: f32) -> DrawOrb {
        DrawOrb {
            pos: self.pos,
            size: cfg.size,
            color: [cfg.color[0], cfg.color[1], cfg.color[2]],
            intensity: self.intensity(cfg, time),
        }
    }
    pub fn intensity(&self, cfg: &Config, time: f32) -> f32 {
        let fade =
            (self.age / 0.35).clamp(0.0, 1.0) * ((self.life - self.age) / 0.6).clamp(0.0, 1.0);
        let flicker = if cfg.flicker {
            0.94 + 0.06 * (time * 11.0 + self.seed).sin()
        } else {
            1.0
        };
        fade * flicker * glow_pulse(time) * cfg.strength * cfg.color[3]
    }
}
#[derive(Default)]
pub(super) struct Effects {
    pub blend: f32,
    pub orbs: Vec<Orb>,
    previous: Option<[f64; 3]>,
    serial: usize,
    target: bool,
}
impl Effects {
    pub fn transition(&mut self, cfg: &Config, active: bool, dt: f32, center: [f64; 3]) {
        if self.previous.replace(center).is_some_and(|old| {
            old.iter()
                .zip(center)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                > 64.0
        }) {
            self.orbs.clear();
        }
        let active = active && cfg.global;
        if active != self.target {
            self.target = active;
            log::info!(
                "[Smoke] combat Aura transition: {}",
                if active { "on" } else { "off" }
            );
        }
        let delta = finite(dt, 0.0, 0.0, 0.1) * cfg.fade_speed;
        self.blend = if active {
            (self.blend + delta).min(1.0)
        } else {
            (self.blend - delta).max(0.0)
        };
        if self.blend < 0.00001 {
            self.blend = 0.0;
        }
        if self.blend > 0.99999 {
            self.blend = 1.0;
        }
    }
    pub fn append_global(
        &self,
        cfg: &Config,
        center: [f64; 3],
        range: f32,
        size: f32,
        light: f32,
        tint: [f32; 4],
        time: f32,
        out: &mut Vec<DrawPuff>,
    ) {
        if self.blend == 0.0 {
            return;
        }
        let radius = (range * cfg.radius).clamp(1.0, 64.0) as f64;
         
        for i in 0..MAX_GLOBAL {
            let occupancy = (cfg.density * self.blend - i as f32).clamp(0.0, 1.0);
            if occupancy == 0.0 {
                break;
            }
            let angle = i as f64 * 2.399963229728653 + time as f64 * 0.025;
            let r = radius * ((i as f64 + 0.5) * 0.61803398875).fract().sqrt();
            let y = ((i as f64 * 0.754877666).fract() * 2.0 - 1.0) * 2.0 + 1.0;
            out.push(DrawPuff {
                pos: [
                    center[0] + r * angle.cos(),
                    center[1] + y + 0.12 * (time + i as f32).sin() as f64,
                    center[2] + r * angle.sin(),
                ],
                radius: size * 1.5,
                seed: i as f32 * 0.731,
                alpha: cfg.opacity * tint[3] * self.blend * occupancy,
                light,
                tint: [tint[0], tint[1], tint[2]],
                blast: false,
            });
        }
    }
    pub fn update_orbs(&mut self, cfg: &Config, smoke: &[DrawPuff], dt: f32, can_emit: bool) {
        if !cfg.orbs {
            self.orbs.clear();
            return;
        }
        for orb in &mut self.orbs {
            orb.advance(dt);
        }
        self.orbs.retain(|o| o.age < o.life);
        self.orbs.truncate(cfg.count);
        if !can_emit || dt == 0.0 || smoke.is_empty() {
            return;
        }
         
        for _ in 0..(cfg.count - self.orbs.len()).min(2) {
            self.serial = self.serial.wrapping_add(1);
            let puff = &smoke[self.serial.wrapping_mul(17) % smoke.len()];
            let seed = self.serial as f32 * 0.731;
            let mut pos = puff.pos;
            pos[0] += puff.radius.min(0.4) as f64 * 0.3 * (seed as f64).sin();
            pos[2] += puff.radius.min(0.4) as f64 * 0.3 * (seed as f64).cos();
            let mut velocity = [
                -(cfg.direction.sin() * cfg.wind * 0.25) as f64,
                0.08,
                (cfg.direction.cos() * cfg.wind * 0.25) as f64,
            ];
            if puff.seed < -1000.0 {
                 
                 
                let angle = seed as f64 * 2.399963229728653;
                velocity[0] += angle.cos() * 0.10;
                velocity[2] += angle.sin() * 0.10;
            }
            self.orbs.push(Orb {
                pos,
                origin: pos,
                velocity,
                age: 0.0,
                life: 2.5 + (seed.sin() + 1.0) * 0.75,
                seed,
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Config {
        Config::read(&crate::module::render::smoke::SmokeModule::new().module)
    }
    #[test]
    fn transition_finishes_in_half_second_and_returns_exactly_to_zero() {
        let cfg = config();
        let mut e = Effects::default();
        for _ in 0..5 {
            e.transition(&cfg, true, 0.1, [0.0; 3]);
        }
        assert_eq!(e.blend, 1.0);
        e.transition(&cfg, false, 0.0, [0.0; 3]);
        assert_eq!(e.blend, 1.0);
        for _ in 0..6 {
            e.transition(&cfg, false, 0.1, [0.0; 3]);
        }
        assert_eq!(e.blend, 0.0);
        let mut out = Vec::new();
        e.append_global(&cfg, [0.0; 3], 4.0, 0.65, 1.0, [1.0; 4], 0.0, &mut out);
        assert!(out.is_empty());
    }
    #[test]
    fn global_cloud_is_faint_bounded_and_scales_with_combat_range() {
        let mut e = Effects::default();
        e.blend = 1.0;
        let cfg = config();
        let mut a = Vec::new();
        let mut b = Vec::new();
        e.append_global(&cfg, [0.0; 3], 4.0, 0.65, 1.0, [1.0; 4], 0.0, &mut a);
        e.append_global(&cfg, [0.0; 3], 8.0, 0.65, 1.0, [1.0; 4], 0.0, &mut b);
        assert_eq!(a.len(), 12);
        assert!(a.len() <= MAX_GLOBAL);
        for (a, b) in a.iter().zip(b.iter()) {
            assert_eq!(a.pos[0] * 2.0, b.pos[0]);
            assert!(a.alpha <= 0.055);
            assert!(a.pos[0].hypot(a.pos[2]) <= 6.0);
        }
    }
    #[test]
    fn orbs_require_smoke_are_capped_fade_expire_and_clear_on_teleport() {
        let mut cfg = config();
        cfg.orbs = true;
        cfg.count = 24;
        let mut e = Effects::default();
        e.update_orbs(&cfg, &[], 0.1, true);
        assert!(e.orbs.is_empty());
        let mut smoke = Vec::new();
        e.blend = 1.0;
        e.append_global(&cfg, [0.0; 3], 4.0, 0.65, 1.0, [1.0; 4], 0.0, &mut smoke);
        e.update_orbs(&cfg, &smoke, 0.1, true);
        assert_eq!(e.orbs.len(), 2);
        assert_eq!(e.orbs[0].intensity(&cfg, 0.0), 0.0);
        for _ in 0..30 {
            e.update_orbs(&cfg, &smoke, 0.1, true);
        }
        assert_eq!(e.orbs.len(), MAX_ORBS);
        for _ in 0..60 {
            e.update_orbs(&cfg, &[], 0.1, false);
        }
        assert!(e.orbs.is_empty());
        e.update_orbs(&cfg, &smoke, 0.1, true);
        e.transition(&cfg, false, 0.1, [0.0; 3]);
        e.transition(&cfg, false, 0.1, [100.0; 3]);
        assert!(e.orbs.is_empty());
    }
    #[test]
    fn orb_wind_is_smooth_seeded_and_frame_rate_independent() {
        let mut cfg = config();
        cfg.orbs = true;
        cfg.wind = 2.0;
        let smoke = [DrawPuff {
            pos: [0.0; 3],
            radius: 1.0,
            seed: 0.0,
            alpha: 1.0,
            light: 1.0,
            tint: [1.0; 3],
            blast: false,
        }];
        let mut e = Effects::default();
        e.update_orbs(&cfg, &smoke, 0.1, true);
        let mut a = e.orbs[0].clone();
        let mut b = a.clone();
        let start = a.pos;
        for _ in 0..10 {
            a.advance(0.1);
        }
        for _ in 0..100 {
            b.advance(0.01);
        }
        for i in 0..3 {
            assert!((a.pos[i] - b.pos[i]).abs() < 0.00001);
        }
        assert!((a.pos[0] - start[0]).hypot(a.pos[2] - start[2]) > 0.1);
        let frozen = a.pos;
        a.advance(0.0);
        assert_eq!(a.pos, frozen);
        let mut other = e.orbs[1].clone();
        other.advance(0.1);
        assert_ne!(other.pos, a.pos);
    }
    #[test]
    fn shared_pulse_is_bounded_and_periodic() {
        for i in 0..300 {
            let t = i as f32 / 100.0;
            assert!((0.7..=1.0).contains(&glow_pulse(t)));
            assert!((glow_pulse(t) - glow_pulse(t + 3.0)).abs() < 0.00001);
        }
    }
}
