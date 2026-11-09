use crate::model::*;

#[derive(Debug, Clone)]
pub struct Issue {
    pub path: String,
    pub message: String,
}
pub fn is_valid_variable(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}
fn issue(out: &mut Vec<Issue>, path: &str, message: &str) {
    out.push(Issue {
        path: path.into(),
        message: message.into(),
    });
}
pub fn validate(doc: &Macro) -> Vec<Issue> {
    let mut out = Vec::new();
    if doc.format_version != 1 {
        issue(&mut out, "format_version", "unsupported macro version");
    }
    if doc.id.is_empty()
        || doc.id.len() > 64
        || !doc
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        issue(&mut out, "id", "use letters, numbers, - and _");
    }
    if doc.name.trim().is_empty() || doc.name.len() > 128 {
        issue(&mut out, "name", "name must contain 1 to 128 bytes");
    }
    if doc.description.len() > 2048 {
        issue(&mut out, "description", "description is too long");
    }
    if doc.cooldown_ms > MAX_DELAY_MS {
        issue(&mut out, "cooldown_ms", "cooldown is too long");
    }
    if doc.limits.max_runtime_ms == 0 || doc.limits.max_runtime_ms > MAX_DELAY_MS {
        issue(
            &mut out,
            "limits.max_runtime_ms",
            "runtime must be between 1 and 3600000 ms",
        );
    }
    if doc.limits.max_iterations == 0 || doc.limits.max_iterations > MAX_ITERATIONS {
        issue(
            &mut out,
            "limits.max_iterations",
            "iteration limit is out of range",
        );
    }
    if doc.variables.len() > 256 {
        issue(&mut out, "variables", "too many variables");
    }
    for name in doc.variables.keys() {
        if !is_valid_variable(name) {
            issue(&mut out, "variables", "invalid variable name");
        }
    }
    if doc.triggers.len() > 128 {
        issue(&mut out, "triggers", "too many triggers");
    }
    for (i, t) in doc.triggers.iter().enumerate() {
        let path = format!("triggers[{i}]");
        match t {
            Trigger::KeyPress { key } | Trigger::KeyRelease { key }
                if crate::find_key(key).is_none() =>
            {
                issue(&mut out, &path, "unknown key")
            }
            Trigger::MousePress { button } | Trigger::MouseRelease { button }
                if !(1..=2).contains(button) =>
            {
                issue(&mut out, &path, "unsupported mouse button")
            }
            Trigger::Interval {
                every_ms,
                start_delay_ms,
            } if *every_ms == 0 || *every_ms > MAX_DELAY_MS || *start_delay_ms > MAX_DELAY_MS => {
                issue(&mut out, &path, "invalid interval")
            }
            Trigger::AfterDelay { delay_ms } if *delay_ms > MAX_DELAY_MS => {
                issue(&mut out, &path, "delay is too long")
            }
            Trigger::Condition { when, every_ms, .. } => {
                if *every_ms > MAX_DELAY_MS {
                    issue(&mut out, &path, "interval is too long");
                }
                condition(when, &path, 0, &mut out);
            }
            Trigger::PlayerEnteredRange { range, .. } | Trigger::PlayerLeftRange { range, .. }
                if !range.is_finite() || *range <= 0.0 =>
            {
                issue(&mut out, &path, "invalid range")
            }
            Trigger::PlayerDistanceChanged { by, .. } if !by.is_finite() || *by <= 0.0 => {
                issue(&mut out, &path, "invalid distance")
            }
            Trigger::HealthBelow { value } | Trigger::HungerBelow { value }
                if !value.is_finite() =>
            {
                issue(&mut out, &path, "invalid threshold")
            }
            _ => {}
        }
    }
    if let Some(key) = &doc.key {
        if crate::find_key(key).is_none() {
            issue(&mut out, "key", "unknown key");
        }
    }
    for (i, c) in doc.conditions.iter().enumerate() {
        condition(c, &format!("conditions[{i}]"), 0, &mut out);
    }
    let mut count = 0;
    actions(&doc.actions, "actions", 0, &mut count, &mut out);
    out
}
fn condition(c: &Condition, path: &str, depth: usize, out: &mut Vec<Issue>) {
    if depth > MAX_DEPTH {
        issue(out, path, "condition nesting is too deep");
        return;
    }
    match c {
        Condition::All { conditions } | Condition::Any { conditions } => {
            if conditions.len() > MAX_ACTIONS {
                issue(out, path, "too many conditions");
                return;
            }
            for c in conditions {
                condition(c, path, depth + 1, out);
            }
        }
        Condition::Not { condition: c } => condition(c, path, depth + 1, out),
        Condition::Compare { left, right, .. } => {
            for r in [left, right] {
                if let ValueRef::Var { name } = r {
                    if !is_valid_variable(name) {
                        issue(out, path, "invalid variable");
                    }
                }
                if let ValueRef::State {
                    path: StateKey::Effect(name),
                } = r
                {
                    if name.starts_with("__unknown_state__") {
                        issue(out, path, "unknown state");
                    }
                }
            }
        }
    }
}
fn actions(list: &[Action], path: &str, depth: usize, count: &mut usize, out: &mut Vec<Issue>) {
    if depth > MAX_DEPTH {
        issue(out, path, "action nesting is too deep");
        return;
    }
    *count += list.len();
    if *count > MAX_ACTIONS {
        issue(out, path, "too many actions");
        return;
    }
    for (i, action) in list.iter().enumerate() {
        let p = format!("{path}[{i}]");
        match action {
            Action::Wait { duration_ms } if *duration_ms > MAX_DELAY_MS => {
                issue(out, &p, "delay is too long")
            }
            Action::WaitRandom { min_ms, max_ms } if min_ms > max_ms || *max_ms > MAX_DELAY_MS => {
                issue(out, &p, "invalid delay range")
            }
            Action::WaitUntil {
                condition: c,
                timeout_ms,
            } => {
                if *timeout_ms == 0 || *timeout_ms > MAX_DELAY_MS {
                    issue(out, &p, "invalid timeout");
                }
                condition(c, &p, depth + 1, out);
            }
            Action::Key { key, hold_ms, .. } => {
                if crate::find_key(key).is_none() {
                    issue(out, &p, "unknown key");
                }
                if *hold_ms > MAX_DELAY_MS {
                    issue(out, &p, "hold is too long");
                }
            }
            Action::Mouse {
                button, hold_ms, ..
            } => {
                if !(1..=2).contains(button) {
                    issue(out, &p, "unsupported mouse button");
                }
                if *hold_ms > MAX_DELAY_MS {
                    issue(out, &p, "hold is too long");
                }
            }
            Action::Movement { hold_ms, .. } | Action::UseItem { hold_ms }
                if *hold_ms > MAX_DELAY_MS =>
            {
                issue(out, &p, "hold is too long")
            }
            Action::Hotbar { slot } if !(1..=9).contains(slot) => {
                issue(out, &p, "slot must be 1 through 9")
            }
            Action::SetVar { name, .. } if !is_valid_variable(name) => {
                issue(out, &p, "invalid variable")
            }
            Action::If {
                condition: c,
                then,
                otherwise,
            } => {
                condition(c, &p, depth + 1, out);
                actions(then, &p, depth + 1, count, out);
                actions(otherwise, &p, depth + 1, count, out);
            }
            Action::Repeat {
                times,
                actions: body,
            } => {
                if *times == 0 || *times > MAX_ITERATIONS {
                    issue(out, &p, "invalid repeat count");
                }
                actions(body, &p, depth + 1, count, out);
            }
            Action::RepeatWhile {
                condition: c,
                max_iterations,
                actions: body,
            }
            | Action::RepeatUntil {
                condition: c,
                max_iterations,
                actions: body,
            } => {
                if *max_iterations == 0 || *max_iterations > MAX_ITERATIONS {
                    issue(out, &p, "invalid repeat count");
                }
                condition(c, &p, depth + 1, out);
                actions(body, &p, depth + 1, count, out);
            }
            Action::RunMacro { id } if id.is_empty() => issue(out, &p, "macro id is required"),
            Action::Chat { message } | Action::Log { message } if message.len() > 2048 => {
                issue(out, &p, "message is too long")
            }
            _ => {}
        }
    }
}
