 
 
 
 
 
 
 
 

use std::collections::BTreeMap;

use anyhow::anyhow;
use infered_macros::{
    action_summary, condition_summary, Action, Condition, ErrorPolicy, KeyAction, Limits, Macro,
    MovementKey, MAX_DEPTH, MAX_ITERATIONS, MAX_RUN_DEPTH,
};
use serde_json::Value as Json;

use super::conds;
use super::host::{Host, Snapshot};

 
 
pub const ACTION_BUDGET: usize = 128;

 
pub const TICK_MS: u64 = 50;

 
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Held {
    Movement(MovementKey),
    Use,
}

impl Held {
    fn apply(self, host: &mut dyn Host, down: bool) -> anyhow::Result<()> {
        match self {
            Held::Movement(key) => host.set_movement(key, down),
            Held::Use => host.set_use(down),
        }
    }

    fn label(self) -> String {
        match self {
            Held::Movement(key) => key.label().to_string(),
            Held::Use => "Use item".to_string(),
        }
    }
}

 
 
#[derive(Debug, Default)]
pub struct Ownership {
    owners: BTreeMap<Held, String>,
}

impl Ownership {
    fn acquire(&mut self, held: Held, run_id: &str) -> Result<bool, String> {
        match self.owners.get(&held) {
            Some(owner) if owner == run_id => Ok(false),
            Some(owner) => Err(format!("{} is already held by {owner}", held.label())),
            None => {
                self.owners.insert(held, run_id.to_string());
                Ok(true)
            }
        }
    }

    fn release(&mut self, held: Held, run_id: &str) -> bool {
        if self.owners.get(&held).is_some_and(|owner| owner == run_id) {
            self.owners.remove(&held);
            true
        } else {
            false
        }
    }

     
     
    pub fn release_all(&mut self, run_id: &str) -> Vec<Held> {
        let owned: Vec<Held> = self
            .owners
            .iter()
            .filter(|(_, owner)| owner.as_str() == run_id)
            .map(|(held, _)| *held)
            .collect();
        for held in &owned {
            self.owners.remove(held);
        }
        owned
    }

     
    pub fn owner(&self, held: Held) -> Option<&str> {
        self.owners.get(&held).map(String::as_str)
    }

     
    pub fn held(&self) -> Vec<(Held, String)> {
        self.owners
            .iter()
            .map(|(held, owner)| (*held, owner.clone()))
            .collect()
    }
}

 
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng {
            state: seed.max(0x9E37_79B9_7F4A_7C15),
        }
    }

    pub fn next_u64(&mut self) -> u64 {
         
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

     
    pub fn range(&mut self, min: u64, max: u64) -> u64 {
        if max <= min {
            return min;
        }
        min + self.next_u64() % (max - min + 1)
    }
}

#[derive(Debug)]
enum FrameKind {
    Plain,
    Repeat {
        remaining: u32,
    },
    While {
        condition: Condition,
        remaining: u32,
    },
    Until {
        condition: Condition,
        remaining: u32,
    },
}

#[derive(Debug)]
struct Frame {
    actions: Vec<Action>,
    index: usize,
    kind: FrameKind,
     
     
    child: Option<String>,
}

 
#[derive(Debug)]
pub struct Run {
    pub id: String,
    pub name: String,
    pub started_ms: u64,
    pub updated_ms: u64,
    pub status: String,
    pub error: Option<String>,
    pub warnings: u32,
    pub finished: bool,
     
    pub stop_all: bool,
    frames: Vec<Frame>,
    vars: BTreeMap<String, Json>,
    wait_ready_ms: Option<u64>,
    wait_condition: Option<(Condition, u64)>,
    releases: Vec<(u64, Held)>,
    held: Vec<Held>,
    limits: Limits,
    steps: u64,
    iterations: u32,
    call_stack: Vec<String>,
}

