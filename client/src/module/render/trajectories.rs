 
 
 
 
 
 
 
 
 
 

use crate::graphic::esp::{color_setting, toggle_setting, View, V3};
use crate::mapping::client::world::World;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::math::BlockPos;
use crate::mapping::object::MappedObject;
use crate::module::render::trail::bounded;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use egui::{Color32, Painter, Pos2, Stroke};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

 
 
const TRAJECTORY_INTERVAL: Duration = Duration::from_millis(16);
 
const ANGLE_EPSILON: f32 = 0.15;
 
const MAX_STEPS: usize = 200;
 
const IMPACT_HALF_SIZE: f64 = 0.1;
 
 
const SAMPLE_STEP: f64 = 0.25;
 
const BOW_FULL_DRAW_TICKS: f64 = 20.0;

const DEFAULT_THICKNESS: f32 = 2.0;
const DEFAULT_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const DEFAULT_TINT: Color32 = Color32::from_rgb(255, 0, 0);

 
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrajectoryConfig {
    pub(crate) enabled: bool,
    pub(crate) color: Color32,
    pub(crate) thickness: f32,
    pub(crate) show_impact: bool,
}

impl Default for TrajectoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            color: DEFAULT_TINT,
            thickness: DEFAULT_THICKNESS,
            show_impact: true,
        }
    }
}

impl TrajectoryConfig {
    fn from_data(data: &ModuleData) -> Self {
        Self {
            enabled: data.enabled,
            color: color_setting(data, "Color", DEFAULT_TINT),
            thickness: bounded(data, "Thickness", DEFAULT_THICKNESS, 0.5, 6.0),
            show_impact: toggle_setting(data, "Show Impact", true),
        }
    }
}

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProjectileProfile {
     
    pub(crate) speed: f64,
     
    pub(crate) gravity: f64,
     
    pub(crate) drag: f64,
     
    pub(crate) water_drag: f64,
     
    pub(crate) bow: bool,
     
    pub(crate) add_thrower_velocity: bool,
}

 
 
 
pub(crate) fn profile_for(name: &str) -> Option<ProjectileProfile> {
    let bare = name.strip_prefix("minecraft:").unwrap_or(name);
     
     
     
    let (speed, gravity, drag, water_drag, bow) = match bare {
        "bow" => (3.0, 0.05, 0.99, 0.6, true),
        "crossbow" => (3.15, 0.05, 0.99, 0.6, false),
        "trident" => (2.5, 0.05, 0.99, 0.99, false),
        "ender_pearl" | "snowball" | "egg" => (1.5, 0.03, 0.99, 0.8, false),
        "experience_bottle" => (0.7, 0.07, 0.99, 0.8, false),
        _ if bare.ends_with("splash_potion") || bare.ends_with("lingering_potion") => {
            (0.5, 0.05, 0.99, 0.8, false)
        }
        _ => return None,
    };
    Some(ProjectileProfile {
        speed,
        gravity,
        drag,
        water_drag,
        bow,
        add_thrower_velocity: true,
    })
}

 
 
pub(crate) fn look_vector(yaw_deg: f32, pitch_deg: f32) -> V3 {
    let yaw = f64::from(yaw_deg).to_radians();
    let pitch = f64::from(pitch_deg).to_radians();
    V3 {
        x: -yaw.sin() * pitch.cos(),
        y: -pitch.sin(),
        z: yaw.cos() * pitch.cos(),
    }
}

