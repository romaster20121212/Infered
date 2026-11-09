 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::{FieldType, MappedObject};
use jni::objects::{GlobalRef, JValue};

 
 
pub const TICK_BUDGET_NANOS: f64 = 7_000_000.0;

 
 
pub const SERVER_MAX_CHUNKS_PER_TICK: f64 = 64.0;

 
 
pub const MIN_CHUNKS_PER_TICK: f64 = 0.1;

 
 
pub fn nanos_for_chunks_per_tick(chunks_per_tick: f64) -> f64 {
    TICK_BUDGET_NANOS / chunks_per_tick
}

 
 
pub fn target_chunks_per_tick(baseline: f64, multiplier: f64) -> f64 {
    (baseline * multiplier).clamp(MIN_CHUNKS_PER_TICK, SERVER_MAX_CHUNKS_PER_TICK)
}

 
 
 
 
 
 
 
 
pub fn boosted_nanos_per_chunk(
    current_nanos: f64,
    baseline_chunks_per_tick: f64,
    multiplier: f64,
) -> Option<f64> {
    if !baseline_chunks_per_tick.is_finite() || baseline_chunks_per_tick <= 0.0 {
        return None;
    }
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return None;
    }
    let wanted =
        nanos_for_chunks_per_tick(target_chunks_per_tick(baseline_chunks_per_tick, multiplier));
    if !wanted.is_finite() || wanted <= 0.0 {
        return None;
    }
    (current_nanos > wanted).then_some(wanted)
}

 
 
#[derive(Debug, MappedObject)]
#[mapped(class = ChunkBatchSizeCalculator)]
pub struct ChunkBatchSizeCalculator {
    jni_ref: GlobalRef,
}

impl ChunkBatchSizeCalculator {
     
    pub fn new(jni_ref: GlobalRef) -> ChunkBatchSizeCalculator {
        ChunkBatchSizeCalculator { jni_ref }
    }

     
    pub fn desired_chunks_per_tick(&self) -> anyhow::Result<f64> {
        self.in_frame(|| Ok(self.call_method("getDesiredChunksPerTick", &[])?.f()? as f64))
    }

     
     
    pub fn aggregated_nanos_per_chunk(&self) -> anyhow::Result<f64> {
        self.in_frame(|| {
            Ok(self
                .get_field("aggregatedNanosPerChunk", FieldType::Double)?
                .d()?)
        })
    }

     
     
    pub fn force_nanos_per_chunk(&self, nanos: f64) -> anyhow::Result<()> {
        self.in_frame(|| {
            self.set_field(
                "aggregatedNanosPerChunk",
                FieldType::Double,
                JValue::Double(nanos),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_round_trips_through_the_nanosecond_figure() {
         
         
         
        assert!((nanos_for_chunks_per_tick(3.5) - 2_000_000.0).abs() < 1.0);
        assert!((nanos_for_chunks_per_tick(7.0) - 1_000_000.0).abs() < 1.0);
    }

    #[test]
    fn target_is_clamped_to_the_server_ceiling() {
         
        assert!((target_chunks_per_tick(3.5, 10.0) - 35.0).abs() < 1e-9);
         
         
        assert_eq!(
            target_chunks_per_tick(9.0, 10.0),
            SERVER_MAX_CHUNKS_PER_TICK
        );
         
        assert!((target_chunks_per_tick(3.5, 1.0) - 3.5).abs() < 1e-9);
    }

    #[test]
    fn boost_raises_a_pessimistic_estimate() {
        let current = nanos_for_chunks_per_tick(3.5);  
        let wanted = boosted_nanos_per_chunk(current, 3.5, 10.0).expect("should boost");
        assert!(wanted < current, "nanoseconds must fall to raise the rate");
        assert!((wanted - nanos_for_chunks_per_tick(35.0)).abs() < 1.0);
    }

    #[test]
    fn boost_never_drags_a_faster_estimate_back_down() {
         
         
        let fast = nanos_for_chunks_per_tick(50.0);
        assert_eq!(boosted_nanos_per_chunk(fast, 3.5, 10.0), None);
    }

    #[test]
    fn boost_is_a_no_op_at_1x() {
        let current = nanos_for_chunks_per_tick(3.5);
        assert_eq!(boosted_nanos_per_chunk(current, 3.5, 1.0), None);
    }

    #[test]
    fn boost_ignores_degenerate_readings() {
        let current = nanos_for_chunks_per_tick(3.5);
        assert_eq!(boosted_nanos_per_chunk(current, 0.0, 10.0), None);
        assert_eq!(boosted_nanos_per_chunk(current, -1.0, 10.0), None);
        assert_eq!(boosted_nanos_per_chunk(current, f64::NAN, 10.0), None);
        assert_eq!(boosted_nanos_per_chunk(current, 3.5, 0.0), None);
        assert_eq!(boosted_nanos_per_chunk(current, 3.5, f64::INFINITY), None);
    }

    #[test]
    fn a_degenerate_current_reading_is_left_alone() {
         
         
         
        assert_eq!(boosted_nanos_per_chunk(0.0, 3.5, 10.0), None);
    }
}
