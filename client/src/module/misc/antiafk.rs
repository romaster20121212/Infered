use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
 
const SMOOTH_TICKS: u32 = 8;
 
 
const MIN_HOLD_TICKS: u32 = 2;

 
 
 
 
 
 
 
 
 
 
 
 
 
 

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Look,
    Jump,
    Move,
    Swing,
    Chat,
}

impl Action {
     
    const ALL: [Action; 5] = [
        Action::Look,
        Action::Jump,
        Action::Move,
        Action::Swing,
        Action::Chat,
    ];

     
     
    fn from_choice(index: usize) -> Action {
        Self::ALL.get(index).copied().unwrap_or(Action::Look)
    }

    fn label(self) -> &'static str {
        match self {
            Action::Look => "Look",
            Action::Jump => "Jump",
            Action::Move => "Move",
            Action::Swing => "Swing",
            Action::Chat => "Chat",
        }
    }
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    Forward,
    Back,
    Jump,
}

 
 
fn set_key(key: Key, down: bool) -> anyhow::Result<()> {
    let options = minecraft().options()?;
    match key {
        Key::Forward => options.key_up()?.set_down(down),
        Key::Back => options.key_back()?.set_down(down),
        Key::Jump => options.key_jump()?.set_down(down),
    }
}

 
#[derive(Debug, Clone, Copy, PartialEq)]
enum Effect {
     
     
    Look { from: f32, to: f32 },
     
    Hold(Key),
     
    Jump,
     
    Swing,
     
    Chat,
}

 
#[derive(Debug, Clone, Copy, PartialEq)]
struct Phase {
    ticks: u32,
    effect: Effect,
}

 
 
 
 
fn action_plan(
    action: Action,
    smooth: bool,
    move_back: bool,
    strength: f32,
    forward: bool,
) -> Vec<Phase> {
     
    let ramp = if smooth { SMOOTH_TICKS } else { 1 };
     
    let hold = ramp.max(MIN_HOLD_TICKS);
    let delta = if forward { strength } else { -strength };
     
     
    match action {
        Action::Look => {
            let mut phases = vec![Phase {
                ticks: ramp,
                effect: Effect::Look {
                    from: 0.0,
                    to: delta,
                },
            }];
            if move_back {
                phases.push(Phase {
                    ticks: ramp,
                    effect: Effect::Look {
                        from: delta,
                        to: 0.0,
                    },
                });
            }
            phases
        }
        Action::Move => {
            let mut phases = vec![Phase {
                ticks: hold,
                effect: Effect::Hold(Key::Forward),
            }];
            if move_back {
                phases.push(Phase {
                    ticks: hold,
                    effect: Effect::Hold(Key::Back),
                });
            }
            phases
        }
        Action::Jump => vec![Phase {
            ticks: MIN_HOLD_TICKS,
            effect: Effect::Jump,
        }],
        Action::Swing => vec![Phase {
            ticks: 1,
            effect: Effect::Swing,
        }],
        Action::Chat => vec![Phase {
            ticks: 1,
            effect: Effect::Chat,
        }],
    }
}

 
 
 
fn seed_from_clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        .max(1)
}

 
fn ease(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

 
 
 
 
 
 
fn next_interval(base: Duration, jitter: f32, seed: u64) -> (Duration, u64) {
    let mut seed = seed.max(1);
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
     
    let unit = (seed >> 11) as f64 / (1u64 << 53) as f64;
    let spread = f64::from(jitter.clamp(0.0, 0.5));
    let factor = 1.0 + spread * (unit * 2.0 - 1.0);
    let seconds = base.as_secs_f64() * factor;
    (Duration::from_secs_f64(seconds.max(0.1)), seed)
}

 
fn look_offset(from: f32, to: f32, step: u32, ticks: u32) -> f32 {
    if ticks == 0 {
        return to;
    }
    from + (to - from) * ease(step as f32 / ticks as f32)
}

 
#[derive(Debug)]
struct Active {
    phases: Vec<Phase>,
     
    phase: usize,
     
    step: u32,
     
    start_yaw: f32,
}

#[derive(Debug, Default)]
struct AfkState {
     
    last_action: Option<Instant>,
     
    forward: bool,
     
    active: Option<Active>,
     
     
    releasing: Option<Key>,
     
     
    wait: Duration,
     
    seed: u64,
}

#[derive(Debug)]
pub struct AntiAfkModule {
    pub module: ModuleData,
    state: Mutex<AfkState>,
}

impl AntiAfkModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AntiAfk,
                description: "Acts every few seconds to avoid idle kicks".to_string(),
                category: ModuleCategory::Misc,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Interval".to_string(),
                        value: 6.0,
                        min: 1.0,
                        max: 30.0,
                    },
                    ModuleSetting::Slider {
                        name: "Strength".to_string(),
                        value: 12.0,
                        min: 1.0,
                        max: 45.0,
                    },
                    ModuleSetting::Choice {
                        name: "Action".to_string(),
                        value: 0,
                        options: Action::ALL
                            .iter()
                            .map(|action| action.label().to_string())
                            .collect(),
                    },
                    ModuleSetting::Toggle {
                        name: "Smooth".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Move Back".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Randomize".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Jitter".to_string(),
                        value: 25.0,
                        min: 0.0,
                        max: 50.0,
                    },
                    ModuleSetting::Text {
                        name: "Message".to_string(),
                        value: "afk".to_string(),
                    },
                ],
            },
            state: Mutex::new(AfkState::default()),
        }
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(fallback)
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

     
     
    fn next_wait(&self, base: Duration, state: &mut AfkState) -> Duration {
        if !self.toggle("Randomize", true) {
            return base;
        }
        let jitter = self.slider("Jitter", 25.0) / 100.0;
        let (wait, seed) = next_interval(base, jitter, state.seed);
        state.seed = seed;
        wait
    }

    fn choice(&self, name: &str, fallback: usize) -> usize {
        match self.module.get_setting(name) {
            Some(ModuleSetting::Choice { value, .. }) => *value,
            _ => fallback,
        }
    }

     
    fn message(&self) -> String {
        self.module
            .get_setting("Message")
            .and_then(|setting| setting.get_text_value())
            .unwrap_or("afk")
            .to_string()
    }

     
     
     
     
    fn send_chat(&self) {
        let message = self.message();
        if message.trim().is_empty() {
            return;
        }
        if let Err(error) = minecraft().send_chat(&message) {
            log::warn!("Anti-AFK chat message failed: {error}");
        }
    }

     
     
    fn release_all(&self) {
        let mut state = self.state.lock().unwrap();
        state.active = None;
        state.releasing = None;
        for key in [Key::Forward, Key::Back, Key::Jump] {
            let _ = set_key(key, false);
        }
    }
}