impl Run {
    pub fn new(macro_: &Macro, event_vars: BTreeMap<String, Json>, now_ms: u64) -> Run {
        let mut vars = macro_.variables.clone();
        vars.extend(event_vars);
        Run {
            id: macro_.id.clone(),
            name: macro_.name.clone(),
            started_ms: now_ms,
            updated_ms: now_ms,
            status: "Starting".into(),
            error: None,
            warnings: 0,
            finished: false,
            stop_all: false,
            frames: vec![Frame {
                actions: macro_.actions.clone(),
                index: 0,
                kind: FrameKind::Plain,
                child: None,
            }],
            vars,
            wait_ready_ms: None,
            wait_condition: None,
            releases: Vec::new(),
            held: Vec::new(),
            limits: macro_.limits,
            steps: 0,
            iterations: 0,
            call_stack: vec![macro_.id.clone()],
        }
    }

     
    pub fn waiting(&self) -> bool {
        self.wait_ready_ms.is_some() || self.wait_condition.is_some()
    }

     
     
    pub fn release_inputs(&mut self, host: &mut dyn Host, ownership: &mut Ownership) {
        for held in ownership.release_all(&self.id) {
            let _ = held.apply(host, false);
        }
        self.held.clear();
        self.releases.clear();
    }

     
    #[allow(clippy::too_many_arguments)]
    pub fn advance(
        &mut self,
        host: &mut dyn Host,
        snapshot: &Snapshot,
        now_ms: u64,
        rng: &mut Rng,
        ownership: &mut Ownership,
        catalog: &BTreeMap<String, std::sync::Arc<Macro>>,
    ) {
        if self.finished {
            return;
        }
        self.updated_ms = now_ms;

        if now_ms.saturating_sub(self.started_ms) > self.limits.max_runtime_ms {
            self.fail(format!(
                "runtime limit of {} ms reached",
                self.limits.max_runtime_ms
            ));
            return;
        }

        self.release_due(host, ownership, now_ms);

        if let Some(deadline) = self.wait_ready_ms {
            if now_ms < deadline {
                return;
            }
            self.wait_ready_ms = None;
        }

        if let Some((condition, deadline)) = self.wait_condition.clone() {
            if conds::evaluate(&condition, &self.vars, snapshot) {
                self.wait_condition = None;
            } else if now_ms >= deadline {
                self.wait_condition = None;
                self.action_failed(
                    host,
                    format!("timed out waiting for {}", condition_summary(&condition)),
                );
                if self.finished {
                    return;
                }
            } else {
                return;
            }
        }

        let mut budget = ACTION_BUDGET;
        while budget > 0 && !self.finished {
            budget -= 1;
            self.unwind(snapshot);
             
             
            if self.finished {
                return;
            }

            let Some(frame) = self.frames.last_mut() else {
                self.finish("Finished");
                return;
            };
            if frame.index >= frame.actions.len() {
                self.unwind(snapshot);
                continue;
            }
            let action = frame.actions[frame.index].clone();
            frame.index += 1;
            self.steps += 1;
            if self.steps > infered_macros::model::MAX_ACTIONS as u64 * 256 {
                self.fail("step safety limit reached".to_string());
                return;
            }
            self.status = action_summary(&action);

            if let Err(error) =
                self.perform(action, host, snapshot, now_ms, rng, ownership, catalog)
            {
                self.action_failed(host, error.to_string());
                if self.finished {
                    return;
                }
            }

            if self.wait_ready_ms.is_some() || self.wait_condition.is_some() {
                return;
            }
        }

         
        if self.wait_ready_ms.is_none() && self.wait_condition.is_none() && !self.finished {
            self.status = "Running".into();
        }
    }

     
     
