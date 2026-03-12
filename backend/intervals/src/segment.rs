use crate::preprocess::PreprocessedData;
use crate::stats;
use crate::types::{IntervalConfig, Segment, SegmentKind};

/// Result of the segmentation phase.
#[derive(Debug, Clone)]
pub struct SegmentationResult {
    pub segments: Vec<Segment>,
    pub threshold_speed_mps: f64,
    pub cluster_low_mps: f64,
    pub cluster_high_mps: f64,
}

/// Label assigned to each sample before converting to segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Label {
    Work,
    Recovery,
    Pause,
}

/// Segment the preprocessed data into work/recovery/pause regions.
pub fn segment(data: &PreprocessedData, config: &IntervalConfig) -> SegmentationResult {
    let n = data.time.len();

    // Collect non-pause speed samples for clustering
    let active_speeds: Vec<f64> = (0..n)
        .filter(|&i| !data.pause_mask[i])
        .map(|i| data.speed_smooth[i])
        .collect();

    if active_speeds.len() < 2 {
        return SegmentationResult {
            segments: vec![],
            threshold_speed_mps: 0.0,
            cluster_low_mps: 0.0,
            cluster_high_mps: 0.0,
        };
    }

    // K-means k=2 to find work/recovery threshold
    let (cluster_low, cluster_high, boundary) = stats::kmeans_k2(&active_speeds, 50);

    // Hysteresis labeling
    let labels = hysteresis_label(data, boundary, config);

    // Convert labels to segments
    let mut segments = labels_to_segments(&labels, data);

    // Cleanup short segments
    cleanup_segments(&mut segments, config);

    // Filter work segments whose avg speed is barely above the boundary.
    // Real interval work should be well into the high cluster, not just marginally above threshold.
    let min_work_speed = boundary + (cluster_high - boundary) * 0.35;
    for seg in segments.iter_mut() {
        if seg.kind == SegmentKind::Work && seg.avg_speed_mps < min_work_speed {
            seg.kind = SegmentKind::Recovery;
        }
    }
    merge_consecutive(&mut segments);

    SegmentationResult {
        segments,
        threshold_speed_mps: boundary,
        cluster_low_mps: cluster_low,
        cluster_high_mps: cluster_high,
    }
}

/// Debounced hysteresis labeling.
///
/// Enter WORK only after speed stays above v_enter for `enter_confirm_s` consecutive seconds.
/// Exit WORK only after speed stays below v_exit for `exit_confirm_s` consecutive seconds.
/// During the confirmation countdown, keep the previous state label (Work stays Work, etc.).
fn hysteresis_label(data: &PreprocessedData, boundary: f64, config: &IntervalConfig) -> Vec<Label> {
    let n = data.time.len();
    let v_enter = boundary + config.hysteresis_delta_mps;
    let v_exit = boundary - config.hysteresis_delta_mps;

    let enter_k = (config.enter_confirm_s.max(1.0)) as usize;
    let exit_k = (config.exit_confirm_s.max(1.0)) as usize;

    let mut labels = vec![Label::Recovery; n];
    let mut in_work = false;
    let mut above_enter = 0usize;
    let mut below_exit = 0usize;

    for i in 0..n {
        if data.pause_mask[i] {
            labels[i] = Label::Pause;
            in_work = false;
            above_enter = 0;
            below_exit = 0;
            continue;
        }

        let speed = data.speed_smooth[i];

        if in_work {
            if speed <= v_exit {
                below_exit += 1;
                if below_exit >= exit_k {
                    // Confirmed exit: relabel the dip samples as Recovery
                    in_work = false;
                    for j in (i + 1 - below_exit)..=i {
                        if labels[j] != Label::Pause {
                            labels[j] = Label::Recovery;
                        }
                    }
                    below_exit = 0;
                } else {
                    // Still in confirmation period — keep as Work
                    labels[i] = Label::Work;
                }
            } else {
                below_exit = 0;
                labels[i] = Label::Work;
            }
        } else {
            if speed >= v_enter {
                above_enter += 1;
                if above_enter >= enter_k {
                    in_work = true;
                    // Relabel the confirmation samples as Work
                    for j in (i + 1 - above_enter)..=i {
                        if labels[j] != Label::Pause {
                            labels[j] = Label::Work;
                        }
                    }
                    above_enter = 0;
                } else {
                    labels[i] = Label::Recovery;
                }
            } else {
                above_enter = 0;
                labels[i] = Label::Recovery;
            }
        }
    }

    labels
}

