 
 
 
 
 

use std::collections::BTreeMap;

use infered_macros::{CompareOp, Condition, ValueRef};
use serde_json::Value;

use super::host::Snapshot;

 
pub fn value(reference: &ValueRef, vars: &BTreeMap<String, Value>, snapshot: &Snapshot) -> Value {
    match reference {
        ValueRef::Literal { value } => value.clone(),
        ValueRef::Var { name } => vars.get(name).cloned().unwrap_or(Value::Null),
        ValueRef::State { path } => snapshot.value(path),
    }
}

 
 
pub fn evaluate(
    condition: &Condition,
    vars: &BTreeMap<String, Value>,
    snapshot: &Snapshot,
) -> bool {
    match condition {
        Condition::All { conditions } => conditions
            .iter()
            .all(|condition| evaluate(condition, vars, snapshot)),
        Condition::Any { conditions } => conditions
            .iter()
            .any(|condition| evaluate(condition, vars, snapshot)),
        Condition::Not { condition } => !evaluate(condition, vars, snapshot),
        Condition::Compare { left, op, right } => {
            let left = value(left, vars, snapshot);
            let right = value(right, vars, snapshot);
            compare(&left, *op, &right)
        }
    }
}

 
pub fn all_hold(
    conditions: &[Condition],
    vars: &BTreeMap<String, Value>,
    snapshot: &Snapshot,
) -> bool {
    conditions
        .iter()
        .all(|condition| evaluate(condition, vars, snapshot))
}

 
pub fn compare(left: &Value, op: CompareOp, right: &Value) -> bool {
    match op {
        CompareOp::Eq => equal(left, right),
        CompareOp::Ne => !equal(left, right),
        CompareOp::Lt => numeric_pair(left, right).is_some_and(|(a, b)| a < b),
        CompareOp::Le => numeric_pair(left, right).is_some_and(|(a, b)| a <= b),
        CompareOp::Gt => numeric_pair(left, right).is_some_and(|(a, b)| a > b),
        CompareOp::Ge => numeric_pair(left, right).is_some_and(|(a, b)| a >= b),
        CompareOp::Contains => contains(left, right),
    }
}

fn numeric_pair(left: &Value, right: &Value) -> Option<(f64, f64)> {
    Some((as_number(left)?, as_number(right)?))
}

fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
         
         
        _ => None,
    }
}

 
 
fn equal(left: &Value, right: &Value) -> bool {
    if let Some((a, b)) = numeric_pair(left, right) {
        return (a - b).abs() < 1e-6;
    }
    match (left, right) {
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        (Value::Array(a), Value::Array(b)) => a == b,
        (Value::Object(a), Value::Object(b)) => a == b,
        _ => false,
    }
}

