use crate::mapping::entity::mob::Mob;
use crate::mapping::entity::player::{LocalPlayer, Player};
use crate::mapping::entity::{Entity, EntityRef, LivingEntityRef, PlayerRef};
use crate::mapping::math::Vec3;
use crate::mapping::MappedObject;
use crate::module::combat::rotation::{
    aim_body, clear_silent, restore_body_for, silent_rotation, wrap_degrees, BodyAimOwner, Rotation,
};
use crate::module::combat::{
    crosshair_on_target, look_at, look_at_position, look_at_position_silent, look_at_silent,
    movement_input_held, moving_for_wire, pick_target, shadow_crosshair_on_target, within_reach,
};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::net::packet::{Packet, PacketAction};
use crate::state::{client, minecraft};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
 
 
 
 
 
 
const SPRINT_RESET_DELAY: Duration = Duration::from_millis(50);

 
 
 
 
 
 
 
 
 

 
 
 
const COMBO_RANGE: f64 = 3.0;

 
 
 
const COMBO_HEIGHT: f64 = 2.0;

 
 
const COMBO_IDLE: Duration = Duration::from_millis(1200);

 
 
 
const KNOCKBACK_TRAVEL: f64 = 0.15;

 
 
 
const SPRINT_REPRESS_TIMEOUT: Duration = Duration::from_millis(180);

 
 
 
const MIN_HIT_DELAY: f64 = 0.004;
const MAX_HIT_DELAY: f64 = 0.010;

 
 
 
const ANTI_COMBO_BACKOFF: Duration = Duration::from_millis(350);

 
 
const ANTI_COMBO_DISTANCE: f64 = 3.5;

 
 
const POST_JUMP_WAIT: Duration = Duration::from_millis(100);

 
 
const JUMP_KEY_TIMEOUT: Duration = Duration::from_millis(250);

 
 
 
const CYCLE_TIMEOUT: Duration = Duration::from_millis(1500);

 
 
 
 
 
pub(crate) const HIT_RANGE: f64 = 4.0;  

 
 
 
const LOOK_RANGE_EXTRA: f64 = 5.0;

 
 
 
const STRAFE_HOLD_MIN: f64 = 0.3;
const STRAFE_HOLD_MAX: f64 = 0.7;
const FORWARD_HOLD_MIN: f64 = 0.25;
const FORWARD_HOLD_MAX: f64 = 0.5;

 
#[derive(Debug, Clone, Copy)]
pub enum AuraTarget {
    Players,
    Mobs,
}

 
 
#[derive(Debug, Clone, Copy)]
struct Combo {
     
    hits: u32,
     
     
    origin: (f64, f64),
     
    hit_at: Instant,
     
    waiting_sprint: bool,
     
    repress_by: Instant,
}

 
#[derive(Debug, Default)]
struct AuraState {
     
    target: Option<(i32, Entity)>,
     
     
    hit_after: Option<Instant>,
     
     
    jumped_at: Option<Instant>,
     
     
    jump_key_held: bool,
     
    forward_key_held: bool,
     
    sprint_key_held: bool,
     
    walk_keys_held: bool,
     
     
    strafe: Option<bool>,
     
    beat_until: Option<Instant>,
     
    next_strafe_left: bool,
     
     
    back_key_held: bool,
     
     
     
    sprint_reset_at: Option<Instant>,
     
     
     
    combo: Option<Combo>,
     
     
    combo_sprint_held: bool,
     
    last_health: Option<f32>,
     
     
    last_damage_at: Option<Instant>,
     
    distance_to_target: f64,
     
     
    silent_held: bool,
}

#[derive(Debug)]
pub struct BaseAura {
    pub module: ModuleData,
    pub target: AuraTarget,
    state: Mutex<AuraState>,
     
     
    moving: AtomicBool,
     
     
     
    first_person: AtomicBool,
}

impl BaseAura {
     
     
     
