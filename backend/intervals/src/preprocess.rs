use crate::hydrate::HydratedStreams;
use crate::stats::{rolling_mean, rolling_median};
use crate::types::IntervalConfig;

/// Preprocessed data ready for segmentation.
#[derive(Debug, Clone)]
pub struct PreprocessedData {
    /// Time array (seconds since activity start)
    pub time: Vec<f64>,
    /// Cumulative distance (meters)
    pub distance: Vec<f64>,
    /// Smoothed speed (mps)
    pub speed_smooth: Vec<f64>,
    /// True where the athlete is paused
    pub pause_mask: Vec<bool>,
    /// Heart rate (optional)
    pub heartrate: Option<Vec<f64>>,
    /// Cadence (optional)
    pub cadence: Option<Vec<f64>>,
}

/// Apply smoothing and pause detection to hydrated streams.
pub fn preprocess(streams: &HydratedStreams, config: &IntervalConfig) -> PreprocessedData {
    // Pause detection uses raw velocity (before smoothing) to avoid edge artifacts
    let pause_mask = detect_pauses(streams, &streams.velocity_smooth, config);

    // Two-stage smoothing: median filter removes spikes, then rolling mean smooths jitter
    let after_median = rolling_median(&streams.velocity_smooth, config.smooth_window);
    let speed_smooth = if config.smooth_mean_window > 1 {
        rolling_mean(&after_median, config.smooth_mean_window)
    } else {
        after_median
    };

    PreprocessedData {
        time: streams.time.clone(),
        distance: streams.distance.clone(),
        speed_smooth,
        pause_mask,
        heartrate: streams.heartrate.clone(),
        cadence: streams.cadence.clone(),
    }
}

/// Detect pauses using the `moving` stream as a hint (if available), else by speed threshold.
/// In both cases, apply the min-duration filter so brief Strava `moving=false` glitches
/// don't produce pause segments that split work.
fn detect_pauses(
    streams: &HydratedStreams,
    speed_smooth: &[f64],
    config: &IntervalConfig,
) -> Vec<bool> {
    // Build raw candidate pause mask: use moving flag if available, else speed threshold
    let raw_pause: Vec<bool> = if let Some(ref moving) = streams.moving {
        // Treat !moving as a candidate, but still require min duration
        moving.iter().map(|m| !m).collect()
    } else {
        speed_smooth
            .iter()
            .map(|&s| s < config.pause_speed_threshold)
            .collect()
    };

    // Only keep pause regions that last >= pause_min_duration_s
    apply_min_duration_filter(&raw_pause, &streams.time, config.pause_min_duration_s)
}

/// Keep only contiguous true-regions whose duration >= min_duration_s.
fn apply_min_duration_filter(mask: &[bool], time: &[f64], min_duration_s: f64) -> Vec<bool> {
    let n = mask.len();
    let mut result = vec![false; n];
    let mut i = 0;
    while i < n {
        if mask[i] {
            let start = i;
            while i < n && mask[i] {
                i += 1;
            }
            let end = i;
            let duration = if end > start && end <= n {
                time[end.min(n - 1)] - time[start]
            } else {
                0.0
            };
            if duration >= min_duration_s {
                for j in start..end {
                    result[j] = true;
                }
            }
        } else {
            i += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydrate::HydratedStreams;

    fn make_streams(
        time: Vec<f64>,
        velocity: Vec<f64>,
        moving: Option<Vec<bool>>,
    ) -> HydratedStreams {
        let distance: Vec<f64> = time.iter().map(|&t| t * 3.0).collect(); // fake distance
        HydratedStreams {
            time,
            distance,
            velocity_smooth: velocity,
            moving,
            heartrate: None,
            cadence: None,
            altitude: None,
        }
    }

    #[test]
    fn test_preprocess_smoothing() {
        let streams = make_streams(
            (0..10).map(|i| i as f64).collect(),
            vec![3.0, 10.0, 3.0, 10.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0],
            None,
        );
        let config = IntervalConfig::default();
        let pp = preprocess(&streams, &config);
        // Smoothed speed should be less spiky
        assert_eq!(pp.speed_smooth.len(), 10);
        // With window=5, spike at index 1 should be dampened
        assert!(pp.speed_smooth[1] < 10.0);
    }

    #[test]
    fn test_pause_detection_from_moving() {
        // Pause must last >= pause_min_duration_s (3s) even when using moving stream
        let streams = make_streams(
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            vec![3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.0],
            Some(vec![true, false, false, false, false, false, true]),
        );
        let config = IntervalConfig::default();
        let pp = preprocess(&streams, &config);
        assert_eq!(
            pp.pause_mask,
            vec![false, true, true, true, true, true, false]
        );
    }

    #[test]
    fn test_short_moving_glitch_not_pause() {
        // A 1s !moving glitch should be filtered out by min-duration
        let streams = make_streams(
            vec![0.0, 1.0, 2.0, 3.0, 4.0],
            vec![3.0, 3.0, 0.0, 3.0, 3.0],
            Some(vec![true, true, false, true, true]),
        );
        let config = IntervalConfig::default();
        let pp = preprocess(&streams, &config);
        // Only 1s of !moving → too short → not a pause
        assert_eq!(pp.pause_mask, vec![false, false, false, false, false]);
    }

    #[test]
    fn test_pause_detection_from_speed() {
        // 5 seconds of slow speed - should be detected as pause (>= 3s)
        let streams = make_streams(
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            vec![4.0, 0.1, 0.1, 0.1, 0.1, 0.1, 4.0],
            None,
        );
        let config = IntervalConfig::default();
        let pp = preprocess(&streams, &config);
        // Indices 1-5 are slow for 5 seconds, should be pause
        assert!(!pp.pause_mask[0]);
        assert!(pp.pause_mask[1]);
        assert!(pp.pause_mask[5]);
        assert!(!pp.pause_mask[6]);
    }

    #[test]
    fn test_short_pause_not_detected() {
        // 1 second of slow speed - too short to be pause
        let streams = make_streams(vec![0.0, 1.0, 2.0, 3.0], vec![4.0, 0.1, 4.0, 4.0], None);
        let config = IntervalConfig::default();
        let pp = preprocess(&streams, &config);
        // Only 1s slow → not a pause
        assert!(!pp.pause_mask[1]);
    }
}