    fn unwind(&mut self, snapshot: &Snapshot) {
        loop {
            let Some(frame) = self.frames.last_mut() else {
                return;
            };
            if frame.index < frame.actions.len() {
                return;
            }
            let kind_finished = match &mut frame.kind {
                FrameKind::Plain => true,
                FrameKind::Repeat { remaining } => {
                    *remaining = remaining.saturating_sub(1);
                    if *remaining > 0 {
                        frame.index = 0;
                        false
                    } else {
                        true
                    }
                }
                FrameKind::While {
                    condition,
                    remaining,
                } => {
                    *remaining = remaining.saturating_sub(1);
                    if *remaining > 0 && conds::evaluate(condition, &self.vars, snapshot) {
                        frame.index = 0;
                        false
                    } else {
                        true
                    }
                }
                FrameKind::Until {
                    condition,
                    remaining,
                } => {
                    *remaining = remaining.saturating_sub(1);
                    if *remaining > 0 && !conds::evaluate(condition, &self.vars, snapshot) {
                        frame.index = 0;
                        false
                    } else {
                        true
                    }
                }
            };
            if !kind_finished {
                self.iterations += 1;
                if self.iterations > self.limits.max_iterations {
                    self.fail(format!(
                        "iteration limit of {} reached",
                        self.limits.max_iterations
                    ));
                }
                return;
            }
            if let Some(frame) = self.frames.pop() {
                if let Some(child) = frame.child {
                    if self.call_stack.last().is_some_and(|id| *id == child) {
                        self.call_stack.pop();
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn perform(
        &mut self,
        action: Action,
        host: &mut dyn Host,
        snapshot: &Snapshot,
        now_ms: u64,
        rng: &mut Rng,
        ownership: &mut Ownership,
        catalog: &BTreeMap<String, std::sync::Arc<Macro>>,
    ) -> anyhow::Result<()> {
        match action {
            Action::Wait { duration_ms } => {
                self.wait_ready_ms =
                    Some(now_ms + duration_ms.min(infered_macros::model::MAX_DELAY_MS));
            }
            Action::WaitRandom { min_ms, max_ms } => {
                let duration = rng.range(
                    min_ms.min(infered_macros::model::MAX_DELAY_MS),
                    max_ms.min(infered_macros::model::MAX_DELAY_MS),
                );
                self.status = format!("Wait {}", infered_macros::duration(duration));
                self.wait_ready_ms = Some(now_ms + duration);
            }
            Action::WaitUntil {
                condition,
                timeout_ms,
            } => {
                self.wait_condition = Some((condition, now_ms + timeout_ms));
            }
            Action::Key {
                key,
                action,
                hold_ms,
            } => match action {
                KeyAction::Tap => host.tap_key(&key)?,
                KeyAction::Press => {
                    let movement = movement_key(&key)
                        .ok_or_else(|| anyhow!("{key} has no vanilla key mapping; use tap"))?;
                    self.hold(host, ownership, Held::Movement(movement), now_ms + hold_ms)?;
                }
                KeyAction::Release => {
                    let movement = movement_key(&key)
                        .ok_or_else(|| anyhow!("{key} has no vanilla key mapping; use tap"))?;
                    self.release(host, ownership, Held::Movement(movement));
                }
            },
            Action::Mouse {
                button,
                action,
                hold_ms,
            } => match (button, action) {
                (1, KeyAction::Tap) => host.attack()?,
                (2, KeyAction::Tap) => host.interact()?,
                (2, KeyAction::Press) => {
                    self.hold(host, ownership, Held::Use, now_ms + hold_ms)?;
                }
                (2, KeyAction::Release) => self.release(host, ownership, Held::Use),
                _ => anyhow::bail!("unsupported mouse button/action combination"),
            },
            Action::ReleaseInputs => {
                for held in ownership.release_all(&self.id) {
                    let _ = held.apply(host, false);
                }
                self.held.clear();
                self.releases.clear();
            }
            Action::Hotbar { slot } => host.hotbar(slot)?,
            Action::CycleHotbar { steps } => {
                let current = host.selected_slot()?.clamp(0, 8);
                let next = (current + steps as i32).rem_euclid(9);
                host.hotbar(next as u8 + 1)?;
            }
            Action::UseItem { hold_ms } => {
                self.hold(host, ownership, Held::Use, now_ms + hold_ms.max(TICK_MS))?;
            }
            Action::Attack => host.attack()?,
            Action::Interact => host.interact()?,
            Action::PickBlock => host.pick_block()?,
            Action::Movement {
                key,
                action,
                hold_ms,
            } => match action {
                KeyAction::Press => {
                    self.hold(host, ownership, Held::Movement(key), now_ms + hold_ms)?;
                }
                KeyAction::Release => self.release(host, ownership, Held::Movement(key)),
                KeyAction::Tap => {
                    self.hold(
                        host,
                        ownership,
                        Held::Movement(key),
                        now_ms + hold_ms.max(TICK_MS),
                    )?;
                }
            },
            Action::OpenInventory => host.open_inventory()?,
            Action::CloseScreen => host.close_screen()?,
            Action::Chat { message } => host.send_chat(&message)?,
            Action::SetVar { name, value } => {
                if !infered_macros::validate::is_valid_variable(&name) {
                    anyhow::bail!("invalid variable name {name:?}");
                }
                self.vars.insert(name, value);
            }
            Action::If {
                condition,
                then,
                otherwise,
            } => {
                let branch = if conds::evaluate(&condition, &self.vars, snapshot) {
                    then
                } else {
                    otherwise
                };
                if !branch.is_empty() {
                    self.push_frame(branch, FrameKind::Plain, None);
                }
            }
            Action::Repeat { times, actions } => {
                if times == 0 {
                    anyhow::bail!("repeat needs at least one iteration");
                }
                if !actions.is_empty() {
                    self.push_frame(
                        actions,
                        FrameKind::Repeat {
                            remaining: times.min(MAX_ITERATIONS),
                        },
                        None,
                    );
                }
            }
            Action::RepeatWhile {
                condition,
                max_iterations,
                actions,
            } => {
                if conds::evaluate(&condition, &self.vars, snapshot) && !actions.is_empty() {
                    self.push_frame(
                        actions,
                        FrameKind::While {
                            condition,
                            remaining: max_iterations.clamp(1, MAX_ITERATIONS),
                        },
                        None,
                    );
                }
            }
            Action::RepeatUntil {
                condition,
                max_iterations,
                actions,
            } => {
                if !actions.is_empty() {
                    self.push_frame(
                        actions,
                        FrameKind::Until {
                            condition,
                            remaining: max_iterations.clamp(1, MAX_ITERATIONS),
                        },
                        None,
                    );
                }
            }
            Action::RunMacro { id } => {
                if self.call_stack.len() > MAX_RUN_DEPTH
                    || self.call_stack.iter().any(|used| used == &id)
                {
                    anyhow::bail!("run_macro cycle or depth limit at {id}");
                }
                let Some(child) = catalog.get(&id) else {
                    anyhow::bail!("run_macro target {id:?} is not loaded");
                };
                if self.frames.len() >= MAX_DEPTH * 4 {
                    anyhow::bail!("macro call stack is too deep");
                }
                for (name, value) in &child.variables {
                    self.vars
                        .entry(name.clone())
                        .or_insert_with(|| value.clone());
                }
                self.call_stack.push(id);
                self.push_frame(
                    child.actions.clone(),
                    FrameKind::Plain,
                    self.call_stack.last().cloned(),
                );
            }
            Action::Stop => {
                self.finish("Stopped by macro");
            }
            Action::StopAll => {
                self.stop_all = true;
                self.finish("Stopped all macros");
            }
            Action::Log { message } => host.log(&format!("{}: {message}", self.name)),
        }
        Ok(())
    }

    fn push_frame(&mut self, actions: Vec<Action>, kind: FrameKind, child: Option<String>) {
        self.frames.push(Frame {
            actions,
            index: 0,
            kind,
            child,
        });
    }

     
    fn hold(
        &mut self,
        host: &mut dyn Host,
        ownership: &mut Ownership,
        held: Held,
        release_at: u64,
    ) -> anyhow::Result<()> {
        if ownership
            .acquire(held, &self.id)
            .map_err(|error| anyhow!(error))?
        {
            held.apply(host, true)?;
            self.held.push(held);
        }
        if release_at > 0 {
            self.releases.push((release_at, held));
        }
        Ok(())
    }

    fn release(&mut self, host: &mut dyn Host, ownership: &mut Ownership, held: Held) {
        if ownership.release(held, &self.id) {
            let _ = held.apply(host, false);
        }
        self.held.retain(|active| *active != held);
        self.releases.retain(|(_, active)| *active != held);
    }

    fn release_due(&mut self, host: &mut dyn Host, ownership: &mut Ownership, now_ms: u64) {
        let due: Vec<Held> = self
            .releases
            .iter()
            .filter(|(at, _)| *at <= now_ms)
            .map(|(_, held)| *held)
            .collect();
        for held in due {
            if ownership.release(held, &self.id) {
                let _ = held.apply(host, false);
            }
            self.held.retain(|active| *active != held);
            self.releases
                .retain(|(at, active)| !(*at <= now_ms && *active == held));
        }
    }

     
    fn action_failed(&mut self, host: &mut dyn Host, message: String) {
        match self.limits.on_error {
            ErrorPolicy::Stop => self.fail(message),
            ErrorPolicy::Continue => {
                self.warnings = self.warnings.saturating_add(1);
                self.error = Some(message.clone());
                host.log(&format!("{}: {message}", self.name));
            }
        }
    }

    fn fail(&mut self, message: String) {
        self.error = Some(message);
        self.status = "Failed".into();
        self.finished = true;
        self.wait_ready_ms = None;
        self.wait_condition = None;
    }

    fn finish(&mut self, status: &str) {
        self.status = status.to_string();
        self.finished = true;
        self.wait_ready_ms = None;
        self.wait_condition = None;
    }

     
    pub fn variables(&self) -> &BTreeMap<String, Json> {
        &self.vars
    }
}

 
fn movement_key(key: &str) -> Option<MovementKey> {
    let key = infered_macros::find_key(key)?;
    Some(match key.name {
        "W" => MovementKey::Forward,
        "S" => MovementKey::Back,
        "A" => MovementKey::Left,
        "D" => MovementKey::Right,
        "Space" => MovementKey::Jump,
        "Left Shift" => MovementKey::Sneak,
        "Left Control" => MovementKey::Sprint,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::test_host::FakeHost;
    use infered_macros::{Action, Trigger};

    fn run(macro_: &Macro) -> Run {
        Run::new(macro_, BTreeMap::new(), 0)
    }

    fn macro_with(actions: Vec<Action>) -> Macro {
        let mut macro_ = Macro::new_template("test", "Test");
        macro_.actions = actions;
        macro_.triggers = vec![Trigger::Manual];
        macro_
    }

    fn catalog(macro_: &Macro) -> BTreeMap<String, std::sync::Arc<Macro>> {
        BTreeMap::from([(macro_.id.clone(), std::sync::Arc::new(macro_.clone()))])
    }

     
     
    fn tick(
        run: &mut Run,
        host: &mut FakeHost,
        now: u64,
        rng: &mut Rng,
        ownership: &mut Ownership,
        catalog: &BTreeMap<String, std::sync::Arc<Macro>>,
    ) {
        let snapshot = host.snapshot();
        run.advance(host, &snapshot, now, rng, ownership, catalog);
    }

    #[test]
    fn a_sequence_runs_in_order() {
        let macro_ = macro_with(vec![
            Action::Log {
                message: "one".into(),
            },
            Action::Wait { duration_ms: 100 },
            Action::Log {
                message: "two".into(),
            },
        ]);
        let mut run = run(&macro_);
        let mut host = FakeHost::default();
        let mut rng = Rng::new(1);
        let mut ownership = Ownership::default();
        let catalog = catalog(&macro_);

        tick(&mut run, &mut host, 0, &mut rng, &mut ownership, &catalog);
        assert_eq!(host.logs, vec!["Test: one"]);
        assert!(run.waiting());
        tick(&mut run, &mut host, 60, &mut rng, &mut ownership, &catalog);
        assert_eq!(host.logs.len(), 1, "the wait is not over yet");
        tick(&mut run, &mut host, 100, &mut rng, &mut ownership, &catalog);
        assert_eq!(host.logs, vec!["Test: one", "Test: two"]);
        assert!(run.finished);
    }

    #[test]
    fn random_waits_stay_within_their_bounds() {
        let macro_ = macro_with(vec![Action::WaitRandom {
            min_ms: 1_000,
            max_ms: 5_000,
        }]);
        let mut run = run(&macro_);
        let mut host = FakeHost::default();
        let mut rng = Rng::new(42);
        let mut ownership = Ownership::default();
        let catalog = catalog(&macro_);
        tick(&mut run, &mut host, 50, &mut rng, &mut ownership, &catalog);
        assert!(run.waiting());
         
        for now in (100..=5_100).step_by(50) {
            if run.finished {
                break;
            }
            tick(&mut run, &mut host, now, &mut rng, &mut ownership, &catalog);
            assert!(now <= 5_050, "wait exceeded its maximum");
        }
        assert!(run.finished);
    }

    #[test]
    fn repeat_bodies_are_counted_and_bounded() {
        let macro_ = macro_with(vec![Action::Repeat {
            times: 3,
            actions: vec![Action::Attack],
        }]);
        let mut run = run(&macro_);
        let mut host = FakeHost::default();
        let mut rng = Rng::new(1);
        let mut ownership = Ownership::default();
        let catalog = catalog(&macro_);
        tick(&mut run, &mut host, 0, &mut rng, &mut ownership, &catalog);
        assert_eq!(host.attacks, 3);
        assert!(run.finished);
    }

    #[test]
    fn repeat_while_stops_when_the_condition_fails() {
        let mut macro_ = macro_with(vec![Action::RepeatWhile {
            condition: Condition::compare(
                infered_macros::ValueRef::state("health"),
                infered_macros::CompareOp::Gt,
                infered_macros::ValueRef::literal(serde_json::json!(15)),
            ),
            max_iterations: 10,
            actions: vec![Action::Attack],
        }]);
        macro_.limits.max_iterations = 4;
        let mut run = run(&macro_);
        let mut host = FakeHost::default();
        host.snapshot.health = 20.0;
        let mut rng = Rng::new(1);
        let mut ownership = Ownership::default();
        let catalog = catalog(&macro_);
         
        for now in 0..20 {
            if run.finished {
                break;
            }
            tick(
                &mut run,
                &mut host,
                now * 50,
                &mut rng,
                &mut ownership,
                &catalog,
            );
        }
        assert!(run.finished);
        assert!(run.error.unwrap().contains("iteration limit"));
        assert!(host.attacks <= 5, "attacked {} times", host.attacks);
    }

    #[test]
    fn cancellation_releases_held_inputs() {
        let macro_ = macro_with(vec![Action::Movement {
            key: MovementKey::Forward,
            action: KeyAction::Press,
            hold_ms: 0,
        }]);
        let mut run = run(&macro_);
        let mut host = FakeHost::default();
        let mut ownership = Ownership::default();
        let mut rng = Rng::new(1);
        let catalog = catalog(&macro_);
        tick(&mut run, &mut host, 0, &mut rng, &mut ownership, &catalog);
        assert_eq!(host.held, vec![(MovementKey::Forward, true)]);
        assert!(ownership
            .owner(Held::Movement(MovementKey::Forward))
            .is_some());

        run.release_inputs(&mut host, &mut ownership);
        assert_eq!(host.held.last(), Some(&(MovementKey::Forward, false)));
        assert!(ownership.held().is_empty());
    }

    #[test]
    fn a_second_macro_cannot_hold_the_same_key() {
        let macro_ = macro_with(vec![Action::Movement {
            key: MovementKey::Forward,
            action: KeyAction::Press,
            hold_ms: 0,
        }]);
        let mut first = run(&macro_);
        let mut second = run(&macro_);
        second.id = "other".into();
        let mut host = FakeHost::default();
        let mut ownership = Ownership::default();
        let mut rng = Rng::new(1);
        let catalog = catalog(&macro_);
        tick(&mut first, &mut host, 0, &mut rng, &mut ownership, &catalog);
        tick(
            &mut second,
            &mut host,
            50,
            &mut rng,
            &mut ownership,
            &catalog,
        );
        assert!(second.finished);
        assert!(second.error.unwrap().contains("already held"));
    }

    #[test]
    fn if_branches_follow_the_condition() {
        let condition = |health: f64| {
            Condition::compare(
                infered_macros::ValueRef::state("health"),
                infered_macros::CompareOp::Lt,
                infered_macros::ValueRef::literal(serde_json::json!(health)),
            )
        };
        let macro_ = macro_with(vec![Action::If {
            condition: condition(10.0),
            then: vec![Action::Chat {
                message: "low".into(),
            }],
            otherwise: vec![Action::Chat {
                message: "fine".into(),
            }],
        }]);
        let mut run = run(&macro_);
        let mut host = FakeHost::default();
        host.snapshot.health = 5.0;
        let mut rng = Rng::new(1);
        let mut ownership = Ownership::default();
        let catalog = catalog(&macro_);
        tick(&mut run, &mut host, 0, &mut rng, &mut ownership, &catalog);
        assert_eq!(host.chats, vec!["low"]);
    }

    #[test]
    fn wait_until_times_out_under_the_error_policy() {
        let mut macro_ = macro_with(vec![Action::WaitUntil {
            condition: Condition::compare(
                infered_macros::ValueRef::state("food"),
                infered_macros::CompareOp::Lt,
                infered_macros::ValueRef::literal(serde_json::json!(1)),
            ),
            timeout_ms: 100,
        }]);
        macro_.limits.on_error = ErrorPolicy::Stop;
        let mut run = run(&macro_);
        let mut host = FakeHost::default();
        host.snapshot.food = 20.0;
        let mut rng = Rng::new(1);
        let mut ownership = Ownership::default();
        let catalog = catalog(&macro_);
        tick(&mut run, &mut host, 0, &mut rng, &mut ownership, &catalog);
        assert!(run.waiting());
        tick(&mut run, &mut host, 150, &mut rng, &mut ownership, &catalog);
        assert!(run.finished);
        assert!(run.error.unwrap().contains("timed out"));
    }

    #[test]
    fn variables_and_run_macro_share_values() {
        let child = macro_with(vec![Action::SetVar {
            name: "answer".into(),
            value: serde_json::json!(42),
        }]);
        let mut parent = macro_with(vec![
            Action::RunMacro { id: "test".into() },
            Action::Log {
                message: "done".into(),
            },
        ]);
        parent.id = "parent".into();
        let mut run = run(&parent);
        let mut host = FakeHost::default();
        let mut rng = Rng::new(1);
        let mut ownership = Ownership::default();
        let catalog = BTreeMap::from([("test".into(), std::sync::Arc::new(child))]);
        tick(&mut run, &mut host, 0, &mut rng, &mut ownership, &catalog);
        assert_eq!(run.variables().get("answer"), Some(&serde_json::json!(42)));
        assert!(run.finished);
    }

    #[test]
    fn a_run_macro_cycle_is_refused() {
        let mut macro_ = macro_with(vec![Action::RunMacro { id: "test".into() }]);
        macro_.id = "test".into();
        let mut run = run(&macro_);
        let mut host = FakeHost::default();
        let mut rng = Rng::new(1);
        let mut ownership = Ownership::default();
        let catalog = BTreeMap::from([("test".into(), std::sync::Arc::new(macro_.clone()))]);
        tick(&mut run, &mut host, 0, &mut rng, &mut ownership, &catalog);
        assert!(run.finished);
        assert!(run.error.unwrap().contains("cycle"));
    }

    #[test]
    fn tapping_an_unmapped_key_is_still_delivered() {
        let macro_ = macro_with(vec![Action::Key {
            key: "F8".into(),
            action: KeyAction::Tap,
            hold_ms: 0,
        }]);
        let mut run = run(&macro_);
        let mut host = FakeHost::default();
        let mut rng = Rng::new(1);
        let mut ownership = Ownership::default();
        let catalog = catalog(&macro_);
        tick(&mut run, &mut host, 0, &mut rng, &mut ownership, &catalog);
        assert_eq!(host.taps, vec!["F8"]);
    }
}
