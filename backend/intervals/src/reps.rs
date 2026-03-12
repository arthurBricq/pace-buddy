use crate::preprocess::PreprocessedData;
use crate::stats;
use crate::types::{IntervalConfig, Rep, Segment, SegmentKind};

/// Label warmup/cooldown and build reps from work segments.
/// Recovery is computed as the time gap between consecutive work segments.
pub fn build_reps(
    segments: &mut Vec<Segment>,
    data: &PreprocessedData,
    config: &IntervalConfig,
    mas_speed: Option<f64>,
) -> Vec<Rep> {
    label_warmup_cooldown(segments, config);

    let work_segments: Vec<Segment> = segments
        .iter()
        .filter(|s| s.kind == SegmentKind::Work)
        .cloned()
        .collect();

    work_segments
        .iter()
        .enumerate()
        .map(|(i, work)| {
            let pace_std = compute_pace_std(data, work);
            let steadiness = compute_steadiness(data, work);
            let fade = compute_fade(data, work);

            let avg_speed_mps = work.avg_speed_mps;
            let avg_pace_s_per_km = if avg_speed_mps > 0.0 {
                1000.0 / avg_speed_mps
            } else {
                0.0
            };

            let pct_mas = mas_speed.map(|mas| if mas > 0.0 { avg_speed_mps / mas } else { 0.0 });

            // Recovery = time from end of this work to start of next work
            let recovery_duration_s = work_segments
                .get(i + 1)
                .map(|next| next.start_t - work.end_t);

            Rep {
                distance_m: work.distance_m,
                duration_s: work.duration_s,
                avg_pace_s_per_km,
                avg_speed_mps,
                pace_std,
                pct_mas,
                steadiness,
                fade,
                recovery_duration_s,
                work: work.clone(),
                rep_index: i,
                set_index: None,
            }
        })
        .collect()
}

/// Label the first segment(s) as Warmup and last as Cooldown if they meet duration thresholds.
///
/// "First work" means the first *substantial* work segment (meeting min duration/distance),
/// not just any brief acceleration during warmup jogging.
fn label_warmup_cooldown(segments: &mut Vec<Segment>, config: &IntervalConfig) {
    if segments.is_empty() {
        return;
    }

    // For warmup/cooldown detection, require a more substantial work segment
    // than the basic cleanup threshold. This prevents brief accelerations during
    // warmup from being mistaken for the start of the interval block.
    let substantial_dur = config.min_work_duration_s * 3.0; // 36s
    let substantial_dist = config.min_work_distance_m * 3.0; // 300m
    let is_substantial_work = |s: &Segment| {
        s.kind == SegmentKind::Work
            && s.duration_s >= substantial_dur
            && s.distance_m >= substantial_dist
    };

    let first_work = segments.iter().position(|s| is_substantial_work(s));
    let last_work = segments.iter().rposition(|s| is_substantial_work(s));

    if let Some(first) = first_work {
        // Everything before first substantial work
        let warmup_duration: f64 = segments[..first].iter().map(|s| s.duration_s).sum();
        if warmup_duration >= config.warmup_min_s {
            for seg in segments[..first].iter_mut() {
                if seg.kind == SegmentKind::Recovery || seg.kind == SegmentKind::Work {
                    seg.kind = SegmentKind::Warmup;
                }
            }
        }
    }

    if let Some(last) = last_work {
        if last + 1 < segments.len() {
            let cooldown_duration: f64 = segments[last + 1..].iter().map(|s| s.duration_s).sum();
            if cooldown_duration >= config.cooldown_min_s {
                for seg in segments[last + 1..].iter_mut() {
                    if seg.kind == SegmentKind::Recovery || seg.kind == SegmentKind::Work {
                        seg.kind = SegmentKind::Cooldown;
                    }
                }
            }
        }
    }
}

/// Compute pace standard deviation within a work segment's time range.
fn compute_pace_std(data: &PreprocessedData, work: &Segment) -> f64 {
    let speeds = get_speeds_in_range(data, work.start_t, work.end_t);
    if speeds.is_empty() {
        return 0.0;
    }
    // Convert to pace (s/km) and compute std dev
    let paces: Vec<f64> = speeds
        .iter()
        .filter(|&&s| s > 0.1)
        .map(|&s| 1000.0 / s)
        .collect();
    stats::std_dev(&paces)
}

/// Steadiness: 1.0 - CV of speed within the work segment. Higher = more even pacing.
fn compute_steadiness(data: &PreprocessedData, work: &Segment) -> f64 {
    let speeds = get_speeds_in_range(data, work.start_t, work.end_t);
    if speeds.is_empty() {
        return 0.0;
    }
    let coefficient = stats::cv(&speeds);
    (1.0 - coefficient).max(0.0).min(1.0)
}

