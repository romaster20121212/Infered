 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::state::minecraft;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;

 
 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
     
    Block,
     
    Entity,
}

#[derive(Debug, Default)]
struct Ledger {
     
    tick: Option<i32>,
    block: bool,
    entity: bool,
     
    screen: bool,
     
    screen_tick: Option<i32>,
}

static LEDGER: Mutex<Ledger> = Mutex::new(Ledger {
    tick: None,
    block: false,
    entity: false,
    screen: false,
    screen_tick: None,
});

 
 
 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RightButton {
     
    CrystalSpam,
     
    AnchorMacro,
     
    CrystalMacro,
}

impl RightButton {
    fn bit(self) -> u8 {
        match self {
            RightButton::CrystalSpam => 1,
            RightButton::AnchorMacro => 2,
            RightButton::CrystalMacro => 4,
        }
    }
}

 
 
static RIGHT_BUTTON_OWNERS: AtomicU8 = AtomicU8::new(0);

 
 
pub(crate) fn owns_right_button() -> bool {
    RIGHT_BUTTON_OWNERS.load(Ordering::Acquire) != 0
}

 
 
pub(crate) fn owns(owner: RightButton) -> bool {
    RIGHT_BUTTON_OWNERS.load(Ordering::Acquire) & owner.bit() != 0
}

 
 
 
 
pub(crate) fn claim_right_button(owner: RightButton) {
    if RIGHT_BUTTON_OWNERS.fetch_or(owner.bit(), Ordering::AcqRel) & owner.bit() != 0 {
        return;
    }
    let released = minecraft()
        .options()
        .and_then(|options| options.key_use())
        .and_then(|key| key.set_down(false));
    if let Err(error) = released {
         
         
        log::warn!("could not release the vanilla use key: {error}");
    }
}

 
 
pub(crate) fn release_right_button(owner: RightButton) {
    RIGHT_BUTTON_OWNERS.fetch_and(!owner.bit(), Ordering::AcqRel);
}

 
 
 
 
 
pub(crate) fn take(player: &LocalPlayer, action: Action) -> bool {
    let Ok(tick) = player.get_tick_count() else {
         
         
         
         
        log::debug!("interaction gate: no tick count, sending unpaced");
        return true;
    };
    let screen_open = minecraft().current_screen().is_open();
    grant(&mut LEDGER.lock().unwrap(), tick, action, screen_open)
}

 
 
 
fn grant(ledger: &mut Ledger, tick: i32, action: Action, screen_open: bool) -> bool {
    if ledger.tick != Some(tick) {
         
         
         
        if ledger.tick.is_some_and(|last| tick < last) {
            ledger.screen = false;
            ledger.screen_tick = None;
        }
        ledger.tick = Some(tick);
        ledger.block = false;
        ledger.entity = false;
    }
     
     
     
     
    if screen_open {
        ledger.screen = true;
        ledger.screen_tick = Some(tick);
        return false;
    }
    if ledger.screen {
         
         
        ledger.screen = false;
        ledger.screen_tick = Some(tick);
    }
    if ledger
        .screen_tick
        .is_some_and(|at| tick <= at.saturating_add(1))
    {
        return false;
    }
    match action {
        Action::Block if !ledger.block && !ledger.entity => {
            ledger.block = true;
            true
        }
        Action::Entity if !ledger.entity && !ledger.block => {
            ledger.entity = true;
            true
        }
        _ => false,
    }
}

 
 
 
 
 
 
 
 
pub(crate) fn window_click_allowed(player: &LocalPlayer) -> bool {
    let sprinting = player.is_sprinting().unwrap_or(false);
    let moving = player.has_move_input().unwrap_or(false);
    click_is_still(sprinting, moving)
}

 
 
fn click_is_still(sprinting: bool, moving: bool) -> bool {
    !sprinting && !moving
}

#[cfg(test)]
mod tests {
    use super::{click_is_still, grant, Action, Ledger};

    #[test]
    fn a_tick_hands_out_one_action_of_either_kind() {
        let mut ledger = Ledger::default();
        assert!(grant(&mut ledger, 7, Action::Block, false));
         
         
        assert!(!grant(&mut ledger, 7, Action::Block, false));
        assert!(!grant(&mut ledger, 7, Action::Entity, false));

        let mut ledger = Ledger::default();
        assert!(grant(&mut ledger, 7, Action::Entity, false));
        assert!(!grant(&mut ledger, 7, Action::Block, false));
        assert!(!grant(&mut ledger, 7, Action::Entity, false));

         
        assert!(grant(&mut ledger, 8, Action::Block, false));
        assert!(!grant(&mut ledger, 8, Action::Entity, false));
        assert!(grant(&mut ledger, 9, Action::Entity, false));
    }

     
     
    #[test]
    fn a_counter_that_moves_backwards_still_opens_a_new_tick() {
        let mut ledger = Ledger::default();
        assert!(grant(&mut ledger, 900, Action::Block, false));
        assert!(!grant(&mut ledger, 900, Action::Block, false));
        assert!(grant(&mut ledger, 1, Action::Block, false));
    }

     
     
     
    #[test]
    fn a_tick_spent_in_the_inventory_is_not_a_tick_to_interact_on() {
        let mut ledger = Ledger::default();
        assert!(grant(&mut ledger, 10, Action::Block, false));
         
        assert!(!grant(&mut ledger, 11, Action::Block, true));
        assert!(!grant(&mut ledger, 11, Action::Entity, true));
         
        assert!(!grant(&mut ledger, 12, Action::Block, false));
        assert!(!grant(&mut ledger, 13, Action::Block, false));
        assert!(grant(&mut ledger, 14, Action::Block, false));
    }

     
     
    #[test]
    fn a_new_player_entity_forgets_the_screen_history() {
        let mut ledger = Ledger::default();
        assert!(!grant(&mut ledger, 400, Action::Block, true));
        assert!(!grant(&mut ledger, 401, Action::Block, false));
         
        assert!(grant(&mut ledger, 1, Action::Block, false));
    }

     
     
    #[test]
    fn right_button_claims_are_per_owner() {
        use super::{
            claim_right_button, owns, owns_right_button, release_right_button, RightButton,
        };
         
         
        claim_right_button(RightButton::CrystalSpam);
        assert!(owns_right_button());
        assert!(owns(RightButton::CrystalSpam));
        assert!(!owns(RightButton::AnchorMacro));
        claim_right_button(RightButton::AnchorMacro);
        release_right_button(RightButton::CrystalSpam);
        assert!(
            owns_right_button(),
            "the other loop still drives the button"
        );
        assert!(!owns(RightButton::CrystalSpam));
        assert!(owns(RightButton::AnchorMacro));
        release_right_button(RightButton::AnchorMacro);
        assert!(!owns_right_button());
    }

     
     
    #[test]
    fn a_window_click_waits_for_a_still_tick() {
        assert!(click_is_still(false, false));
        assert!(!click_is_still(true, false));
        assert!(!click_is_still(false, true));
        assert!(!click_is_still(true, true));
    }
}