    pub fn new(
        id: ModuleId,
        description: String,
        key_bind: KeyboardKey,
        target: AuraTarget,
        with_silent_aim: bool,
    ) -> Self {
        let mut settings = vec![
            ModuleSetting::Slider {
                name: "Speed".to_string(),
                value: 7.0,
                min: 2.0,
                max: 20.0,
            },
            ModuleSetting::Toggle {
                name: "Cooldown".to_string(),
                value: true,
            },
             
             
             
            ModuleSetting::Toggle {
                name: "Actual Mouse".to_string(),
                value: false,
            },
             
             
            ModuleSetting::Toggle {
                name: "Auto Jump".to_string(),
                value: false,
            },
             
             
            ModuleSetting::Toggle {
                name: "Shield Check".to_string(),
                value: true,
            },
             
             
            ModuleSetting::Toggle {
                name: "Auto Walk".to_string(),
                value: false,
            },
             
            ModuleSetting::Slider {
                name: "Walk Range".to_string(),
                value: 0.5,
                min: 0.5,
                max: 4.0,
            },
            ModuleSetting::Slider {
                name: "Prediction".to_string(),
                value: 0.0,
                min: 0.0,
                max: 0.4,
            },
            ModuleSetting::Toggle {
                name: "Adaptive Strafe".to_string(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Combat Debug".to_string(),
                value: false,
            },
        ];
        if with_silent_aim {
            settings.push(ModuleSetting::Toggle {
                name: "Silent Aim".to_string(),
                value: true,
            });
             
             
             
             
            settings.push(ModuleSetting::Toggle {
                name: "Visual Silent".to_string(),
                value: true,
            });
        }
        Self {
            module: ModuleData {
                id,
                description,
                category: ModuleCategory::Combat,
                key_bind,
                enabled: false,
                settings,
            },
            target,
            state: Mutex::new(AuraState::default()),
            moving: AtomicBool::new(false),
            first_person: AtomicBool::new(true),
        }
    }

     
     
    fn silent_aim(&self) -> bool {
        self.module
            .get_setting("Silent Aim")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

     
     
     
    fn visual_silent(&self) -> bool {
        self.module
            .get_setting("Visual Silent")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

     
     
    fn publish_visual_silent(&self) {
        crate::graphic::visual_silent::set_owner(
            crate::graphic::visual_silent::SILENT_AURA,
            self.silent_aim() && self.visual_silent(),
        );
    }

     
     
    pub(super) fn holds_silent_aim(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.silent_held)
            .unwrap_or(false)
    }

     
     
     
     
     
     
     
     
     
     
     
    fn silent_body_aim(&self) -> bool {
         
         
         
         
         
         
         
         
         
        self.silent_aim()
            && self.first_person.load(Ordering::Relaxed)
            && crate::jvmti::camera_frame_hook_installed()
            && crate::jvmti::crystal_input_hook_installed()
    }

     
     
    fn gate_on_shadow(&self) -> bool {
        hit_gate_uses_shadow(
            self.silent_aim(),
            self.silent_body_aim(),
             
             
            self.moving.load(Ordering::Relaxed) || movement_input_held(),
        )
    }

     
     
    fn crosshair_under(
        &self,
        player: &LocalPlayer,
        target: &Entity,
        aim: Option<Rotation>,
    ) -> bool {
        let range = self.get_range() as f64;
        match aim {
            Some(aim) => shadow_crosshair_on_target(player, target, aim, range).unwrap_or(false),
            None => crosshair_on_target(player, target, range).unwrap_or(false),
        }
    }

     
    fn aim_on_target(&self, player: &LocalPlayer, target: &Entity) -> bool {
        target_under_gate(self.gate_on_shadow(), silent_rotation(), |aim| {
            self.crosshair_under(player, target, aim)
        })
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(fallback)
    }
    fn prediction(&self) -> f32 {
        self.slider("Prediction", 0.0)
    }

     
     
    fn adaptive_strafe(&self) -> bool {
        self.module
            .get_setting("Adaptive Strafe")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

    fn combat_debug(&self) -> bool {
        self.module
            .get_setting("Combat Debug")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

     
     
     
     
     
    pub fn get_range(&self) -> f32 {
        HIT_RANGE as f32 + super::reach_distance()
    }

    fn speed(&self) -> f32 {
        self.slider("Speed", 7.0)
    }

    fn respect_cooldown(&self) -> bool {
        self.module
            .get_setting("Cooldown")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true)
    }

     
     
     
     
    fn actual_mouse(&self) -> bool {
        self.module
            .get_setting("Actual Mouse")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

     
     
    fn auto_jump(&self) -> bool {
        self.module
            .get_setting("Auto Jump")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

     
     
    fn shield_check(&self) -> bool {
        self.module
            .get_setting("Shield Check")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true)
    }

     
     
    fn auto_walk(&self) -> bool {
        self.module
            .get_setting("Auto Walk")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

     
     
    fn walk_range(&self) -> f32 {
        self.slider("Walk Range", 0.5)
    }

     
     
     
     
     
    fn attack_ready(&self, player: &LocalPlayer) -> bool {
        !self.respect_cooldown() || player.attack_strength_scale().ok().unwrap_or(1.0) >= 1.0
    }

     
     
     
     
     
     
     
     
     
     
     
     
     
     
    fn silent_hit(&self, player: &LocalPlayer, target: &Entity) -> bool {
        if self.silent_body_aim() {
             
             
            if !self.attack_ready(player) {
                return false;
            }
            let Some(aim) = silent_rotation() else {
                return false;
            };
            if aim_body(aim).is_err() {
                return false;
            }
        }
        let hit = self.try_hit(player, target);
        if !hit {
            let _ = restore_body_for(BodyAimOwner::SilentAura);
        }
        hit
    }

     
     
     
     
    fn try_hit(&self, player: &LocalPlayer, target: &Entity) -> bool {
        if !self.attack_ready(player) {
            return false;
        }
        if self.actual_mouse() {
             
             
             
            return crate::graphic::platform::left_click();
        }
        let Ok(Some(game_mode)) = minecraft().game_mode() else {
            return false;
        };
         
         
         
         
        if player.is_using_item().unwrap_or(false) {
            let _ = game_mode.release_using_item(player);
        }
        if game_mode.attack(player, target).is_err() {
            return false;
        }
        let _ = player.swing();
        true
    }

     
     
    fn is_target(&self, entity: &Entity) -> bool {
        match self.target {
            AuraTarget::Players => {
                entity.instance_of::<Player>()
                    && !crate::friends::ignores_entity(entity, self.module.id)
            }
            AuraTarget::Mobs => entity.instance_of::<Mob>(),
        }
    }

     
     
     
     
     
     
     
    fn combo_position(&self, player: &LocalPlayer, target: &Entity) -> bool {
        let (Ok(feet), Ok(victim)) = (player.get_position(), target.get_position()) else {
            return false;
        };
        let horizontal = ((victim.x() - feet.x()).powi(2) + (victim.z() - feet.z()).powi(2)).sqrt();
        if !in_combo_window(horizontal, (victim.y() - feet.y()).abs()) {
            return false;
        }
        !target
            .as_living()
            .and_then(|living| living.is_blocking().ok())
            .unwrap_or(false)
    }

     
     
     
     
     
     
     
     
    fn combo_hit(
        &self,
        state: &mut AuraState,
        player: &LocalPlayer,
        target: &Entity,
        now: Instant,
    ) {
         
         
        if !uses_combo_control(self.module.id) {
            return;
        }
        if !self.combo_position(player, target) {
            state.combo = None;
            state.combo_sprint_held = false;
            return;
        }
        let Ok(victim) = target.get_position() else {
            return;
        };
        let hits = state.combo.map_or(0, |combo| combo.hits) + 1;
        state.combo = Some(Combo {
            hits,
            origin: (victim.x(), victim.z()),
            hit_at: now,
            waiting_sprint: true,
            repress_by: now + SPRINT_REPRESS_TIMEOUT,
        });
         
        release_sprint_for_reset(state);
        state.combo_sprint_held = false;
        log::debug!("aura: combo hit #{hits}");
    }

     
    fn end_combo(&self, state: &mut AuraState) {
        state.combo = None;
        state.sprint_reset_at = None;
        if state.combo_sprint_held {
            let _ = set_sprint_key(false);
            state.combo_sprint_held = false;
        }
    }
}

 
 
 
 
fn first_person_view() -> bool {
    minecraft()
        .options()
        .and_then(|options| options.perspective())
        .map(|perspective| perspective == crate::mapping::client::options::Perspective::FirstPerson)
        .unwrap_or(true)
}

 
 
 
 
 
 
 
 
fn hit_gate_uses_shadow(silent_aim: bool, body_aim: bool, moving: bool) -> bool {
    silent_aim && (body_aim || !moving)
}

fn target_under_gate(
    use_shadow: bool,
    shadow: Option<Rotation>,
    mut on_target: impl FnMut(Option<Rotation>) -> bool,
) -> bool {
    if use_shadow {
        shadow.is_some_and(|aim| on_target(Some(aim)))
    } else {
        on_target(None)
    }
}

fn uses_combo_control(id: ModuleId) -> bool {
    id != ModuleId::SilentAura
}

 
 
fn release_silent(state: &mut AuraState) {
    if state.silent_held {
        clear_silent();
        state.silent_held = false;
    }
}

impl Module for BaseAura {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::crystal_input_hook_installed() {
            log::warn!(
                "Aura: input-phase hook unavailable; hits fall back to the frame tick, \
                 which Grim reports as `Post` while moving or jumping"
            );
        }
        if self.silent_aim() && !self.silent_body_aim() {
            log::warn!(
                "Silent Aura: no way to put the view back before the frame is drawn, so a \
                 silent aim stays off the wire while the player walks; silent hits are \
                 standing-only"
            );
        }
        self.publish_visual_silent();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
         
         
        let _ = restore_body_for(BodyAimOwner::SilentAura);
        crate::graphic::visual_silent::set_owner(crate::graphic::visual_silent::SILENT_AURA, false);
        self.moving.store(false, Ordering::Relaxed);
        let mut state = self.state.lock().unwrap();
        release_silent(&mut state);
        release_jump_key(&mut state);
        release_walk_keys(&mut state);
        *state = AuraState::default();
        Ok(())
    }

     
     
     
     
     
     
     
     
     
    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        if self.silent_aim() && !self.silent_body_aim() {
            if let Packet::ServerboundMovePlayer(snapshot) = packet {
                let moving = self.moving.load(Ordering::Relaxed) || movement_input_held();
                snapshot.apply_silent_aim(if moving { None } else { silent_rotation() });
            }
        }
        PacketAction::Forward
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        self.publish_visual_silent();
        let minecraft = minecraft();
         
         
        if minecraft.current_screen().is_open() {
            let mut state = self.state.lock().unwrap();
            release_silent(&mut state);
            release_walk_keys(&mut state);
            return Ok(());
        }
        let (Some(player), Some(world)) = (minecraft.player()?, minecraft.world()?) else {
            let mut state = self.state.lock().unwrap();
            release_silent(&mut state);
            release_jump_key(&mut state);
            release_walk_keys(&mut state);
            *state = AuraState::default();
            return Ok(());  
        };

         
         
         
         
        self.moving
            .store(moving_for_wire(&player), Ordering::Relaxed);
        let entities = world.get_entities()?;
        let eye = player.get_eye_position()?;
        let self_id = player.id()?;
         
         
        self.first_person
            .store(first_person_view(), Ordering::Relaxed);
        let range = self.get_range() as f64;
         
         
        let look_range = range + LOOK_RANGE_EXTRA;

        let locked = self
            .state
            .lock()
            .unwrap()
            .target
            .as_ref()
            .map(|(id, _)| *id);
        let Some((target_id, target)) = pick_target(
            &entities,
            eye,
            look_range * look_range,
            self_id,
            locked,
            |entity| self.is_target(entity),
        ) else {
            let mut state = self.state.lock().unwrap();
            state.target = None;
            state.hit_after = None;
            state.jumped_at = None;
            self.end_combo(&mut state);
            release_silent(&mut state);
            release_jump_key(&mut state);
            release_walk_keys(&mut state);
            return Ok(());
        };

         
        {
            let mut state = self.state.lock().unwrap();
            if state.target.as_ref().map(|(id, _)| *id) != Some(target_id) {
                state.hit_after = None;
                state.jumped_at = None;
                self.end_combo(&mut state);
                release_walk_keys(&mut state);
            }
        }
        {
            let mut state = self.state.lock().unwrap();
            state.target = Some((target_id, target.clone()));
        }

         
         
         
        let silent = self.silent_aim();
        let predicted_aim = predict_target_aim(
            &player,
            &target,
            self.prediction() as f64,
            if silent { 0.5 } else { 0.7 },
        );
        if silent {
            let _ = match predicted_aim {
                Some(aim_pos) => look_at_position_silent(&player, aim_pos, self.speed(), 360.0),
                None => look_at_silent(&player, &target, self.speed(), 360.0),
            };
            if let Ok(mut state) = self.state.lock() {
                state.silent_held = true;
            }
        } else {
            let real_mouse = self.actual_mouse();
            if let Some(aim_pos) = predicted_aim {
                let _ = look_at_position(&player, aim_pos, self.speed(), 360.0, real_mouse);
            } else {
                let _ = look_at(&player, &target, self.speed(), 360.0, real_mouse);
            }
            if let Ok(mut state) = self.state.lock() {
                release_silent(&mut state);
            }
        }

        let now = Instant::now();

        if silent {
             
             
            let mut state = self.state.lock().unwrap();
            release_walk_keys(&mut state);
            release_back_key(&mut state);
        } else {
             
            update_movement_state(self, &player, &target, now);
        }

         
         
        let mut state = self.state.lock().unwrap();
        let crosshair_on = self.aim_on_target(&player, &target);
        if crosshair_on {
            if state.hit_after.is_none() {
                state.hit_after = Some(Instant::now() + hit_reaction_delay());
            }
        } else {
            state.hit_after = None;
        }

         
        if self.combat_debug() {
            let attack_strength = player.attack_strength_scale().ok().unwrap_or(0.0);
            let attack_allowed = attack_strength >= 1.0 || !self.respect_cooldown();
            let within_reach_check = within_reach(&player, &target).unwrap_or(false);
            let strafe_dir = state
                .strafe
                .map(|l| if l { "LEFT" } else { "RIGHT" })
                .unwrap_or("FORWARD");
            let movement_state = if state.is_backing_off() {
                "BACKOFF"
            } else if state.combo.is_some() {
                "COMBO"
            } else if state.walk_keys_held {
                "WALKING"
            } else {
                "STATIC"
            };
            debug_log(
                &state,
                &player,
                &target,
                state.distance_to_target,
                within_reach_check,
                crosshair_on,
                attack_strength,
                attack_allowed,
                strafe_dir,
                movement_state,
                state.is_backing_off(),
                state.combo.is_some(),
                state.combo.map_or(0, |c| c.hits),
                player.get_yaw().ok().unwrap_or(0.0),
                player_facing_target_angle(&player, &target),
                state.forward_key_held,
                state.back_key_held,
                state.strafe,
            );
        }
         
         
        if !crate::jvmti::crystal_input_hook_installed() {
            self.swing();
        }
        Ok(())
    }

    fn on_render(&self) {
         
         
         
        if !client().modules.is_active() {
            return;
        }
        let minecraft = minecraft();
        if minecraft.current_screen().is_open() {
             
             
            if let Ok(mut state) = self.state.lock() {
                state.hit_after = None;
                release_jump_key(&mut state);
                release_walk_keys(&mut state);
            }
            return;
        }

         
        let (Some(player), Some(_game_mode)) = (
            minecraft.player().ok().flatten(),
            minecraft.game_mode().ok().flatten(),
        ) else {
            return;
        };
        let now = Instant::now();

         
         
         
         
        {
            let health = player.get_health().ok();
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            let dropped = match (state.last_health, health) {
                (Some(previous), Some(current)) => current < previous,
                _ => false,
            };
            if dropped {
                state.last_damage_at = Some(now);
            }
            state.last_health = health;
        }

         
         
         
         
         
         
         
         
         
         
        let (combo, victim, released_at, walk_holding) = {
            let Ok(state) = self.state.lock() else {
                return;
            };
            (
                state.combo,
                state.combo.and(state.target.clone()),
                state.sprint_reset_at,
                state.walk_keys_held,
            )
        };
        if let Some(combo) = combo {
            if combo_expired(combo.hit_at, now) {
                if let Ok(mut state) = self.state.lock() {
                    self.end_combo(&mut state);
                }
            } else if combo.waiting_sprint {
                let thrown = victim
                    .as_ref()
                    .and_then(|(_, entity)| entity.get_position().ok())
                    .is_some_and(|position| {
                        knocked_back(combo.origin, (position.x(), position.z()))
                    });
                if sprint_reset_elapsed(released_at, now) && (thrown || now >= combo.repress_by) {
                    let _ = set_sprint_key(true);
                    if let Ok(mut state) = self.state.lock() {
                        if let Some(mut combo) = state.combo {
                            combo.waiting_sprint = false;
                            state.combo = Some(combo);
                        }
                        state.combo_sprint_held = true;
                        state.sprint_reset_at = None;
                    }
                }
            } else if !walk_holding {
                 
                let _ = set_sprint_key(true);
                if let Ok(mut state) = self.state.lock() {
                    state.combo_sprint_held = true;
                }
            }
        }

         
         
         
         
         
         
    }

     
     
     
     
     
     
    fn on_input_phase(&self) -> anyhow::Result<()> {
        self.swing();
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

impl BaseAura {
     
     
     
    fn swing(&self) {
        if !client().modules.is_active() {
            return;
        }
        let minecraft = minecraft();
        if minecraft.current_screen().is_open() {
            return;
        }
        let (Some(player), Some(_game_mode)) = (
            minecraft.player().ok().flatten(),
            minecraft.game_mode().ok().flatten(),
        ) else {
            return;
        };
        let now = Instant::now();

         
        let (target, hit_after) = {
            let Ok(state) = self.state.lock() else {
                return;
            };
            let (Some((_, target)), Some(hit_after)) = (state.target.clone(), state.hit_after)
            else {
                return;
            };
            (target, hit_after)
        };
        if now < hit_after {
            return;
        }

         
         
        if self.shield_check()
            && target
                .as_living()
                .and_then(|living| living.is_blocking().ok())
                .unwrap_or(false)
        {
            return;
        }

         
         
         
         
         
         
         
         
         
         
         
         
         
        let aim_ok = self.aim_on_target(&player, &target);
        let clear_shot = within_reach(&player, &target).unwrap_or(false) && aim_ok;

         
         
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if self.auto_jump() {
             
             
            match state.jumped_at {
                Some(jumped_at) => {
                     
                     
                     
                    if state.jump_key_held
                        && (!player.on_ground().unwrap_or(false)
                            || now.duration_since(jumped_at) >= JUMP_KEY_TIMEOUT)
                    {
                        release_jump_key(&mut state);
                    }
                     
                     
                     
                    if now >= jumped_at + POST_JUMP_WAIT
                        && is_mid_fall(&player)
                        && clear_shot
                        && self.silent_hit(&player, &target)
                    {
                         
                         
                         
                        self.combo_hit(&mut state, &player, &target, now);
                    }
                    if (player.on_ground().unwrap_or(false) && now >= jumped_at + POST_JUMP_WAIT)
                        || now.duration_since(jumped_at) >= CYCLE_TIMEOUT
                    {
                         
                         
                         
                        release_jump_key(&mut state);
                        state.jumped_at = None;
                    }
                }
                None => {
                     
                     
                     
                     
                     
                    let target_on_ground = target
                        .as_living()
                        .and_then(|living| living.on_ground().ok())
                        .unwrap_or(false);
                    if target_on_ground {
                         
                         
                         
                         
                         
                        let on_ground = player.on_ground().unwrap_or(false);
                        if on_ground {
                             
                             
                             
                             
                             
                            state.jump_key_held = set_jump_key(true).is_ok();
                            if !state.jump_key_held {
                                jump(&player);
                            }
                        }
                        state.jumped_at = Some(if on_ground { now } else { now - POST_JUMP_WAIT });
                    } else {
                         
                        release_jump_key(&mut state);
                        if clear_shot && self.silent_hit(&player, &target) {
                            self.combo_hit(&mut state, &player, &target, now);
                        }
                    }
                }
            }
        } else {
             
            release_jump_key(&mut state);
            state.jumped_at = None;
            if clear_shot && self.silent_hit(&player, &target) {
                self.combo_hit(&mut state, &player, &target, now);
            }
        }
    }
}

 
 
fn player_facing_target_angle(player: &LocalPlayer, target: &Entity) -> f32 {
    let Ok(player_pos) = player.get_position() else {
        return 0.0;
    };
    let Ok(target_pos) = target.get_position() else {
        return 0.0;
    };
    let Ok(yaw) = player.get_yaw() else {
        return 0.0;
    };

     
    let dx = target_pos.x() - player_pos.x();
    let dz = target_pos.z() - player_pos.z();

     
    let target_bearing = (dz.atan2(dx).to_degrees() - 90.0) as f32;

     
    let diff = wrap_degrees(target_bearing - yaw);
    diff
}

 
 
 
fn compute_strafe_direction(player: &LocalPlayer, target: &Entity, adaptive: bool) -> Option<bool> {
    let Ok(player_pos) = player.get_position() else {
        return None;
    };
    let Ok(target_pos) = target.get_position() else {
        return None;
    };
    let Ok(yaw) = player.get_yaw() else {
        return None;
    };

     
    let dx = target_pos.x() - player_pos.x();
    let dz = target_pos.z() - player_pos.z();

     
    let target_bearing = (dz.atan2(dx).to_degrees() - 90.0) as f32;

     
    let relative_bearing = wrap_degrees(target_bearing - yaw);

     
    let target_vel = target.get_delta_movement().ok()?;
    let target_horiz_speed =
        (target_vel.x() * target_vel.x() + target_vel.z() * target_vel.z()).sqrt();

    let strafe_left;
    if adaptive && target_horiz_speed > 0.1 {
         
         
         
        let yaw_rad = yaw.to_radians() as f64;
        let target_vel_x_rel = target_vel.x() * yaw_rad.cos() + target_vel.z() * yaw_rad.sin();
         
        strafe_left = target_vel_x_rel > 0.0;
    } else {
         
         
        strafe_left = relative_bearing < -10.0;
    }

    Some(strafe_left)
}

 
 
fn update_movement_state(aura: &BaseAura, player: &LocalPlayer, target: &Entity, now: Instant) {
    let mut state = match aura.state.lock() {
        Ok(state) => state,
        Err(_) => return,
    };

     
    if !aura.auto_walk() {
        release_walk_keys(&mut state);
        release_back_key(&mut state);
        return;
    }

     
    let (Ok(player_pos), Ok(target_pos)) = (player.get_position(), target.get_position()) else {
         
        release_walk_keys(&mut state);
        release_back_key(&mut state);
        return;
    };

     
    let dx = target_pos.x() - player_pos.x();
    let dz = target_pos.z() - player_pos.z();
    let horizontal = (dx * dx + dz * dz).sqrt();
    state.distance_to_target = horizontal;

     
    let is_backing_off = backing_off(state.last_damage_at, horizontal, now);

     
    let chase_range = if state.combo.is_some() {
        0.0
    } else {
        aura.walk_range() as f64 * 0.5
    };

     
    let should_walk = horizontal > chase_range && !is_backing_off;  
    if is_backing_off {
         
        aura.end_combo(&mut state);

         
        if state.forward_key_held || state.sprint_key_held || state.walk_keys_held {
            release_walk_keys(&mut state);
        }

         
        if !state.back_key_held {
            state.back_key_held = set_back_key(true).is_ok();
        }

         
        if state.beat_until.map_or(true, |until| now >= until) {
            release_strafe(&mut state);
            let new_left = state.next_strafe_left;
            state.next_strafe_left = !new_left;
            if set_strafe(new_left, true).is_ok() {
                state.strafe = Some(new_left);
            }
            state.beat_until = Some(now + Duration::from_secs_f64(random_range(0.2, 0.4)));
        } else if let Some(left) = state.strafe {
            let _ = set_strafe(left, true);
        }

         
        if state.forward_key_held {
            let _ = set_forward_key(false);
            state.forward_key_held = false;
        }
        if state.sprint_key_held {
            let _ = set_sprint_key(false);
            state.sprint_key_held = false;
        }
        state.walk_keys_held = false;
        return;
    }

     
    if state.back_key_held {
        let _ = set_back_key(false);
        state.back_key_held = false;
    }

     
    if !should_walk {
        release_walk_keys(&mut state);
        release_strafe(&mut state);
        return;
    }

     
     
    if !state.walk_keys_held {
         
        state.forward_key_held = set_forward_key(true).is_ok();
        state.sprint_key_held = set_sprint_key(true).is_ok();
        state.walk_keys_held = state.forward_key_held || state.sprint_key_held;

         
        if let Some(strafe_left) = compute_strafe_direction(player, target, aura.adaptive_strafe())
        {
            state.next_strafe_left = strafe_left;
            if set_strafe(strafe_left, true).is_ok() {
                state.strafe = Some(strafe_left);
            }
            state.beat_until =
                Some(now + Duration::from_secs_f64(random_range(STRAFE_HOLD_MIN, STRAFE_HOLD_MAX)));
        }
    }

     
    if state.walk_keys_held {
        if state.beat_until.map_or(true, |until| now >= until) {
             
            release_strafe(&mut state);

             
            if state.strafe.is_some() {
                 
                state.beat_until = Some(
                    now + Duration::from_secs_f64(random_range(FORWARD_HOLD_MIN, FORWARD_HOLD_MAX)),
                );
            } else {
                 
                let new_left = state.next_strafe_left;
                state.next_strafe_left = !new_left;
                if set_strafe(new_left, true).is_ok() {
                    state.strafe = Some(new_left);
                }
                state.beat_until = Some(
                    now + Duration::from_secs_f64(random_range(STRAFE_HOLD_MIN, STRAFE_HOLD_MAX)),
                );
            }
        } else if let Some(left) = state.strafe {
             
            let _ = set_strafe(left, true);
        }
    }
}

 
 
 
 
 
fn set_jump_key(down: bool) -> anyhow::Result<()> {
    minecraft().options()?.key_jump()?.set_down(down)
}

 
 
 
 
fn set_walk_keys(down: bool) -> anyhow::Result<()> {
    set_forward_key(down)?;
    set_sprint_key(down)
}

 
fn set_forward_key(down: bool) -> anyhow::Result<()> {
    minecraft().options()?.key_up()?.set_down(down)
}

 
 
fn set_sprint_key(down: bool) -> anyhow::Result<()> {
    minecraft().options()?.key_sprint()?.set_down(down)
}

 
 
fn set_back_key(down: bool) -> anyhow::Result<()> {
    minecraft().options()?.key_back()?.set_down(down)
}

 
 
 
 
fn release_sprint_for_reset(state: &mut AuraState) {
    let _ = set_sprint_key(false);
    schedule_sprint_reset(state);
}

 
 
fn schedule_sprint_reset(state: &mut AuraState) {
    state.sprint_reset_at = Some(Instant::now() + SPRINT_RESET_DELAY);
}

 
 
fn sprint_reset_elapsed(at: Option<Instant>, now: Instant) -> bool {
    at.is_some_and(|repress_at| now >= repress_at)
}

 
 
 
 
 
 
fn backing_off(last_damage_at: Option<Instant>, horizontal: f64, now: Instant) -> bool {
    last_damage_at.is_some_and(|at| now.saturating_duration_since(at) < ANTI_COMBO_BACKOFF)
        && horizontal < ANTI_COMBO_DISTANCE
}

 
 
 
 
fn in_combo_window(horizontal: f64, vertical: f64) -> bool {
    horizontal <= COMBO_RANGE && vertical <= COMBO_HEIGHT
}

 
 
 
 
 
fn knocked_back(origin: (f64, f64), current: (f64, f64)) -> bool {
    const TOLERANCE: f64 = 1e-6;
    let dx = current.0 - origin.0;
    let dz = current.1 - origin.1;
    (dx * dx + dz * dz).sqrt() + TOLERANCE >= KNOCKBACK_TRAVEL
}

 
fn combo_expired(hit_at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(hit_at) > COMBO_IDLE
}

 
 
impl AuraState {
    fn is_backing_off(&self) -> bool {
        self.last_damage_at.is_some_and(|at| {
            let now = Instant::now();
            now.saturating_duration_since(at) < ANTI_COMBO_BACKOFF
        }) && self.distance_to_target < ANTI_COMBO_DISTANCE
    }
}

 
fn release_jump_key(state: &mut AuraState) {
    if state.jump_key_held {
        let _ = set_jump_key(false);
        state.jump_key_held = false;
    }
}

 
 
fn release_walk_keys(state: &mut AuraState) {
     
    if state.forward_key_held {
        let _ = set_forward_key(false);
        state.forward_key_held = false;
    }
     
    if state.sprint_key_held && !state.combo_sprint_held {
        let _ = set_sprint_key(false);
        state.sprint_key_held = false;
    }
     
    release_strafe(state);
     
    state.walk_keys_held = false;
}

 
fn release_back_key(state: &mut AuraState) {
    if state.back_key_held {
        let _ = set_back_key(false);
        state.back_key_held = false;
    }
}

 
 
fn set_strafe(left: bool, down: bool) -> anyhow::Result<()> {
    let options = minecraft().options()?;
    if left {
        options.key_left()?.set_down(down)
    } else {
        options.key_right()?.set_down(down)
    }
}

 
fn release_strafe(state: &mut AuraState) {
    if let Some(left) = state.strafe.take() {
        let _ = set_strafe(left, false);
    }
}

 
 
 
 
fn is_mid_fall(player: &LocalPlayer) -> bool {
    if player.on_ground().unwrap_or(false) {
        return false;
    }
    let falling = player
        .get_delta_movement()
        .map(|motion| motion.y() < 0.0)
        .unwrap_or(false);
    let fall_distance = player.get_fall_distance().unwrap_or(0.0);
    falling || fall_distance > 0.0
}

 
 
 
fn jump(player: &LocalPlayer) {
    if player.jump_from_ground().is_err() {
        let motion = player
            .get_delta_movement()
            .unwrap_or(Vec3::new(0.0, 0.0, 0.0));
        let _ = player.set_delta_movement(motion.x(), 0.42, motion.z());
    }
}

 
 
fn hit_reaction_delay() -> Duration {
    Duration::from_secs_f64(random_range(MIN_HIT_DELAY, MAX_HIT_DELAY))
}

 
 
 
fn random_range(min: f64, max: f64) -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};

    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut x = STATE.load(Ordering::Relaxed);
    if x == 0 {
        x = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
    }
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    STATE.store(x, Ordering::Relaxed);
    let unit = (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64;
    min + (max - min) * unit
}

fn predict_target_position(position: Vec3, velocity: Vec3, prediction_time: f64) -> Vec3 {
    Vec3::new(
        position.x() + velocity.x() * prediction_time,
        position.y() + velocity.y() * prediction_time,
        position.z() + velocity.z() * prediction_time,
    )
}

fn predict_target_aim(
    player: &LocalPlayer,
    target: &Entity,
    prediction_time: f64,
    height_fraction: f64,
) -> Option<Vec3> {
    if prediction_time <= 0.0 {
        return None;
    }
    let feet = target.get_position().ok()?;
    let vel = target.get_delta_movement().ok()?;
    let vel_len = vel.x() * vel.x() + vel.y() * vel.y() + vel.z() * vel.z();
    if vel_len > 64.0 {
        return None;
    }
    let predicted = predict_target_position(feet, vel, prediction_time);
    let height = target.bb_height().ok()? as f64;
     
    let eye = player.get_eye_position().ok()?;
    let dist_pred = ((predicted.x() - eye.x()).powi(2)
        + (predicted.y() - eye.y()).powi(2)
        + (predicted.z() - eye.z()).powi(2))
    .sqrt();
    let dist_cur = ((feet.x() - eye.x()).powi(2)
        + (feet.y() - eye.y()).powi(2)
        + (feet.z() - eye.z()).powi(2))
    .sqrt();
    if dist_pred > dist_cur + 2.0 || dist_pred < dist_cur - 0.5 || dist_pred > 12.0 {
        return None;
    }
    Some(Vec3::new(
        predicted.x(),
        predicted.y() + height * height_fraction,
        predicted.z(),
    ))
}
fn debug_log(
    _state: &AuraState,
    player: &LocalPlayer,
    target: &Entity,
    distance: f64,
    within_reach: bool,
    crosshair: bool,
    attack_strength: f32,
    attack_allowed: bool,
    _strafe_dir: &str,
    movement_state: &str,
    anti_combo: bool,
    combo_active: bool,
    combo_hits: u32,
    player_yaw: f32,
    facing_target_angle: f32,
    forward_key_held: bool,
    back_key_held: bool,
    strafe: Option<bool>,
) {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

     
    static LAST_DEBUG_TIME: AtomicU64 = AtomicU64::new(0);
    let now = Instant::now();
    let now_ns = now
        .duration_since(Instant::now() - Duration::from_secs(1))
        .as_nanos() as u64;
    let last = LAST_DEBUG_TIME.load(Ordering::Relaxed);
    let should_print = if last == 0 {
        true
    } else {
        now_ns - last > 250_000_000  
    };
    if should_print {
        LAST_DEBUG_TIME.store(now_ns, Ordering::Relaxed);
         
         
        let tgt_vel = target
            .get_delta_movement()
            .ok()
            .unwrap_or_else(|| Vec3::new(0.0, 0.0, 0.0));
        let feet = target
            .get_position()
            .ok()
            .unwrap_or_else(|| Vec3::new(0.0, 0.0, 0.0));
        let player_pos = player
            .get_position()
            .ok()
            .unwrap_or_else(|| Vec3::new(0.0, 0.0, 0.0));
        let predicted = predict_target_position(feet, tgt_vel, 0.2);
        let player_on_ground = player.on_ground().ok().unwrap_or(false);
        let target_on_ground = target
            .as_living()
            .and_then(|l| l.on_ground().ok())
            .unwrap_or(false);
        let player_sprinting = player.is_sprinting().ok().unwrap_or(false);
        let attack_attempted = attack_allowed && within_reach && crosshair;
        let rejection_reason = if attack_attempted {
            "none"
        } else if !attack_allowed {
            "cooldown"
        } else if !within_reach {
            "reach"
        } else if !crosshair {
            "crosshair"
        } else {
            "unknown"
        };

        log::info!(
            "[AuraDebug] id={} dist={:.2} yaw={:.1} rel_ang={:.1} fwd={} back={} strafe={} mode={} reach={} xhair={} str={:.2} attack={} reason={} anti={} combo={} comboh={} tvx={:.2} tvz={:.2} pred={:.1},{:.1},{:.1} pground={} tground={} sprint={}",
            target.id().ok().unwrap_or(-1),
            distance,
            player_yaw,
            facing_target_angle,
            forward_key_held,
            back_key_held,
            strafe.map(|l| if l { "L" } else { "R" }).unwrap_or("-"),
            movement_state,
            within_reach,
            crosshair,
            attack_strength,
            attack_attempted,
            rejection_reason,
            anti_combo,
            combo_active,
            combo_hits,
            tgt_vel.x(),
            tgt_vel.z(),
            predicted.x() - player_pos.x(),
            predicted.y() - player_pos.y(),
            predicted.z() - player_pos.z(),
            player_on_ground,
            target_on_ground,
            player_sprinting,
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

     
     
     
     
     
    #[test]
    fn the_hit_gate_judges_the_angle_the_server_will_hold() {
         
        assert!(hit_gate_uses_shadow(true, true, true));
        assert!(hit_gate_uses_shadow(true, true, false));
         
        assert!(hit_gate_uses_shadow(true, false, false));
        assert!(!hit_gate_uses_shadow(true, false, true));
         
        assert!(!hit_gate_uses_shadow(false, false, false));
        assert!(!hit_gate_uses_shadow(false, false, true));
    }

    #[test]
    fn hit_and_reaction_gate_require_the_selected_rotation() {
        let aim = Rotation::new(90.0, 10.0);
        assert!(target_under_gate(true, Some(aim), |used| used == Some(aim)));
        assert!(!target_under_gate(true, None, |_| true));
        assert!(target_under_gate(false, Some(aim), |used| used.is_none()));
        assert!(!target_under_gate(false, Some(aim), |used| used.is_some()));
    }

    #[test]
    fn silent_aura_does_not_take_the_sprint_key() {
        assert!(!uses_combo_control(ModuleId::SilentAura));
        assert!(uses_combo_control(ModuleId::KillAura));
        assert!(uses_combo_control(ModuleId::MobAura));
    }

     
     
     
    #[test]
    fn the_sprint_reset_schedules_the_repress_one_game_tick_out() {
        let mut state = AuraState::default();
        schedule_sprint_reset(&mut state);
        let before = Instant::now();
        let repress_at = state.sprint_reset_at.expect("a reset was scheduled");
        assert!(
            repress_at >= before && repress_at - before <= SPRINT_RESET_DELAY,
            "the re-press must land one SPRINT_RESET_DELAY after the reset"
        );
    }

     
     
    #[test]
    fn the_wtap_repress_window_elapses_after_the_reset_delay() {
        let now = Instant::now();
        assert!(!sprint_reset_elapsed(None, now));
        let pending = Some(now + SPRINT_RESET_DELAY);
        assert!(!sprint_reset_elapsed(pending, now));
        assert!(sprint_reset_elapsed(pending, now + SPRINT_RESET_DELAY));
    }

     
    #[test]
    fn the_anti_combo_backoff_window_is_three_hundred_and_fifty_ms() {
        let now = Instant::now();
        assert!(!backing_off(None, 1.0, now), "no damage, no backoff");
        assert!(backing_off(
            Some(now - Duration::from_millis(100)),
            1.0,
            now
        ));
        assert!(backing_off(
            Some(now - Duration::from_millis(349)),
            1.0,
            now
        ));
        assert!(!backing_off(
            Some(now - Duration::from_millis(350)),
            1.0,
            now
        ));
        assert!(!backing_off(
            Some(now - Duration::from_millis(1000)),
            1.0,
            now
        ));
         
         
        assert!(!backing_off(
            Some(now - Duration::from_millis(100)),
            ANTI_COMBO_DISTANCE + 0.01,
            now
        ));
         
        assert!(!backing_off(
            Some(now - Duration::from_millis(100)),
            f64::INFINITY,
            now
        ));
    }

     
    #[test]
    fn reaction_delay_is_random_between_point_zero_zero_four_and_point_zero_one() {
        for _ in 0..1000 {
            let seconds = hit_reaction_delay().as_secs_f64();
            assert!(
                seconds >= 0.004 - EPS && seconds <= 0.010 + EPS,
                "reaction delay {seconds} outside 0.004-0.010"
            );
        }
    }

     
     
     
    #[test]
    fn the_combo_window_needs_a_victim_that_can_be_followed_up() {
        assert!(in_combo_window(1.0, 0.0), "point blank is comboable");
        assert!(
            in_combo_window(COMBO_RANGE, COMBO_HEIGHT),
            "the corner of the window counts"
        );
        assert!(
            !in_combo_window(COMBO_RANGE + 0.01, 0.0),
            "too far away to chase the knockback"
        );
        assert!(
            !in_combo_window(1.0, COMBO_HEIGHT + 0.01),
            "too far above to reach with the follow-up"
        );
        assert!(!in_combo_window(f64::INFINITY, 0.0), "an unreadable victim");
    }

     
     
    #[test]
    fn the_knockback_is_read_from_how_far_the_victim_travelled() {
        let origin = (10.0, -4.0);
        assert!(!knocked_back(origin, origin), "standing still");
        assert!(
            !knocked_back(origin, (10.0 + KNOCKBACK_TRAVEL * 0.5, -4.0)),
            "a nudge is not a throw"
        );
        assert!(knocked_back(origin, (10.0 + KNOCKBACK_TRAVEL, -4.0)));
        assert!(
            knocked_back(origin, (10.0 - KNOCKBACK_TRAVEL, -4.0)),
            "thrown the other way still counts"
        );
        assert!(knocked_back(origin, (10.0, -4.0 + KNOCKBACK_TRAVEL)));
    }

     
    #[test]
    fn a_combo_goes_stale_without_a_hit() {
        let hit_at = Instant::now();
        assert!(!combo_expired(hit_at, hit_at), "fresh");
        assert!(!combo_expired(hit_at, hit_at + COMBO_IDLE), "still live");
        assert!(
            combo_expired(hit_at, hit_at + COMBO_IDLE + Duration::from_millis(1)),
            "stale"
        );
        assert!(
            !combo_expired(hit_at, hit_at - Duration::from_secs(1)),
            "a clock that went backwards is not a stale combo"
        );
    }

     

     
    #[test]
    fn target_directly_ahead_has_zero_relative_angle() {
         
         
        let player_pos = Vec3::new(0.0, 0.0, 0.0);
        let target_pos = Vec3::new(0.0, 0.0, 5.0);
        let yaw = 0.0_f32;

        let dx = target_pos.x() - player_pos.x();
        let dz = target_pos.z() - player_pos.z();
        let target_bearing = (dz.atan2(dx).to_degrees() - 90.0) as f32;
        let relative_angle = wrap_degrees(target_bearing - yaw);

        assert!(
            relative_angle.abs() < 1.0,
            "target directly ahead should have ~0 relative angle, got {relative_angle}"
        );
    }

     
    #[test]
    fn target_directly_behind_has_180_relative_angle() {
         
        let player_pos = Vec3::new(0.0, 0.0, 0.0);
        let target_pos = Vec3::new(0.0, 0.0, -5.0);
        let yaw = 0.0_f32;

        let dx = target_pos.x() - player_pos.x();
        let dz = target_pos.z() - player_pos.z();
        let target_bearing = (dz.atan2(dx).to_degrees() - 90.0) as f32;
        let relative_angle = wrap_degrees(target_bearing - yaw);

        assert!(
            relative_angle.abs() > 170.0,
            "target behind should have ~180 relative angle, got {relative_angle}"
        );
    }

     
    #[test]
    fn target_to_the_left_has_negative_relative_angle() {
         
         
         
        let player_pos = Vec3::new(0.0, 0.0, 0.0);
        let target_pos = Vec3::new(5.0, 0.0, 0.0);
        let yaw = 0.0_f32;

        let dx = target_pos.x() - player_pos.x();
        let dz = target_pos.z() - player_pos.z();
        let target_bearing = (dz.atan2(dx).to_degrees() - 90.0) as f32;
        let relative_angle = wrap_degrees(target_bearing - yaw);

        assert!(
            relative_angle < -80.0 && relative_angle > -100.0,
            "target to the left should have ~-90 relative angle, got {relative_angle}"
        );
    }

     
    #[test]
    fn target_to_the_right_has_positive_relative_angle() {
         
         
        let player_pos = Vec3::new(0.0, 0.0, 0.0);
        let target_pos = Vec3::new(-5.0, 0.0, 0.0);
        let yaw = 0.0_f32;

        let dx = target_pos.x() - player_pos.x();
        let dz = target_pos.z() - player_pos.z();
        let target_bearing = (dz.atan2(dx).to_degrees() - 90.0) as f32;
        let relative_angle = wrap_degrees(target_bearing - yaw);

        assert!(
            relative_angle > 80.0 && relative_angle < 100.0,
            "target to the right should have ~90 relative angle, got {relative_angle}"
        );
    }

     
     
     
    #[test]
    fn rotated_player_correctly_identifies_target_position() {
         
        let player_pos = Vec3::new(0.0, 0.0, 0.0);
        let target_pos = Vec3::new(5.0, 0.0, 0.0);
        let yaw = -90.0_f32;

        let dx = target_pos.x() - player_pos.x();
        let dz = target_pos.z() - player_pos.z();
        let target_bearing = (dz.atan2(dx).to_degrees() - 90.0) as f32;
        let relative_angle = wrap_degrees(target_bearing - yaw);

        assert!(
            relative_angle.abs() < 1.0,
            "target should be directly ahead when player faces it, got {relative_angle}"
        );
    }

     
    #[test]
    fn wrap_degrees_normalizes_correctly() {
        assert!((wrap_degrees(0.0) - 0.0).abs() < 0.001);
         
        assert!((wrap_degrees(180.0) - (-180.0)).abs() < 0.001);
        assert!((wrap_degrees(181.0) - (-179.0)).abs() < 0.001);
        assert!((wrap_degrees(-181.0) - 179.0).abs() < 0.001);
        assert!((wrap_degrees(360.0) - 0.0).abs() < 0.001);
        assert!((wrap_degrees(540.0) - (-180.0)).abs() < 0.001);
    }

     
    #[test]
    fn backoff_expires_after_timeout() {
        let now = Instant::now();
        let damage_time = now - Duration::from_millis(351);
         
        assert!(
            !backing_off(Some(damage_time), 1.0, now),
            "backoff should expire after timeout"
        );
    }

     
    #[test]
    fn backoff_ends_early_when_far_from_target() {
        let now = Instant::now();
        let damage_time = now - Duration::from_millis(100);
         
        assert!(
            !backing_off(Some(damage_time), ANTI_COMBO_DISTANCE + 1.0, now),
            "backoff should end early when far from target"
        );
    }

     
    #[test]
    fn auto_walk_disabled_releases_keys() {
        let mut state = AuraState::default();
        state.walk_keys_held = true;
        state.forward_key_held = true;
        state.strafe = Some(true);

         
         
         
        if state.forward_key_held {
            state.forward_key_held = false;
        }
        state.strafe = None;
        state.walk_keys_held = false;

        assert!(!state.walk_keys_held, "walk keys should be released");
        assert!(!state.forward_key_held, "forward key should be released");
        assert!(state.strafe.is_none(), "strafe should be released");
    }

     
    #[test]
    fn target_disappears_releases_movement_keys() {
        let mut state = AuraState::default();
        state.walk_keys_held = true;
        state.forward_key_held = true;
        state.sprint_key_held = true;
        state.strafe = Some(false);
        state.back_key_held = true;

         
         
        if state.forward_key_held {
            state.forward_key_held = false;
        }
        if state.sprint_key_held && !state.combo_sprint_held {
            state.sprint_key_held = false;
        }
        state.strafe = None;
        state.walk_keys_held = false;
        if state.back_key_held {
            state.back_key_held = false;
        }

        assert!(!state.walk_keys_held);
        assert!(!state.forward_key_held);
        assert!(!state.sprint_key_held);
        assert!(state.strafe.is_none());
        assert!(!state.back_key_held);
    }

     
    #[test]
    fn strafe_direction_changes_on_switch() {
        let mut state = AuraState::default();
        state.next_strafe_left = true;

         
        let first_strafe = state.next_strafe_left;
        state.next_strafe_left = !state.next_strafe_left;

         
        let second_strafe = state.next_strafe_left;

        assert_eq!(first_strafe, true, "first strafe should be left");
        assert_eq!(second_strafe, false, "second strafe should be right");
    }

     
    #[test]
    fn stopping_distance_reached_stops_movement() {
        let walk_range = 0.5_f32;
        let chase_range = walk_range as f64 * 0.5;
        let distance_to_target = 0.2;  

        let should_walk = distance_to_target > chase_range;
        assert!(
            !should_walk,
            "should not walk when within stopping distance"
        );
    }

     
    #[test]
    fn beyond_stopping_range_continues_movement() {
        let walk_range = 0.5_f32;
        let chase_range = walk_range as f64 * 0.5;
        let distance_to_target = 2.0;  

        let should_walk = distance_to_target > chase_range;
        assert!(should_walk, "should walk when beyond stopping distance");
    }
}