fn plus(a: V3, b: V3) -> V3 {
    V3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cell {
     
    Air,
     
    Water,
     
    Solid,
}

 
 
 
 
 
 
 
 
 
pub(crate) fn simulate(
    start: V3,
    dir: V3,
    profile: &ProjectileProfile,
    thrower_velocity: V3,
    max_steps: usize,
    floor: f64,
    mut classify: impl FnMut(V3) -> Cell,
) -> (Vec<V3>, Option<V3>) {
    let mut vel = V3 {
        x: dir.x * profile.speed,
        y: dir.y * profile.speed,
        z: dir.z * profile.speed,
    };
    if profile.add_thrower_velocity {
        vel = plus(vel, thrower_velocity);
    }
    let mut pos = start;
    let mut points = Vec::with_capacity(max_steps);
    for _ in 0..max_steps {
        let next = plus(pos, vel);
        if next.y < floor {
            break;
        }
        let samples = (distance(pos, next) / SAMPLE_STEP).ceil().max(1.0) as usize;
        let mut in_water = false;
        for sample in 1..=samples {
            let point = lerp(pos, next, sample as f64 / samples as f64);
            match classify(point) {
                Cell::Solid => {
                    points.push(point);
                    return (points, Some(point));
                }
                cell => in_water = cell == Cell::Water,
            }
        }
        pos = next;
        points.push(pos);
        let drag = if in_water {
            profile.water_drag
        } else {
            profile.drag
        };
        vel.x *= drag;
        vel.z *= drag;
        vel.y = vel.y * drag - profile.gravity;
    }
    (points, None)
}

 
fn lerp(a: V3, b: V3, t: f64) -> V3 {
    V3 {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        z: a.z + (b.z - a.z) * t,
    }
}

 
fn distance(a: V3, b: V3) -> f64 {
    V3 {
        x: b.x - a.x,
        y: b.y - a.y,
        z: b.z - a.z,
    }
    .length()
}

 
 
pub(crate) fn bow_power(ticks: f64) -> f64 {
    let x = (ticks / BOW_FULL_DRAW_TICKS).max(0.0);
    ((x * x + 2.0 * x) / 3.0).min(1.0)
}

 
 
const PASS_THROUGH: &[&str] = &[
    "lava",
    "grass",
    "short_grass",
    "tall_grass",
    "fern",
    "large_fern",
    "dead_bush",
    "seagrass",
    "tall_seagrass",
    "dandelion",
    "poppy",
    "blue_orchid",
    "allium",
    "azure_bluet",
    "red_tulip",
    "orange_tulip",
    "white_tulip",
    "pink_tulip",
    "oxeye_daisy",
    "cornflower",
    "lily_of_the_valley",
    "wither_rose",
    "torchflower",
    "sunflower",
    "lilac",
    "rose_bush",
    "peony",
];

 
fn classify_name(name: &str) -> Cell {
    let bare = name.strip_prefix("minecraft:").unwrap_or(name);
    if bare == "water" {
        Cell::Water
    } else if PASS_THROUGH.contains(&bare) {
        Cell::Air
    } else {
        Cell::Solid
    }
}

 
pub(crate) fn block_at(point: V3) -> BlockPos {
    BlockPos::new(
        point.x.floor() as i32,
        point.y.floor() as i32,
        point.z.floor() as i32,
    )
}

 
 
 
pub(crate) fn classify_block(world: &World, pos: &BlockPos) -> Cell {
    match world.is_block_air(pos) {
        Ok(false) => world
            .block_registry_name(pos)
            .map_or(Cell::Solid, |name| classify_name(&name)),
        _ => Cell::Air,
    }
}

 
 
fn simulate_world(
    start: V3,
    dir: V3,
    profile: &ProjectileProfile,
    thrower_velocity: V3,
    world: &World,
    max_steps: usize,
) -> anyhow::Result<(Vec<V3>, Option<V3>)> {
    let floor = f64::from(world.get_min_y()?);
    let mut cache: HashMap<(i32, i32, i32), Cell> = HashMap::new();
    let classify = |point: V3| {
        let pos = block_at(point);
        *cache
            .entry((pos.x(), pos.y(), pos.z()))
            .or_insert_with(|| classify_block(world, &pos))
    };
    Ok(simulate(
        start,
        dir,
        profile,
        thrower_velocity,
        max_steps,
        floor,
        classify,
    ))
}

 
#[derive(Debug, Clone, PartialEq)]
struct Inputs {
    item: String,
    yaw: f32,
    pitch: f32,
    speed: f32,
}

impl Inputs {
     
    fn changed_from(&self, last: &Inputs) -> bool {
        self.item != last.item
            || (self.yaw - last.yaw).abs() > ANGLE_EPSILON
            || (self.pitch - last.pitch).abs() > ANGLE_EPSILON
            || (self.speed - last.speed).abs() > 0.001
    }
}

 
fn simulation_due(
    last_inputs: Option<&Inputs>,
    last_sim: Option<Instant>,
    inputs: &Inputs,
    now: Instant,
    override_interval: Option<Duration>,
) -> bool {
    let interval = override_interval.unwrap_or(TRAJECTORY_INTERVAL);
    let elapsed = last_sim.is_none_or(|last| now.duration_since(last) >= interval);
    let changed = last_inputs.is_none_or(|last| inputs.changed_from(last));
    let item_changed = last_inputs.is_none_or(|last| inputs.item != last.item);
    if override_interval.is_some() {
        item_changed || elapsed
    } else {
        changed || elapsed
    }
}

 
 
struct TrajectoryState {
    points: Vec<V3>,
    impact: Option<V3>,
    last_inputs: Option<Inputs>,
    last_sim: Option<Instant>,
}

fn state() -> &'static Mutex<TrajectoryState> {
    static STATE: OnceLock<Mutex<TrajectoryState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(TrajectoryState {
            points: Vec::new(),
            impact: None,
            last_inputs: None,
            last_sim: None,
        })
    })
}

 
pub(crate) fn clear() {
    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    state.points.clear();
    state.impact = None;
    state.last_inputs = None;
    state.last_sim = None;
}

 
 
 
fn refresh(now: Instant) -> anyhow::Result<()> {
    let Some(player) = minecraft().player()? else {
        clear();
        return Ok(());
    };
    let Some(world) = minecraft().world()? else {
        clear();
        return Ok(());
    };
    if !player.is_alive()? {
        clear();
        return Ok(());
    }
    let inventory = player.get_inventory()?;
    let held = inventory.get_item(inventory.get_selected()?)?;
    if held.is_empty()? {
        clear();
        return Ok(());
    }
    let item = held.get_item()?.registry_name()?;
    let Some(mut profile) = profile_for(&item) else {
        clear();
        return Ok(());
    };
    if profile.bow {
         
         
        let ticks = if player.is_using_item()? {
            f64::from(player.call_method("getTicksUsingItem", &[])?.i()?)
        } else {
            BOW_FULL_DRAW_TICKS
        };
        profile.speed *= bow_power(ticks);
    }
    let inputs = Inputs {
        item,
        yaw: player.get_yaw()?,
        pitch: player.get_pitch()?,
        speed: profile.speed as f32,
    };

    let due = {
        let state = state().lock().unwrap_or_else(|p| p.into_inner());
        let override_interval = crate::module::performance::client_optimizations::override_duration(
            ModuleId::TrajectoryOptimizer,
            "Refresh ms",
            16.0,
            500.0,
        );
        simulation_due(
            state.last_inputs.as_ref(),
            state.last_sim,
            &inputs,
            now,
            override_interval,
        )
    };
    if !due {
        return Ok(());
    }

    let start = V3::from(player.get_eye_position()?);
     
     
    let mut thrower_velocity = V3::from(player.get_delta_movement()?);
    if player.on_ground()? {
        thrower_velocity.y = 0.0;
    }
    let dir = look_vector(inputs.yaw, inputs.pitch);
    let (points, impact) =
        simulate_world(start, dir, &profile, thrower_velocity, &world, MAX_STEPS)?;

    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    state.points = points;
    state.impact = impact;
    state.last_inputs = Some(inputs);
    state.last_sim = Some(now);
    Ok(())
}

 
pub(crate) fn update(cfg: &TrajectoryConfig) {
    if !cfg.enabled {
        clear();
        return;
    }
    if refresh(Instant::now()).is_err() {
        clear();
    }
}

 
 
