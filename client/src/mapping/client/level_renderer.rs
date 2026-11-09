 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::{mapping, minecraft};
use jni::objects::{JObject, JValue};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

 
 
 
 
 
const REBUILD_MIN_GAP: Duration = Duration::from_millis(1200);

 
 
 
 
const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

 
 
struct CullOverride {
     
    active: bool,
     
     
    saved: Option<bool>,
     
     
    pending_invalidate: bool,
     
     
    last_rebuild: Option<Instant>,
     
     
    warned_without_graph_hooks: bool,
}

static CULL_OVERRIDE: Mutex<CullOverride> = Mutex::new(CullOverride {
    active: false,
    saved: None,
    pending_invalidate: false,
    last_rebuild: None,
    warned_without_graph_hooks: false,
});

 
 
 
fn cull_override() -> MutexGuard<'static, CullOverride> {
    CULL_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
 
 
pub(crate) fn partial_suppressed() -> bool {
    cull_override().active
}

 
 
 
pub(crate) fn expansion_armed() -> bool {
    cull_override().active
}

 
 
fn smart_cull_value(value: bool) -> anyhow::Result<bool> {
    let previous = mapping()
        .get_field(
            MinecraftClassType::Minecraft,
            minecraft().jni_ref().as_obj(),
            "smartCull",
            FieldType::Boolean,
        )?
        .z()?;
    if previous != value {
        mapping().set_field(
            MinecraftClassType::Minecraft,
            minecraft().jni_ref().as_obj(),
            "smartCull",
            FieldType::Boolean,
            JValue::Bool(value as u8),
        )?;
    }
    Ok(previous)
}

 
 
 
fn set_needs_full_update(graph: &JObject, value: bool) -> anyhow::Result<()> {
    mapping().set_field(
        MinecraftClassType::SectionOcclusionGraph,
        graph,
        "needsFullUpdate",
        FieldType::Boolean,
        JValue::Bool(value as u8),
    )?;
    Ok(())
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
pub(crate) fn on_graph_invalidate(graph: &JObject) -> anyhow::Result<()> {
    {
        let mut state = cull_override();
        if state.active {
            state.pending_invalidate = true;
            log::debug!("Render All: queued occlusion-graph rebuild request");
            return Ok(());
        }
    }
    set_needs_full_update(graph, true)
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
pub(crate) fn engage(level_renderer: &JObject) -> anyhow::Result<()> {
    if !crate::jvmti::graph_hooks_installed() {
        let mut state = cull_override();
        if !state.warned_without_graph_hooks {
            state.warned_without_graph_hooks = true;
            log::error!(
                "Render All: the section-occlusion-graph hooks are not installed, so smartCull \
                 stays untouched and the reveal is inert. See the retransform errors above."
            );
        }
        return Ok(());
    }

    let fire_one_shot = {
        let mut state = cull_override();
        if !state.active {
            state.active = true;
            let current = smart_cull_value(false)?;
            state.saved = Some(current);
            state.pending_invalidate = true;
            state.last_rebuild = None;
            log::info!("Render All: engaged; smartCull forced off (was {current})");
        }
         
         
        let _ = smart_cull_value(false)?;

        let (gap_elapsed, refresh_due) = match state.last_rebuild.map(|at| at.elapsed()) {
            Some(elapsed) => (elapsed >= REBUILD_MIN_GAP, elapsed >= REFRESH_INTERVAL),
            None => (true, true),
        };
        let fire = gap_elapsed && (state.pending_invalidate || refresh_due);
        if fire {
            state.last_rebuild = Some(Instant::now());
             
            state.pending_invalidate = false;
        }
        fire
    };

    if !fire_one_shot {
        return Ok(());
    }

    let graph = mapping()
        .get_field(
            MinecraftClassType::LevelRenderer,
            level_renderer,
            "sectionOcclusionGraph",
            FieldType::Object(MinecraftClassType::SectionOcclusionGraph),
        )?
        .l()?;
    if graph.is_null() {
         
         
         
        cull_override().pending_invalidate = true;
        return Ok(());
    }
    if let Err(error) = set_needs_full_update(&graph, true) {
        cull_override().pending_invalidate = true;
        return Err(error);
    }
    log::debug!("Render All: occlusion graph full update requested");
    Ok(())
}

 
 
 
 
pub(crate) fn restore_smart_cull(level_renderer: &JObject) -> anyhow::Result<()> {
    let saved;
    {
        let mut state = cull_override();
        if !state.active {
            return Ok(());
        }
        state.active = false;
        state.pending_invalidate = false;
        state.last_rebuild = None;
        saved = state.saved.take().unwrap_or(true);
    }
    let _ = smart_cull_value(saved)?;
    let graph = mapping()
        .get_field(
            MinecraftClassType::LevelRenderer,
            level_renderer,
            "sectionOcclusionGraph",
            FieldType::Object(MinecraftClassType::SectionOcclusionGraph),
        )?
        .l()?;
    if !graph.is_null() {
        set_needs_full_update(&graph, true)?;
    }
    log::info!("Render All: disengaged; smartCull restored to {saved}");
    Ok(())
}

 
 
 
 
pub(crate) fn restore_smart_cull_value() -> anyhow::Result<()> {
    let saved;
    {
        let mut state = cull_override();
        if !state.active {
            return Ok(());
        }
        state.active = false;
        state.pending_invalidate = false;
        state.last_rebuild = None;
        state.warned_without_graph_hooks = false;
        saved = state.saved.take().unwrap_or(true);
    }
    let _ = smart_cull_value(saved)?;
    log::info!("Render All: smartCull restored to {saved} at teardown");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

     
     
    static TEST_LOCK: StdMutex<()> = StdMutex::new(());

     
     
     
    #[test]
    fn a_cold_override_is_inert() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _ = restore_smart_cull_value();
        assert!(!expansion_armed(), "teardown must clear the override");
        assert!(
            !partial_suppressed(),
            "a cold override must not suppress the per-frame partial"
        );
    }

     
     
     
     
     
     
     
     
    #[test]
    fn partial_is_suppressed_exactly_while_engaged() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        {
            cull_override().active = true;
        }
        assert!(expansion_armed());
        assert!(partial_suppressed());
        {
            cull_override().active = false;
        }
        assert!(!expansion_armed());
        assert!(!partial_suppressed());
    }

     
     
     
    #[test]
    fn repeated_teardown_is_a_no_op() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _ = restore_smart_cull_value();
        assert!(!expansion_armed());
        let _ = restore_smart_cull_value();
        assert!(!expansion_armed());
    }

     
     
     
    #[test]
    fn rebuild_gap_is_long_enough_to_bound_the_storm() {
        assert!(
            REBUILD_MIN_GAP >= Duration::from_millis(500),
            "a rebuild gap this short barely throttles a moving freecam"
        );
        assert!(
            REFRESH_INTERVAL > REBUILD_MIN_GAP,
            "the forced refresh must be rarer than invalidate-driven rebuilds, \
             or it makes the gap pointless"
        );
    }
}
