mod auto_speed;
mod manual_lap;

pub use auto_speed::AutoSpeedSegmentationAlgorithm;
pub use manual_lap::ManualLapIntervalAlgorithm;

use crate::stats;
use crate::types::{IntervalConfig, Rep};

/// Compute a simple interval quality score in [0, 1].
pub(crate) fn compute_interval_score(reps: &[Rep], config: &IntervalConfig) -> f64 {
    if reps.len() < config.min_work_segments {
        return 0.0;
    }

    let mut score = 0.0;

    // Factor 1: number of reps (more = more likely intervals)
    let rep_score = (reps.len() as f64 / 10.0).min(1.0);
    score += rep_score * 0.3;

    // Factor 2: fraction of work segments followed by recovery
    let with_recovery = reps
        .iter()
        .filter(|r| r.recovery_duration_s.is_some())
        .count();
    let alternation = with_recovery as f64 / reps.len() as f64;
    score += alternation * 0.3;

    // Factor 3: consistency of work speeds (low CV = better)
    let work_speeds: Vec<f64> = reps.iter().map(|r| r.avg_speed_mps).collect();
    let speed_cv = stats::cv(&work_speeds);
    let consistency = (1.0 - speed_cv).max(0.0).min(1.0);
    score += consistency * 0.4;

    score.min(1.0)
}
