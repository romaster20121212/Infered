use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::graphic::input::GUI_OPEN;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{mapping, minecraft};
use jni::objects::JValue;

 
 
 
static HELD_OFF_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

 
 
pub(crate) fn hold_sprint_off(window: Duration) {
    let deadline = Instant::now() + window;
    let mut held = HELD_OFF_UNTIL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *held = Some(later_hold(*held, deadline));
}

 
 
fn later_hold(held: Option<Instant>, deadline: Instant) -> Instant {
    match held {
        Some(until) if until > deadline => until,
        _ => deadline,
    }
}

 
pub(crate) fn release_sprint_hold() {
    *HELD_OFF_UNTIL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

 
fn sprint_held_off() -> bool {
    let mut held = HELD_OFF_UNTIL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if hold_active(*held, Instant::now()) {
        true
    } else {
        *held = None;
        false
    }
}

 
fn hold_active(held: Option<Instant>, now: Instant) -> bool {
    held.is_some_and(|until| now < until)
}

 
fn wants_bhop_jump(bhop: bool, moving_forward: bool, screen_open: bool) -> bool {
    bhop && moving_forward && !screen_open
}

 
 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct SprintModule {
    pub module: ModuleData,
     
     
    key_held: AtomicBool,
     
    jump_held: AtomicBool,
}

impl SprintModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Sprint,
                description: "Sprints automatically while moving".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Toggle {
                    name: "Bhop".to_string(),
                    value: false,
                }],
            },
            key_held: AtomicBool::new(false),
            jump_held: AtomicBool::new(false),
        }
    }

    fn bhop(&self) -> bool {
        self.module
            .get_setting("Bhop")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

     
     
    fn set_sprint_key_held(&self, held: bool) -> anyhow::Result<()> {
        if held {
            minecraft().options()?.key_sprint()?.set_down(true)?;
            self.key_held.store(true, Ordering::Relaxed);
        } else if self.key_held.swap(false, Ordering::Relaxed) {
            minecraft().options()?.key_sprint()?.set_down(false)?;
        }
        Ok(())
    }

     
     
    fn set_jump_key_held(&self, held: bool) -> anyhow::Result<()> {
        if held {
            let key = minecraft().options()?.key_jump()?;
            if !self.jump_held.load(Ordering::Relaxed) && !key.is_down()? {
                key.set_down(true)?;
                self.jump_held.store(true, Ordering::Relaxed);
            }
        } else if self.jump_held.swap(false, Ordering::Relaxed) {
            minecraft().options()?.key_jump()?.set_down(false)?;
        }
        Ok(())
    }
}

 
 
 
fn clear_jump_cooldown(player: &LocalPlayer) -> anyhow::Result<()> {
    mapping().set_field(
        MinecraftClassType::LivingEntity,
        player.jni_ref().as_obj(),
        "noJumpDelay",
        FieldType::Int,
        JValue::Int(0),
    )
}

impl Module for SprintModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.set_sprint_key_held(false)?;
        self.set_jump_key_held(false)?;
        if let Some(player) = minecraft().player()? {
            let _ = player.set_sprinting(false);
        }
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            self.set_sprint_key_held(false)?;
            return self.set_jump_key_held(false);
        };

         
         
         
         
        let (_, forward) = player.move_input()?;
        let wants_sprint = forward > 0.0 && !player.is_shift_key_down()? && !sprint_held_off();
        self.set_sprint_key_held(wants_sprint)?;

        let screen_open =
            minecraft().current_screen().is_open() || GUI_OPEN.load(Ordering::Relaxed);
        let bhop_jump = wants_bhop_jump(self.bhop(), wants_sprint, screen_open);
        self.set_jump_key_held(bhop_jump)?;

        if bhop_jump && player.is_alive()? {
            clear_jump_cooldown(&player)?;
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
    use super::{hold_active, later_hold, wants_bhop_jump};
    use std::time::{Duration, Instant};

    #[test]
    fn overlapping_sprint_holds_keep_the_later_deadline() {
        let now = Instant::now();
        let short = now + Duration::from_millis(50);
        let long = now + Duration::from_millis(150);
         
        assert_eq!(later_hold(Some(short), long), long);
         
        assert_eq!(later_hold(Some(long), short), long);
         
        assert_eq!(later_hold(None, short), short);
    }

    #[test]
    fn a_sprint_hold_ends_at_its_deadline() {
        let start = Instant::now();
        let deadline = start + Duration::from_millis(100);
        assert!(!hold_active(None, start));
        assert!(hold_active(Some(deadline), start));
        assert!(hold_active(
            Some(deadline),
            start + Duration::from_millis(99)
        ));
        assert!(!hold_active(Some(deadline), deadline));
        assert!(!hold_active(
            Some(deadline),
            deadline + Duration::from_secs(1)
        ));
    }

    #[test]
    fn bhop_only_holds_jump_while_moving_and_no_screen_is_open() {
        assert!(wants_bhop_jump(true, true, false));
        assert!(!wants_bhop_jump(false, true, false), "Bhop is off");
        assert!(!wants_bhop_jump(true, false, false), "not moving forward");
        assert!(!wants_bhop_jump(true, true, true), "a screen owns input");
    }
}