/// Convert per-sample labels into contiguous Segment objects.
fn labels_to_segments(labels: &[Label], data: &PreprocessedData) -> Vec<Segment> {
    if labels.is_empty() {
        return vec![];
    }

    let mut segments = Vec::new();
    let mut current_label = labels[0];
    let mut start_idx = 0;

    for i in 1..labels.len() {
        if labels[i] != current_label {
            segments.push(build_segment(current_label, start_idx, i - 1, data));
            current_label = labels[i];
            start_idx = i;
        }
    }
    // Last segment
    segments.push(build_segment(
        current_label,
        start_idx,
        labels.len() - 1,
        data,
    ));

    segments
}

/// Build a Segment from a range of sample indices.
fn build_segment(
    label: Label,
    start_idx: usize,
    end_idx: usize,
    data: &PreprocessedData,
) -> Segment {
    let kind = match label {
        Label::Work => SegmentKind::Work,
        Label::Recovery => SegmentKind::Recovery,
        Label::Pause => SegmentKind::Pause,
    };

    let start_t = data.time[start_idx];
    let end_t = data.time[end_idx];
    let duration_s = end_t - start_t;
    let distance_m = data.distance[end_idx] - data.distance[start_idx];

    let speeds: Vec<f64> = (start_idx..=end_idx)
        .map(|i| data.speed_smooth[i])
        .collect();
    let avg_speed_mps = stats::mean(&speeds);
    let speed_std_mps = stats::std_dev(&speeds);
    let max_speed_mps = speeds.iter().cloned().fold(0.0_f64, f64::max);

    let avg_hr = data.heartrate.as_ref().map(|hr| {
        let slice: Vec<f64> = (start_idx..=end_idx).map(|i| hr[i]).collect();
        stats::mean(&slice)
    });

    let avg_cadence = data.cadence.as_ref().map(|cad| {
        let slice: Vec<f64> = (start_idx..=end_idx).map(|i| cad[i]).collect();
        stats::mean(&slice)
    });

    Segment {
        kind,
        start_t,
        end_t,
        duration_s,
        distance_m,
        avg_speed_mps,
        speed_std_mps,
        max_speed_mps,
        avg_hr,
        avg_cadence,
    }
}

/// Multi-pass cleanup to consolidate noisy segments.
///
/// GPS noise during standing recoveries creates alternating Pause/Recovery/Pause
/// fragments. We must absorb these short pauses first, then clean up short
/// work/recovery segments.
fn cleanup_segments(segments: &mut Vec<Segment>, config: &IntervalConfig) {
    // Pass 1: Absorb short pauses into their neighbor's kind.
    // A short pause (< pause_min_duration_s) during running is GPS noise.
    // A short pause adjacent to Recovery is part of the recovery.
    absorb_short_pauses(segments, config);
    merge_consecutive(segments);

    // Pass 2: Remove too-short work segments (GPS blips labeled as Work).
    // Use OR: a work segment must meet BOTH duration and distance minimums to survive.
    // This filters warmup speed spikes that are long enough in time but cover little distance.
    for seg in segments.iter_mut() {
        if seg.kind == SegmentKind::Work
            && (seg.duration_s < config.min_work_duration_s
                || seg.distance_m < config.min_work_distance_m)
        {
            seg.kind = SegmentKind::Recovery;
        }
    }
    merge_consecutive(segments);

    // Pass 3: Absorb short gaps (Recovery/Pause) between two Work segments.
    // This is a morphological "closing" that prevents long reps from being split
    // by brief speed dips or GPS pause glitches.
    absorb_gaps_within_work(segments, config);
    merge_consecutive(segments);

    // Pass 4: Remove too-short recovery segments by assigning them to the
    // dominant neighbor kind (look at what's before and after).
    absorb_short_recoveries(segments, config);
    merge_consecutive(segments);
}

/// Absorb short Recovery/Pause gaps between two Work segments into Work.
/// This prevents long reps from being split by brief speed dips or GPS pauses.
fn absorb_gaps_within_work(segments: &mut Vec<Segment>, config: &IntervalConfig) {
    let n = segments.len();
    for i in 0..n {
        if segments[i].kind == SegmentKind::Work {
            continue;
        }
        if segments[i].duration_s >= config.max_gap_within_work_s {
            continue;
        }
        let prev_is_work = i > 0 && segments[i - 1].kind == SegmentKind::Work;
        let next_is_work = i + 1 < n && segments[i + 1].kind == SegmentKind::Work;
        if prev_is_work && next_is_work {
            segments[i].kind = SegmentKind::Work;
        }
    }
}

