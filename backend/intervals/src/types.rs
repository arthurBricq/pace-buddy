use serde::{Deserialize, Serialize};

/// Configuration for the interval parsing pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalConfig {
    /// Rolling median window size for smoothing
    pub smooth_window: usize,
    /// Rolling mean window applied after median (0 = disabled)
    pub smooth_mean_window: usize,
    /// Speed below which we consider the athlete paused (mps)
    pub pause_speed_threshold: f64,
    /// Minimum duration to consider a low-speed region a pause (seconds)
    pub pause_min_duration_s: f64,
    /// Hysteresis delta applied above/below k-means boundary (mps)
    pub hysteresis_delta_mps: f64,
    /// Consecutive seconds speed must stay above v_enter before entering Work
    pub enter_confirm_s: f64,
    /// Consecutive seconds speed must stay below v_exit before exiting Work
    pub exit_confirm_s: f64,
    /// Minimum duration for a work segment to be kept (seconds)
    pub min_work_duration_s: f64,
    /// Minimum distance for a work segment to be kept (meters)
    pub min_work_distance_m: f64,
    /// Minimum recovery duration before merging with neighbors (seconds)
    pub min_recovery_duration_s: f64,
    /// Max gap (Recovery/Pause) between two Work segments that gets absorbed into Work (seconds)
    pub max_gap_within_work_s: f64,
    /// Minimum duration before first work segment to label as warmup (seconds)
    pub warmup_min_s: f64,
    /// Minimum duration after last work segment to label as cooldown (seconds)
    pub cooldown_min_s: f64,
    /// Minimum number of work segments to consider this an interval workout
    pub min_work_segments: usize,
}

impl Default for IntervalConfig {
    fn default() -> Self {
        Self {
            smooth_window: 5,
            smooth_mean_window: 11,
            pause_speed_threshold: 0.5,
            pause_min_duration_s: 3.0,
            hysteresis_delta_mps: 0.15,
            enter_confirm_s: 2.0,
            exit_confirm_s: 8.0,
            min_work_duration_s: 12.0,
            min_work_distance_m: 100.0,
            min_recovery_duration_s: 8.0,
            max_gap_within_work_s: 20.0,
            warmup_min_s: 360.0,
            cooldown_min_s: 300.0,
            min_work_segments: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentKind {
    Warmup,
    Work,
    Recovery,
    Cooldown,
    Pause,
    Steady,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub kind: SegmentKind,
    pub start_t: f64,
    pub end_t: f64,
    pub duration_s: f64,
    pub distance_m: f64,
    pub avg_speed_mps: f64,
    pub speed_std_mps: f64,
    pub max_speed_mps: f64,
    pub avg_hr: Option<f64>,
    pub avg_cadence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rep {
    pub work: Segment,
    pub rep_index: usize,
    pub set_index: Option<usize>,
    pub distance_m: f64,
    pub duration_s: f64,
    pub avg_pace_s_per_km: f64,
    pub avg_speed_mps: f64,
    pub pace_std: f64,
    pub pct_mas: Option<f64>,
    pub steadiness: f64,
    pub fade: f64,
    /// Time in seconds between end of this rep and start of next rep. None for the last rep.
    pub recovery_duration_s: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalResult {
    pub segments: Vec<Segment>,
    pub reps: Vec<Rep>,
    pub is_interval_workout: bool,
    pub interval_score: f64,
    pub threshold_speed_mps: f64,
    pub cluster_low_mps: f64,
    pub cluster_high_mps: f64,
}
