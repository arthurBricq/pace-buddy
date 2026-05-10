use domain::ActivityStream;

use crate::algorithms::{compute_interval_score, INTERVAL_WORKOUT_THRESHOLD};
use crate::error::IntervalError;
use crate::types::{IntervalConfig, IntervalResult};
use crate::{hydrate, intensity, preprocess, reps, segment, IntervalParsingAlgorithm};

/// Default interval parser based on fully automatic speed segmentation.
#[derive(Debug, Default, Clone, Copy)]
pub struct AutoSpeedSegmentationAlgorithm;

impl IntervalParsingAlgorithm for AutoSpeedSegmentationAlgorithm {
    fn parse(
        &self,
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
        let mas_speed = mas_kmh.map(|v| v / 3.6);
        let mut reps_list = reps::build_reps(&mut segments, &preprocessed, config, mas_speed);

        // 5. Intensity: compute %MAS for each rep
        intensity::compute_intensity(&mut reps_list, mas_kmh);

        // TODO: scoring logic could be factorized into the trait, since it is the same between both
        // `IntervalParsingAlgorithm`
        // 6. Scoring
        let interval_score = compute_interval_score(
            &reps_list,
            &segments,
            segmentation.cluster_low_mps,
            segmentation.cluster_high_mps,
            config,
        );
        // is_interval_workout is the high-confidence gate: it requires both
        // structural minimum (rep count) and the score above threshold.
        let is_interval_workout = reps_list.len() >= config.min_work_segments
            && interval_score >= INTERVAL_WORKOUT_THRESHOLD;

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
}