/// Convert short Pause segments to the kind of their longest neighbor.
/// This collapses `Recovery, Pause(2s), Recovery, Pause(1s), Recovery` into
/// one contiguous Recovery.
fn absorb_short_pauses(segments: &mut Vec<Segment>, config: &IntervalConfig) {
    let n = segments.len();
    for i in 0..n {
        if segments[i].kind != SegmentKind::Pause {
            continue;
        }
        // Keep real pauses (long stops) — only absorb short GPS-noise pauses
        if segments[i].duration_s >= config.pause_min_duration_s * 2.0 {
            continue;
        }
        // Look at neighbors to decide what kind to assign
        let prev_kind = if i > 0 {
            Some(segments[i - 1].kind)
        } else {
            None
        };
        let next_kind = if i + 1 < n {
            Some(segments[i + 1].kind)
        } else {
            None
        };

        let new_kind = match (prev_kind, next_kind) {
            // Both neighbors same kind → adopt it
            (Some(a), Some(b)) if a == b => a,
            // One neighbor is Work → keep as Recovery (don't extend Work across pauses)
            (Some(SegmentKind::Work), _) | (_, Some(SegmentKind::Work)) => SegmentKind::Recovery,
            // Otherwise adopt whatever neighbor exists
            (Some(k), _) => k,
            (_, Some(k)) => k,
            (None, None) => SegmentKind::Recovery,
        };
        segments[i].kind = new_kind;
    }
}

/// Convert short Recovery segments to the kind of their dominant neighbor.
fn absorb_short_recoveries(segments: &mut Vec<Segment>, config: &IntervalConfig) {
    let n = segments.len();
    for i in 0..n {
        if segments[i].kind != SegmentKind::Recovery {
            continue;
        }
        if segments[i].duration_s >= config.min_recovery_duration_s {
            continue;
        }
        // Look at neighbors
        let prev_kind = if i > 0 {
            Some(segments[i - 1].kind)
        } else {
            None
        };
        let next_kind = if i + 1 < n {
            Some(segments[i + 1].kind)
        } else {
            None
        };

        let new_kind = match (prev_kind, next_kind) {
            (Some(a), Some(b)) if a == b => a,
            (Some(SegmentKind::Work), Some(SegmentKind::Work)) => SegmentKind::Work,
            // Default: keep as Recovery (don't blindly extend Work)
            _ => SegmentKind::Recovery,
        };
        segments[i].kind = new_kind;
    }
}

