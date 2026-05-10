mod auto_speed;
mod manual_lap;

pub use auto_speed::AutoSpeedSegmentationAlgorithm;
pub use manual_lap::ManualLapIntervalAlgorithm;

use crate::types::{IntervalConfig, Rep, Segment};

/// Compute interval-quality score in `[0, 1]`. The score is intended as a
/// confidence that the activity is an interval workout: it should be high for
/// real intervals and low for races, easy runs, and any other steady-state
/// effort.
///
/// Calibration on the labeled fixture corpus (see `intervals::fixtures` and
/// the `cli calibrate` subcommand) shows that the dominant signals are:
///
/// 1. Speed gap between the work and recovery clusters (interval workouts
///    have a large gap; races and easy runs have a small gap).
/// 2. Overall speed variability across the activity (intervals alternate
///    fast/slow; races and easy runs are steady).
/// 3. Rep count (must be at least `min_work_segments`).
/// 4. Recovery slowness (real recovery is jog/walk pace; race "recoveries"
///    are still close to race pace).
///
/// The earlier scoring (rep count + alternation + per-rep speed CV) confused
/// races with intervals because races have very consistent rep speeds. The
/// new score deliberately rewards activity-wide variance, not per-rep
/// uniformity.
///
/// Threshold convention: `score >= 0.55` is the "looks like intervals" gate
/// used elsewhere in the codebase. See `coach-suggested-sessions-plan.md`
/// Phase 0.
pub(crate) fn compute_interval_score(
    reps: &[Rep],
    segments: &[Segment],
    cluster_low_mps: f64,
    cluster_high_mps: f64,
    config: &IntervalConfig,
) -> f64 {
    if reps.len() < config.min_work_segments {
        return 0.0;
    }

    // 1. Cluster gap (saturate at 1.5 mps ≈ 5.4 km/h)
    let gap_mps = (cluster_high_mps - cluster_low_mps).max(0.0);
    let gap_term = (gap_mps / 1.5).clamp(0.0, 1.0);

    // 2. Overall speed CV — duration-weighted across all segments.
    let total_dur: f64 = segments.iter().map(|s| s.duration_s).sum();
    let cv_term = if total_dur > 60.0 {
        let weighted_mean: f64 = segments
            .iter()
            .map(|s| s.avg_speed_mps * s.duration_s)
            .sum::<f64>()
            / total_dur;
        if weighted_mean > 0.01 {
            let var: f64 = segments
                .iter()
                .map(|s| {
                    let d = s.avg_speed_mps - weighted_mean;
                    d * d * s.duration_s
                })
                .sum::<f64>()
                / total_dur;
            (var.sqrt() / weighted_mean / 0.4).clamp(0.0, 1.0)
        } else {
            0.0
        }
    } else {
        0.0
    };

    // 3. Rep count: 3 reps -> 0.2, 7+ reps -> 1.0.
    let rep_term = ((reps.len() as f64 - 2.0) / 5.0).clamp(0.0, 1.0);

    // 4. Recovery is genuinely slow: low cluster <= 10 km/h -> 1, >= 13 -> 0.
    let low_kmh = cluster_low_mps * 3.6;
    let recovery_term = ((13.0 - low_kmh) / 3.0).clamp(0.0, 1.0);

    0.35 * gap_term + 0.30 * cv_term + 0.15 * rep_term + 0.20 * recovery_term
}