pub(crate) fn read_config() -> TrajectoryConfig {
    let mut cfg = TrajectoryConfig::default();
    if let Some(arc) = client().modules.get(ModuleId::Trajectories) {
        if let Ok(module) = arc.lock() {
            cfg = TrajectoryConfig::from_data(module.get_module_data());
        }
    }
    cfg
}

 
 
pub(crate) fn draw(painter: &Painter, view: &View, cfg: &TrajectoryConfig) {
    if !cfg.enabled {
        return;
    }
    let state = state().lock().unwrap_or_else(|p| p.into_inner());

    let stroke = Stroke::new(cfg.thickness, cfg.color);
    let projected: Vec<Option<Pos2>> = state
        .points
        .iter()
        .map(|point| view.project(*point))
        .collect();
    for pair in projected.windows(2) {
        if let [Some(a), Some(b)] = pair {
            painter.line_segment([*a, *b], stroke);
        }
    }

    if cfg.show_impact {
        if let Some(hit) = state.impact {
            let min = V3 {
                x: hit.x - IMPACT_HALF_SIZE,
                y: hit.y - IMPACT_HALF_SIZE,
                z: hit.z - IMPACT_HALF_SIZE,
            };
            let max = V3 {
                x: hit.x + IMPACT_HALF_SIZE,
                y: hit.y + IMPACT_HALF_SIZE,
                z: hit.z + IMPACT_HALF_SIZE,
            };
            let corners = crate::graphic::esp::box_corners(min, max);
            crate::graphic::esp::draw_wire_box_width(
                painter,
                view,
                &corners,
                cfg.color,
                cfg.thickness,
            );
        }
    }
}

 
#[derive(Debug)]
pub struct TrajectoriesModule {
    pub module: ModuleData,
}

