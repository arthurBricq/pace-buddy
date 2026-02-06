pub mod error;
pub mod hydrate;
pub mod intensity;
pub mod preprocess;
pub mod reps;
pub mod segment;
pub mod stats;
pub mod types;

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
    let with_recovery = reps.iter().filter(|r| r.recovery.is_some()).count();
    let alternation = with_recovery as f64 / reps.len() as f64;
    score += alternation * 0.3;

    // Factor 3: consistency of work speeds (low CV = better)
    let work_speeds: Vec<f64> = reps.iter().map(|r| r.avg_speed_mps).collect();
    let speed_cv = stats::cv(&work_speeds);
    let consistency = (1.0 - speed_cv).max(0.0).min(1.0);
    score += consistency * 0.4;

    score.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ActivityStream, StreamType};
    use uuid::Uuid;

    fn make_stream(stream_type: StreamType, data_json: &str) -> ActivityStream {
        ActivityStream {
            activity_id: Uuid::nil(),
            stream_type,
            data_json: data_json.to_string(),
        }
    }

    /// Build synthetic interval data: warmup, 5x(fast+slow), cooldown
    fn make_interval_streams() -> Vec<ActivityStream> {
        let mut time = Vec::new();
        let mut distance = Vec::new();
        let mut velocity = Vec::new();
        let mut moving = Vec::new();

        let mut t = 0.0;
        let mut d = 0.0;

        // Warmup: 7 minutes at 3.0 m/s
        for _ in 0..420 {
            time.push(t);
            distance.push(d);
            velocity.push(3.0);
            moving.push(true);
            t += 1.0;
            d += 3.0;
        }

        // 5 x (60s fast at 5.0 m/s + 90s slow at 2.5 m/s)
        for _ in 0..5 {
            // Work: 60s at 5.0 m/s
            for _ in 0..60 {
                time.push(t);
                distance.push(d);
                velocity.push(5.0);
                moving.push(true);
                t += 1.0;
                d += 5.0;
            }
            // Recovery: 90s at 2.5 m/s
            for _ in 0..90 {
                time.push(t);
                distance.push(d);
                velocity.push(2.5);
                moving.push(true);
                t += 1.0;
                d += 2.5;
            }
        }

        // Cooldown: 6 minutes at 3.0 m/s
        for _ in 0..360 {
            time.push(t);
            distance.push(d);
            velocity.push(3.0);
            moving.push(true);
            t += 1.0;
            d += 3.0;
        }

        let time_json = serde_json::to_string(&time).unwrap();
        let dist_json = serde_json::to_string(&distance).unwrap();
        let vel_json = serde_json::to_string(&velocity).unwrap();
        let mov_json = serde_json::to_string(&moving).unwrap();

        vec![
            make_stream(StreamType::Time, &time_json),
            make_stream(StreamType::Distance, &dist_json),
            make_stream(StreamType::VelocitySmooth, &vel_json),
            make_stream(StreamType::Moving, &mov_json),
        ]
    }

    #[test]
    fn test_full_pipeline_synthetic_intervals() {
        let streams = make_interval_streams();
        let config = IntervalConfig::default();
        let result = parse_intervals(&streams, &config, Some(18.0)).unwrap();

        assert!(
            result.is_interval_workout,
            "Should detect as interval workout, got {} reps",
            result.reps.len()
        );
        assert!(
            result.reps.len() >= 3,
            "Expected at least 3 reps, got {}",
            result.reps.len()
        );
        assert!(
            result.interval_score > 0.3,
            "Expected decent interval score, got {}",
            result.interval_score
        );

        // Check %MAS was computed
        for rep in &result.reps {
            assert!(rep.pct_mas.is_some());
            let pct = rep.pct_mas.unwrap();
            assert!(pct > 0.5 && pct < 1.5, "Unexpected %MAS: {pct}");
        }
    }

    #[test]
    fn test_full_pipeline_steady_run() {
        // Steady run at constant 3.5 m/s for 30 minutes - should NOT be interval workout
        let n = 1800;
        let time: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let distance: Vec<f64> = (0..n).map(|i| i as f64 * 3.5).collect();
        let velocity: Vec<f64> = vec![3.5; n];

        let streams = vec![
            make_stream(StreamType::Time, &serde_json::to_string(&time).unwrap()),
            make_stream(
                StreamType::Distance,
                &serde_json::to_string(&distance).unwrap(),
            ),
            make_stream(
                StreamType::VelocitySmooth,
                &serde_json::to_string(&velocity).unwrap(),
            ),
        ];

        let config = IntervalConfig::default();
        let result = parse_intervals(&streams, &config, None).unwrap();

        assert!(
            !result.is_interval_workout,
            "Steady run should not be detected as intervals, got {} reps",
            result.reps.len()
        );
    }
}