impl Module for AntiAfkModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
        let base = Duration::from_secs_f32(self.slider("Interval", 6.0).max(0.5));
        let mut state = self.state.lock().unwrap();
        *state = AfkState {
            last_action: Some(Instant::now()),
            wait: base,
            seed: seed_from_clock(),
            ..AfkState::default()
        };
         
        let wait = self.next_wait(base, &mut state);
        state.wait = wait;
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.release_all();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let interval = Duration::from_secs_f32(self.slider("Interval", 6.0).max(0.5));
        let smooth = self.toggle("Smooth", true);
        let move_back = self.toggle("Move Back", true);
        let strength = self.slider("Strength", 12.0);
        let action = Action::from_choice(self.choice("Action", 0));

         
         
        let Some(player) = minecraft().player()? else {
            self.release_all();
            return Ok(());
        };

        let now = Instant::now();
        let mut state = self.state.lock().unwrap();

         
         
        if let Some(key) = state.releasing.take() {
            set_key(key, false)?;
        }

        if state.active.is_none() {
            let paused = state
                .last_action
                .is_some_and(|last| now.duration_since(last) < state.wait);
            if paused {
                return Ok(());
            }
            state.last_action = Some(now);
            let wait = self.next_wait(interval, &mut state);
            state.wait = wait;
            let forward = !state.forward;
            state.forward = forward;
            let start_yaw = player.get_yaw()?;
            state.active = Some(Active {
                phases: action_plan(action, smooth, move_back, strength, forward),
                phase: 0,
                step: 0,
                start_yaw,
            });
        }

        let Some(mut active) = state.active.take() else {
            return Ok(());
        };
        let Some(phase) = active.phases.get(active.phase).copied() else {
            return Ok(());
        };

        match phase.effect {
            Effect::Look { from, to } => {
                let yaw = active.start_yaw + look_offset(from, to, active.step + 1, phase.ticks);
                let pitch = player.get_pitch()?;
                player.set_rotation(yaw, pitch)?;
            }
            Effect::Hold(key) => set_key(key, true)?,
            Effect::Jump => set_key(Key::Jump, true)?,
            Effect::Swing => player.swing()?,
            Effect::Chat => self.send_chat(),
        }

        active.step += 1;
        if active.step >= phase.ticks {
             
             
            match phase.effect {
                Effect::Hold(key) => state.releasing = Some(key),
                Effect::Jump => state.releasing = Some(Key::Jump),
                _ => {}
            }
            active.phase += 1;
            active.step = 0;
            if active.phase < active.phases.len() {
                state.active = Some(active);
            }
        } else {
            state.active = Some(active);
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

#[cfg(test)]
mod tests {
    use super::*;

     
    #[test]
    fn the_dropdown_covers_every_action_in_order() {
        let labels: Vec<&str> = Action::ALL.iter().map(|action| action.label()).collect();
        assert_eq!(labels, vec!["Look", "Jump", "Move", "Swing", "Chat"]);
        for (index, action) in Action::ALL.iter().enumerate() {
            assert_eq!(Action::from_choice(index), *action);
        }
         
         
        assert_eq!(Action::from_choice(99), Action::Look);
    }

     
    #[test]
    fn a_smooth_look_goes_out_and_comes_back() {
        let plan = action_plan(Action::Look, true, true, 20.0, true);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].ticks, SMOOTH_TICKS);
        assert_eq!(plan[1].ticks, SMOOTH_TICKS);
        assert_eq!(
            plan[0].effect,
            Effect::Look {
                from: 0.0,
                to: 20.0
            }
        );
        assert_eq!(
            plan[1].effect,
            Effect::Look {
                from: 20.0,
                to: 0.0
            }
        );

         
        let out = look_offset(0.0, 20.0, SMOOTH_TICKS, SMOOTH_TICKS);
        assert!((out - 20.0).abs() < 0.001);
        let home = look_offset(20.0, 0.0, SMOOTH_TICKS, SMOOTH_TICKS);
        assert!(home.abs() < 0.001);
    }

     
    #[test]
    fn an_unsmoothed_look_steps_in_one_tick() {
        let plan = action_plan(Action::Look, false, true, 30.0, true);
        assert_eq!(plan.len(), 2);
        assert!(plan.iter().all(|phase| phase.ticks == 1));
        assert_eq!(
            plan[0].effect,
            Effect::Look {
                from: 0.0,
                to: 30.0
            }
        );
        assert_eq!(
            plan[1].effect,
            Effect::Look {
                from: 30.0,
                to: 0.0
            }
        );
    }

     
    #[test]
    fn the_look_leans_both_ways() {
        let one = action_plan(Action::Look, false, false, 15.0, true);
        let other = action_plan(Action::Look, false, false, 15.0, false);
        assert_eq!(
            one[0].effect,
            Effect::Look {
                from: 0.0,
                to: 15.0
            }
        );
        assert_eq!(
            other[0].effect,
            Effect::Look {
                from: 0.0,
                to: -15.0
            }
        );
    }

     
    #[test]
    fn move_back_off_keeps_the_action_one_way() {
        let look = action_plan(Action::Look, true, false, 12.0, true);
        assert_eq!(look.len(), 1);
        assert_eq!(
            look[0].effect,
            Effect::Look {
                from: 0.0,
                to: 12.0
            }
        );

        let walk = action_plan(Action::Move, true, false, 12.0, true);
        assert_eq!(walk.len(), 1);
        assert_eq!(walk[0].effect, Effect::Hold(Key::Forward));
    }

     
    #[test]
    fn a_walk_holds_the_key_and_comes_back() {
        let plan = action_plan(Action::Move, true, true, 12.0, true);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].effect, Effect::Hold(Key::Forward));
        assert_eq!(plan[1].effect, Effect::Hold(Key::Back));
        assert!(plan.iter().all(|phase| phase.ticks >= MIN_HOLD_TICKS));

         
        let instant = action_plan(Action::Move, false, true, 12.0, true);
        assert!(instant.iter().all(|phase| phase.ticks == MIN_HOLD_TICKS));
    }

     
    #[test]
    fn the_one_shot_actions_ignore_both_toggles() {
        for smooth in [true, false] {
            for move_back in [true, false] {
                let jump = action_plan(Action::Jump, smooth, move_back, 12.0, true);
                assert_eq!(jump.len(), 1);
                assert_eq!(jump[0].effect, Effect::Jump);
                assert!(jump[0].ticks >= MIN_HOLD_TICKS);

                let swing = action_plan(Action::Swing, smooth, move_back, 12.0, true);
                assert_eq!(
                    swing,
                    vec![Phase {
                        ticks: 1,
                        effect: Effect::Swing
                    }]
                );

                let chat = action_plan(Action::Chat, smooth, move_back, 12.0, true);
                assert_eq!(
                    chat,
                    vec![Phase {
                        ticks: 1,
                        effect: Effect::Chat
                    }]
                );
            }
        }
    }

     
    #[test]
    fn every_action_plan_terminates() {
        for action in Action::ALL {
            for smooth in [true, false] {
                for move_back in [true, false] {
                    let plan = action_plan(action, smooth, move_back, 12.0, true);
                    assert!(!plan.is_empty(), "{action:?} has no phases");
                    assert!(
                        plan.iter().all(|phase| phase.ticks >= 1),
                        "{action:?} has an empty phase"
                    );
                }
            }
        }
    }

     
    #[test]
    fn the_look_ramp_is_eased_and_bounded() {
        assert!(ease(0.0).abs() < f32::EPSILON);
        assert!((ease(1.0) - 1.0).abs() < f32::EPSILON);
         
         
        assert!((ease(0.5) - 0.5).abs() < f32::EPSILON);
        let mut previous = f32::MIN;
        for step in 0..=SMOOTH_TICKS {
            let offset = look_offset(0.0, 10.0, step, SMOOTH_TICKS);
            assert!((0.0..=10.0).contains(&offset), "{offset} outside the ramp");
            assert!(offset >= previous, "the ramp went backwards");
            previous = offset;
        }
    }

     
    #[test]
    fn the_module_exposes_the_expected_settings() {
        let module = AntiAfkModule::new();
        let data = module.get_module_data();
        let names: Vec<&str> = data.settings.iter().map(|setting| setting.name()).collect();
        assert_eq!(
            names,
            vec![
                "Interval",
                "Strength",
                "Action",
                "Smooth",
                "Move Back",
                "Randomize",
                "Jitter",
                "Message"
            ]
        );
        match data.get_setting("Action") {
            Some(ModuleSetting::Choice { value, options, .. }) => {
                assert_eq!(*value, 0, "the default action is the first option");
                assert_eq!(
                    options,
                    &vec![
                        "Look".to_string(),
                        "Jump".to_string(),
                        "Move".to_string(),
                        "Swing".to_string(),
                        "Chat".to_string(),
                    ]
                );
            }
            other => panic!("Action must be a dropdown, got {other:?}"),
        }
        assert_eq!(
            data.get_setting("Smooth")
                .and_then(|s| s.get_toggle_value()),
            Some(true)
        );
        assert_eq!(
            data.get_setting("Move Back")
                .and_then(|s| s.get_toggle_value()),
            Some(true)
        );
        assert_eq!(
            data.get_setting("Message").and_then(|s| s.get_text_value()),
            Some("afk")
        );
    }

     
    #[test]
    fn the_first_action_waits_a_full_interval() {
        let module = AntiAfkModule::new();
        module.on_start().unwrap();
        let state = module.state.lock().unwrap();
        assert!(state.last_action.is_some());
        assert!(state.active.is_none());
        let base = Duration::from_secs(6);
        assert!(state.wait <= base.mul_f64(1.25));
        assert!(state.wait >= base.mul_f64(0.75));
    }

     
     
    #[test]
    fn jitter_stays_inside_the_configured_band() {
        let base = Duration::from_secs(6);
        let (exact, _) = next_interval(base, 0.0, 42);
        assert_eq!(exact, base);

        let mut seed = 7;
        let mut seen = Vec::new();
        for _ in 0..32 {
            let (wait, next) = next_interval(base, 0.25, seed);
            seed = next;
            assert!(
                wait >= base.mul_f64(0.75) && wait <= base.mul_f64(1.25),
                "{wait:?} outside the ±25% band"
            );
            seen.push(wait);
        }
        assert!(
            seen.windows(2).any(|pair| pair[0] != pair[1]),
            "every wait was identical — no jitter at all"
        );
    }

     
    #[test]
    fn a_zero_seed_still_produces_a_valid_wait() {
        let (wait, seed) = next_interval(Duration::from_secs(2), 0.5, 0);
        assert!(wait >= Duration::from_millis(100));
        assert!(seed != 0);
    }
}