fn contains(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::String(haystack), Value::String(needle)) => haystack.contains(needle),
        (Value::Array(items), needle) => items.iter().any(|item| equal(item, needle)),
        (Value::Object(map), Value::String(key)) => map.contains_key(key),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    fn snapshot() -> Snapshot {
        Snapshot {
            available: true,
            in_world: true,
            health: 8.0,
            max_health: 20.0,
            food: 12.0,
            armor: 5.0,
            level: 3,
            slot: 1,
            in_water: false,
            on_ground: true,
            moving: true,
            screen: None,
            x: 10.0,
            y: 64.0,
            z: -4.0,
            effects: Arc::new(vec![("speed".into(), 0)]),
            item_counts: Arc::new(BTreeMap::from([("minecraft:golden_apple".into(), 3)])),
            players: Arc::new(vec![crate::macros::host::PlayerSample {
                name: "Alex".into(),
                distance: 4.0,
            }]),
            ..Snapshot::default()
        }
    }

    fn state_condition(path: &str, op: CompareOp, right: Value) -> Condition {
        Condition::compare(ValueRef::state(path), op, ValueRef::literal(right))
    }

    #[test]
    fn health_slot_and_effects_compare_naturally() {
        let snapshot = snapshot();
        let vars = BTreeMap::new();
        assert!(evaluate(
            &state_condition("health", CompareOp::Lt, json!(10)),
            &vars,
            &snapshot
        ));
        assert!(!evaluate(
            &state_condition("health", CompareOp::Gt, json!(10)),
            &vars,
            &snapshot
        ));
        assert!(evaluate(
            &state_condition("slot", CompareOp::Eq, json!(2)),
            &vars,
            &snapshot
        ));
        assert!(evaluate(
            &state_condition("effect:speed", CompareOp::Ge, json!(1)),
            &vars,
            &snapshot
        ));
        assert!(evaluate(
            &state_condition(
                "has_item:minecraft:golden_apple",
                CompareOp::Eq,
                json!(true)
            ),
            &vars,
            &snapshot
        ));
        assert!(evaluate(
            &state_condition("nearby_players", CompareOp::Contains, json!("Alex")),
            &vars,
            &snapshot
        ));
    }

    #[test]
    fn all_any_and_not_nest() {
        let snapshot = snapshot();
        let vars = BTreeMap::new();
        let hurt = state_condition("health", CompareOp::Lt, json!(10));
        let fed = state_condition("food", CompareOp::Gt, json!(15));
        assert!(evaluate(
            &Condition::All {
                conditions: vec![hurt.clone()]
            },
            &vars,
            &snapshot
        ));
        assert!(evaluate(
            &Condition::Any {
                conditions: vec![fed.clone(), hurt.clone()]
            },
            &vars,
            &snapshot
        ));
        assert!(evaluate(
            &Condition::Not {
                condition: Box::new(fed)
            },
            &vars,
            &snapshot
        ));
        assert!(!evaluate(
            &Condition::All {
                conditions: vec![hurt, state_condition("food", CompareOp::Gt, json!(15))]
            },
            &vars,
            &snapshot
        ));
    }

    #[test]
    fn variables_and_event_values_resolve() {
        let snapshot = snapshot();
        let vars = BTreeMap::from([
            ("threshold".to_string(), json!(9)),
            ("event.player".to_string(), json!("Alex")),
        ]);
        assert!(evaluate(
            &Condition::compare(
                ValueRef::state("health"),
                CompareOp::Lt,
                ValueRef::Var {
                    name: "threshold".into()
                }
            ),
            &vars,
            &snapshot
        ));
        assert!(evaluate(
            &Condition::compare(
                ValueRef::Var {
                    name: "event.player".into()
                },
                CompareOp::Eq,
                ValueRef::literal(json!("Alex"))
            ),
            &vars,
            &snapshot
        ));
    }

    #[test]
    fn missing_values_never_match_ordering_operators() {
        let snapshot = Snapshot::default();
        let vars = BTreeMap::new();
        assert!(!evaluate(
            &state_condition("health", CompareOp::Lt, json!(10)),
            &vars,
            &snapshot
        ));
        assert!(!evaluate(
            &state_condition("effect:speed", CompareOp::Gt, json!(0)),
            &vars,
            &snapshot
        ));
         
        assert!(evaluate(
            &state_condition("effect:speed", CompareOp::Eq, Value::Null),
            &vars,
            &snapshot
        ));
    }

    #[test]
    fn text_contains_is_a_substring_test() {
        let vars = BTreeMap::new();
        let snapshot = snapshot();
        assert!(evaluate(
            &state_condition("screen", CompareOp::Contains, json!("none")),
            &vars,
            &snapshot
        ));
        assert!(compare(
            &json!("hello world"),
            CompareOp::Contains,
            &json!("world")
        ));
        assert!(!compare(&json!(10), CompareOp::Contains, &json!(1)));
    }

    #[test]
    fn float_noise_does_not_break_equality() {
        assert!(compare(
            &json!(20.0_f32 as f64),
            CompareOp::Eq,
            &json!(20.0)
        ));
        assert!(compare(
            &json!(0.1_f32 as f64),
            CompareOp::Lt,
            &json!(0.1001)
        ));
    }
}