impl TrajectoriesModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Trajectories,
                description: "Predicts the flight path of a held projectile".to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: DEFAULT_COLOR,
                    },
                    ModuleSetting::Slider {
                        name: "Thickness".to_string(),
                        value: DEFAULT_THICKNESS,
                        min: 0.5,
                        max: 6.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Show Impact".to_string(),
                        value: true,
                    },
                ],
            },
        }
    }
}

impl Module for TrajectoriesModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        clear();
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

#[cfg(test)]
mod tests {
    use super::*;

    const FREE: ProjectileProfile = ProjectileProfile {
        speed: 1.0,
        gravity: 0.0,
        drag: 1.0,
        water_drag: 1.0,
        bow: false,
        add_thrower_velocity: false,
    };

    const GRAVITY: ProjectileProfile = ProjectileProfile {
        speed: 1.0,
        gravity: 0.05,
        drag: 0.99,
        water_drag: 0.8,
        bow: false,
        add_thrower_velocity: false,
    };

     
    fn open(_: V3) -> Cell {
        Cell::Air
    }

    const STILL: V3 = V3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn at(x: f64) -> V3 {
        V3 { x, y: 0.0, z: 0.0 }
    }

    #[test]
    fn trajectory_optimizer_measured_simulated_workload() {
        use std::hint::black_box;
        let origin = Instant::now();
        let run = |optimized: bool| {
            let timer = Instant::now();
            let mut last_inputs: Option<Inputs> = None;
            let mut last_sim = None;
            let mut simulations = 0;
            let mut probes = 0;
            let direction = V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            };
            for frame in 0..600 {
                let inputs = Inputs {
                    item: "bow".into(),
                    yaw: frame as f32,
                    pitch: 0.0,
                    speed: 1.0,
                };
                let now = origin + Duration::from_millis(frame * 8);
                let interval = optimized.then_some(Duration::from_millis(100));
                if simulation_due(last_inputs.as_ref(), last_sim, &inputs, now, interval) {
                    let (points, hit) =
                        simulate(STILL, direction, &GRAVITY, STILL, 200, -64.0, |point| {
                            probes += 1;
                            black_box(point);
                            Cell::Air
                        });
                    assert!(!points.is_empty() && hit.is_none());
                    black_box(points);
                    simulations += 1;
                    last_sim = Some(now);
                    last_inputs = Some(inputs);
                }
            }
            (simulations, probes, timer.elapsed())
        };
        let baseline = run(false);
        let optimized = run(true);
        assert_eq!(baseline.0, 600);
        assert_eq!(optimized.0, 47);
        assert!(optimized.1 < baseline.1);
        println!("600 frames: baseline {} simulations / {} probes in {:?}; optimized {} simulations / {} probes in {:?}", baseline.0, baseline.1, baseline.2, optimized.0, optimized.1, optimized.2);
    }

    #[test]
    fn optimizer_caps_simulation_during_aim_changes_but_not_item_changes() {
        let start = Instant::now();
        let first = Inputs {
            item: "bow".into(),
            yaw: 0.0,
            pitch: 0.0,
            speed: 1.0,
        };
        let mut next = first.clone();
        next.yaw = 30.0;
        let now = start + Duration::from_millis(20);
        assert!(simulation_due(Some(&first), Some(start), &next, now, None));
        assert!(!simulation_due(
            Some(&first),
            Some(start),
            &next,
            now,
            Some(Duration::from_millis(100))
        ));
        assert!(simulation_due(
            Some(&first),
            Some(start),
            &next,
            start + Duration::from_millis(100),
            Some(Duration::from_millis(100))
        ));
        next.item = "trident".into();
        assert!(simulation_due(
            Some(&first),
            Some(start),
            &next,
            now,
            Some(Duration::from_millis(100))
        ));
        assert!(simulation_due(
            None,
            None,
            &next,
            now,
            Some(Duration::from_millis(100))
        ));
    }

    #[test]
    fn trajectory_defaults_are_disabled_and_red() {
        let module = TrajectoriesModule::new();
        assert_eq!(module.module.id, ModuleId::Trajectories);
        assert!(!module.module.enabled);
        assert!(!TrajectoryConfig::default().enabled);
        match module.module.get_setting("Color") {
            Some(ModuleSetting::Color { value, .. }) => {
                assert_eq!(*value, DEFAULT_COLOR);
            }
            other => panic!("expected a colour setting, got {other:?}"),
        }
        assert_eq!(
            module
                .module
                .get_setting("Show Impact")
                .and_then(ModuleSetting::get_toggle_value),
            Some(true)
        );
    }

    #[test]
    fn look_vector_points_along_yaw_and_pitch() {
        let forward = look_vector(0.0, 0.0);
        assert!(near(forward.x, 0.0) && near(forward.y, 0.0) && near(forward.z, 1.0));

        let east = look_vector(90.0, 0.0);
        assert!(near(east.x, -1.0) && near(east.z, 0.0));

        let up = look_vector(0.0, -90.0);
        assert!(near(up.y, 1.0) && near(up.x, 0.0) && near(up.z, 0.0));

        let down = look_vector(0.0, 90.0);
        assert!(near(down.y, -1.0));

        let mixed = look_vector(37.0, -22.0);
        assert!(near(mixed.length(), 1.0));
    }

    #[test]
    fn profile_for_selects_by_item_and_strips_namespace() {
        let bow = profile_for("minecraft:bow").expect("bow is a projectile");
        assert_eq!(bow.speed, 3.0);
        assert_eq!(bow.gravity, 0.05);

        let snowball = profile_for("minecraft:snowball").expect("snowball is a projectile");
        assert_eq!(snowball.speed, 1.5);
        assert_eq!(snowball.gravity, 0.03);

        assert_eq!(profile_for("crossbow").map(|p| p.speed), Some(3.15));
        assert_eq!(
            profile_for("minecraft:crossbow").map(|p| p.speed),
            Some(3.15)
        );
        assert!(profile_for("minecraft:fishing_rod").is_none());
        assert!(profile_for("minecraft:splash_potion").is_some());
        assert!(profile_for("minecraft:potion").is_none());
        assert!(profile_for("minecraft:diamond").is_none());
        assert!(profile_for("minecraft:spawn_egg").is_none());
    }

    #[test]
    fn simulate_without_gravity_or_drag_is_a_straight_line() {
        let dir = V3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let (points, impact) = simulate(STILL, dir, &FREE, STILL, 10, f64::NEG_INFINITY, open);
        assert_eq!(points.len(), 10);
        assert!(impact.is_none());
        for (k, point) in points.iter().enumerate() {
            assert!(near(point.x, 0.0) && near(point.y, 0.0));
            assert!(near(point.z, (k + 1) as f64));
        }
    }

    #[test]
    fn simulate_with_gravity_curves_downward_and_accelerates() {
        let dir = V3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let (points, _) = simulate(STILL, dir, &GRAVITY, STILL, 15, f64::NEG_INFINITY, open);
        assert_eq!(points.len(), 15);
        assert!(points.last().is_some_and(|last| last.y < 0.0));

        let steps: Vec<f64> = points.windows(2).map(|w| w[1].y - w[0].y).collect();
        for pair in steps.windows(2) {
            assert!(pair[1] < pair[0], "fall must speed up each tick");
        }
    }

    #[test]
    fn simulate_adds_thrower_velocity_when_the_profile_asks() {
        let dir = V3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let thrower = V3 {
            x: 0.5,
            y: 0.0,
            z: 0.0,
        };
        let carried = ProjectileProfile {
            add_thrower_velocity: true,
            ..FREE
        };
        let (points, _) = simulate(STILL, dir, &carried, thrower, 1, f64::NEG_INFINITY, open);
        assert!(near(points[0].x, 0.5));
        let (unaided, _) = simulate(STILL, dir, &FREE, thrower, 1, f64::NEG_INFINITY, open);
        assert!(near(unaided[0].x, 0.0));
    }

    #[test]
    fn a_solid_sample_is_the_impact() {
        let dir = V3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let wall = |p: V3| if p.x >= 3.0 { Cell::Solid } else { Cell::Air };
        let (points, impact) = simulate(STILL, dir, &FREE, STILL, 10, f64::NEG_INFINITY, wall);
        assert_eq!(impact.map(|p| p.x), Some(3.0));
        assert!(points.last().is_some_and(|p| near(p.x, 3.0)));
    }

    #[test]
    fn water_slows_the_projectile_by_its_water_drag() {
        let dir = V3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let slow = ProjectileProfile {
            water_drag: 0.5,
            ..FREE
        };
        let in_water = |_: V3| Cell::Water;
        let (points, _) = simulate(STILL, dir, &slow, STILL, 3, f64::NEG_INFINITY, in_water);
         
        assert!(near(points[0].z, 1.0));
        assert!(near(points[1].z - points[0].z, 0.5));
    }

    #[test]
    fn the_arc_stops_before_falling_below_the_floor() {
        let down = V3 {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        };
        let (points, _) = simulate(STILL, down, &FREE, STILL, 50, -2.5, open);
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn bow_power_follows_the_draw_curve() {
        assert!(near(bow_power(0.0), 0.0));
        assert!(near(bow_power(20.0), 1.0));
        assert!(near(bow_power(60.0), 1.0));
        assert!(bow_power(10.0) > 0.0 && bow_power(10.0) < 1.0);
    }

    #[test]
    fn blocks_are_classified_for_projectiles() {
        assert_eq!(classify_name("minecraft:water"), Cell::Water);
        assert_eq!(classify_name("minecraft:tall_grass"), Cell::Air);
        assert_eq!(classify_name("minecraft:lava"), Cell::Air);
        assert_eq!(classify_name("minecraft:stone"), Cell::Solid);
    }

    #[test]
    fn block_at_floors_each_axis() {
        assert_eq!(
            block_at(V3 {
                x: 1.5,
                y: -0.5,
                z: 2.99,
            }),
            BlockPos::new(1, -1, 2)
        );
    }
}