/// Fade: ratio of second-half avg speed to first-half avg speed. >1 = positive split, <1 = negative split.
fn compute_fade(data: &PreprocessedData, work: &Segment) -> f64 {
    let speeds = get_speeds_in_range(data, work.start_t, work.end_t);
    if speeds.len() < 4 {
        return 0.0;
    }
    let mid = speeds.len() / 2;
    let first_half = stats::mean(&speeds[..mid]);
    let second_half = stats::mean(&speeds[mid..]);
    if first_half > 0.0 {
        (first_half - second_half) / first_half
    } else {
        0.0
    }
}

/// Get speed samples within a time range.
fn get_speeds_in_range(data: &PreprocessedData, start_t: f64, end_t: f64) -> Vec<f64> {
    let (start_idx, end_idx) = time_range_to_indices(&data.time, start_t, end_t);
    let n = data.speed_smooth.len();
    if start_idx >= n || end_idx <= start_idx {
        return vec![];
    }
    data.speed_smooth[start_idx..end_idx.min(n)].to_vec()
}

/// Find the index range [start, end) in the time array for a given time range.
fn time_range_to_indices(time: &[f64], start_t: f64, end_t: f64) -> (usize, usize) {
    let start = time.partition_point(|&t| t < start_t);
    let end = time.partition_point(|&t| t <= end_t);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_segments() -> Vec<Segment> {
        vec![
            Segment {
                kind: SegmentKind::Recovery,
                start_t: 0.0,
                end_t: 400.0,
                duration_s: 400.0,
                distance_m: 1200.0,
                avg_speed_mps: 3.0,
                speed_std_mps: 0.2,
                max_speed_mps: 3.5,
                avg_hr: None,
                avg_cadence: None,
            },
            Segment {
                kind: SegmentKind::Work,
                start_t: 400.0,
                end_t: 480.0,
                duration_s: 80.0,
                distance_m: 400.0,
                avg_speed_mps: 5.0,
                speed_std_mps: 0.3,
                max_speed_mps: 5.5,
                avg_hr: None,
                avg_cadence: None,
            },
            Segment {
                kind: SegmentKind::Recovery,
                start_t: 480.0,
                end_t: 570.0,
                duration_s: 90.0,
                distance_m: 270.0,
                avg_speed_mps: 3.0,
                speed_std_mps: 0.2,
                max_speed_mps: 3.5,
                avg_hr: None,
                avg_cadence: None,
            },
            Segment {
                kind: SegmentKind::Work,
                start_t: 570.0,
                end_t: 650.0,
                duration_s: 80.0,
                distance_m: 400.0,
                avg_speed_mps: 5.0,
                speed_std_mps: 0.3,
                max_speed_mps: 5.5,
                avg_hr: None,
                avg_cadence: None,
            },
            Segment {
                kind: SegmentKind::Recovery,
                start_t: 650.0,
                end_t: 1050.0,
                duration_s: 400.0,
                distance_m: 1200.0,
                avg_speed_mps: 3.0,
                speed_std_mps: 0.2,
                max_speed_mps: 3.5,
                avg_hr: None,
                avg_cadence: None,
            },
        ]
    }

    #[test]
    fn test_warmup_cooldown_labeling() {
        let mut segments = make_segments();
        let config = IntervalConfig {
            warmup_min_s: 360.0,
            cooldown_min_s: 300.0,
            ..IntervalConfig::default()
        };
        label_warmup_cooldown(&mut segments, &config);
        assert_eq!(segments[0].kind, SegmentKind::Warmup);
        assert_eq!(segments[4].kind, SegmentKind::Cooldown);
    }

    #[test]
    fn test_work_segments_collected() {
        let segments = make_segments();
        let work_count = segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Work)
            .count();
        assert_eq!(work_count, 2);
    }

    #[test]
    fn test_fade_computation() {
        // Speeds: first half fast, second half slow → positive fade
        let data = PreprocessedData {
            time: (0..10).map(|i| i as f64).collect(),
            distance: (0..10).map(|i| (i * 5) as f64).collect(),
            speed_smooth: vec![5.0, 5.0, 5.0, 5.0, 5.0, 4.0, 4.0, 4.0, 4.0, 4.0],
            pause_mask: vec![false; 10],
            heartrate: None,
            cadence: None,
        };
        let work = Segment {
            kind: SegmentKind::Work,
            start_t: 0.0,
            end_t: 9.0,
            duration_s: 9.0,
            distance_m: 45.0,
            avg_speed_mps: 4.5,
            speed_std_mps: 0.5,
            max_speed_mps: 5.0,
            avg_hr: None,
            avg_cadence: None,
        };
        let fade = compute_fade(&data, &work);
        // first half avg = 5.0, second half avg = 4.0, fade = (5-4)/5 = 0.2
        assert!((fade - 0.2).abs() < 0.05);
    }
}
