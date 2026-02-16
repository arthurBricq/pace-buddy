pub mod error;
pub mod hydrate;
pub mod intensity;
pub mod preprocess;
pub mod reps;
pub mod segment;
pub mod stats;
pub mod types;

#[cfg(test)]
mod tests;

use domain::ActivityStream;
use error::IntervalError;
use types::{IntervalConfig, IntervalResult};

/// Run the full interval parsing pipeline on raw activity streams.
///
/// - `streams`: raw ActivityStream objects from the database
/// - `config`: tuning parameters (use `IntervalConfig::default()` for sensible defaults)
/// - `mas_kmh`: optional Maximum Aerobic Speed in km/h (for %MAS computation)
///
/// Returns `IntervalResult` with segments, reps, and scoring.
pub fn parse_intervals(
    streams: &[ActivityStream],
    config: &IntervalConfig,
    mas_kmh: Option<f64>,
) -> Result<IntervalResult, IntervalError> {
    // 1. Hydrate: parse JSON streams into typed arrays
    let hydrated = hydrate::hydrate(streams)?;

    // 2. Preprocess: smooth + pause detection
    let preprocessed = preprocess::preprocess(&hydrated, config);

    // 3. Segment: k-means threshold + hysteresis labeling + cleanup
    let segmentation = segment::segment(&preprocessed, config);

    // 4. Build reps: warmup/cooldown, pair work+recovery, quality metrics
    let mut segments = segmentation.segments;
    let mas_mps = mas_kmh.map(|v| v / 3.6);
    let mut reps_list = reps::build_reps(&mut segments, &preprocessed, config, mas_mps);

    // 5. Intensity: compute %MAS for each rep
    intensity::compute_intensity(&mut reps_list, mas_kmh);

    // 6. Scoring
    let work_count = reps_list.len();
    let is_interval_workout = work_count >= config.min_work_segments;
    let interval_score = compute_interval_score(&reps_list, config);

    Ok(IntervalResult {
        segments,
        reps: reps_list,
        is_interval_workout,
        interval_score,
        threshold_speed_mps: segmentation.threshold_speed_mps,
        cluster_low_mps: segmentation.cluster_low_mps,
        cluster_high_mps: segmentation.cluster_high_mps,
    })
}

/// Compute a simple interval quality score in [0, 1].
fn compute_interval_score(reps: &[types::Rep], config: &IntervalConfig) -> f64 {
    if reps.len() < config.min_work_segments {
        return 0.0;
    }

    let mut score = 0.0;

    // Factor 1: number of reps (more = more likely intervals)
    let rep_score = (reps.len() as f64 / 10.0).min(1.0);
    score += rep_score * 0.3;

    // Factor 2: fraction of work segments followed by recovery
    let with_recovery = reps.iter().filter(|r| r.recovery_duration_s.is_some()).count();
    let alternation = with_recovery as f64 / reps.len() as f64;
    score += alternation * 0.3;

    // Factor 3: consistency of work speeds (low CV = better)
    let work_speeds: Vec<f64> = reps.iter().map(|r| r.avg_speed_mps).collect();
    let speed_cv = stats::cv(&work_speeds);
    let consistency = (1.0 - speed_cv).max(0.0).min(1.0);
    score += consistency * 0.4;

    score.min(1.0)
}