/// Merge consecutive segments that share the same kind.
fn merge_consecutive(segments: &mut Vec<Segment>) {
    if segments.len() < 2 {
        return;
    }

    let mut merged: Vec<Segment> = Vec::with_capacity(segments.len());
    merged.push(segments[0].clone());

    for i in 1..segments.len() {
        let last = merged.last_mut().unwrap();
        if last.kind == segments[i].kind {
            // Merge: extend the last segment
            last.end_t = segments[i].end_t;
            last.duration_s = last.end_t - last.start_t;
            last.distance_m += segments[i].distance_m;
            // Recompute speed as weighted average by duration
            let total_dur = last.duration_s;
            if total_dur > 0.0 {
                last.avg_speed_mps = last.distance_m / total_dur;
            }
            if segments[i].max_speed_mps > last.max_speed_mps {
                last.max_speed_mps = segments[i].max_speed_mps;
            }
            // HR and cadence: simple average (imprecise but sufficient)
            last.avg_hr = match (last.avg_hr, segments[i].avg_hr) {
                (Some(a), Some(b)) => Some((a + b) / 2.0),
                (a, b) => a.or(b),
            };
            last.avg_cadence = match (last.avg_cadence, segments[i].avg_cadence) {
                (Some(a), Some(b)) => Some((a + b) / 2.0),
                (a, b) => a.or(b),
            };
        } else {
            merged.push(segments[i].clone());
        }
    }

    *segments = merged;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preprocess::PreprocessedData;

    fn make_preprocessed(
        time: Vec<f64>,
        speed: Vec<f64>,
        distance: Vec<f64>,
        pause_mask: Vec<bool>,
    ) -> PreprocessedData {
        PreprocessedData {
            time,
            distance,
            speed_smooth: speed,
            pause_mask,
            heartrate: None,
            cadence: None,
        }
    }

    #[test]
    fn test_segment_bimodal() {
        // 10 samples of slow (2 mps) then 10 samples of fast (5 mps) then 10 slow
        let n = 30;
        let time: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let speed: Vec<f64> = (0..n)
            .map(|i| if (10..20).contains(&i) { 5.0 } else { 2.0 })
            .collect();
        let distance: Vec<f64> = {
            let mut d = vec![0.0];
            for i in 1..n {
                d.push(d[i - 1] + speed[i]);
            }
            d
        };
        let pause_mask = vec![false; n];

        let data = make_preprocessed(time, speed, distance, pause_mask);
        let config = IntervalConfig {
            min_work_duration_s: 5.0,
            min_work_distance_m: 30.0,
            min_recovery_duration_s: 3.0,
            ..IntervalConfig::default()
        };
        let result = segment(&data, &config);

        // Should have recovery-work-recovery pattern
        let kinds: Vec<SegmentKind> = result.segments.iter().map(|s| s.kind).collect();
        assert!(
            kinds.contains(&SegmentKind::Work),
            "Expected work segment, got: {kinds:?}"
        );
        assert!(
            kinds.contains(&SegmentKind::Recovery),
            "Expected recovery segment, got: {kinds:?}"
        );
    }

    #[test]
    fn test_segment_with_long_pause() {
        // A long pause (>= 2 * pause_min_duration_s) should survive cleanup
        let n = 30;
        let time: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let speed: Vec<f64> = (0..n)
            .map(|i| if (10..20).contains(&i) { 0.0 } else { 3.0 })
            .collect();
        let distance: Vec<f64> = {
            let mut d = vec![0.0];
            for i in 1..n {
                d.push(d[i - 1] + speed[i]);
            }
            d
        };
        let mut pause_mask = vec![false; n];
        for i in 10..20 {
            pause_mask[i] = true;
        }

        let data = make_preprocessed(time, speed, distance, pause_mask);
        let config = IntervalConfig {
            min_work_duration_s: 2.0,
            min_recovery_duration_s: 1.0,
            ..IntervalConfig::default()
        };
        let result = segment(&data, &config);

        let has_pause = result.segments.iter().any(|s| s.kind == SegmentKind::Pause);
        assert!(has_pause, "Long pause should survive cleanup");
    }

    #[test]
    fn test_short_pauses_absorbed() {
        // Short pauses between recovery segments should be absorbed
        let n = 20;
        let time: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let speed: Vec<f64> = (0..n)
            .map(|i| if (5..8).contains(&i) { 0.0 } else { 2.0 })
            .collect();
        let distance: Vec<f64> = {
            let mut d = vec![0.0];
            for i in 1..n {
                d.push(d[i - 1] + speed[i]);
            }
            d
        };
        let mut pause_mask = vec![false; n];
        for i in 5..8 {
            pause_mask[i] = true;
        }

        let data = make_preprocessed(time, speed, distance, pause_mask);
        let config = IntervalConfig {
            min_work_duration_s: 2.0,
            min_recovery_duration_s: 1.0,
            ..IntervalConfig::default()
        };
        let result = segment(&data, &config);

        // 3s pause (< 2 * 3.0 = 6s threshold) should be absorbed into Recovery
        let has_pause = result.segments.iter().any(|s| s.kind == SegmentKind::Pause);
        assert!(
            !has_pause,
            "Short pause should be absorbed, got: {:?}",
            result
                .segments
                .iter()
                .map(|s| (s.kind, s.duration_s))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_cleanup_short_work() {
        // A work segment of 8s (below min_work_duration_s=12) should be cleaned up
        let n = 50;
        let time: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let speed: Vec<f64> = (0..n)
            .map(|i| {
                if (20..28).contains(&i) {
                    5.0 // 8s of fast — shorter than default min_work_duration_s=12
                } else {
                    2.0
                }
            })
            .collect();
        let distance: Vec<f64> = {
            let mut d = vec![0.0];
            for i in 1..n {
                d.push(d[i - 1] + speed[i]);
            }
            d
        };
        let pause_mask = vec![false; n];

        let data = make_preprocessed(time, speed, distance, pause_mask);
        let config = IntervalConfig {
            smooth_window: 1, // no smoothing, so the spike is preserved
            min_work_duration_s: 12.0,
            min_work_distance_m: 50.0,
            min_recovery_duration_s: 3.0,
            ..IntervalConfig::default()
        };

        let result = segment(&data, &config);
        // 8s work segment at 5 mps = 40m distance, below 12s duration threshold
        let work_segs: Vec<&Segment> = result
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Work)
            .collect();
        assert!(
            work_segs.is_empty(),
            "Short work segments should be cleaned up, got: {work_segs:?}"
        );
    }
}
