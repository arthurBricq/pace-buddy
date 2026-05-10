use domain::{ActivityLap, ActivityStream};

use crate::algorithms::compute_interval_score;
use crate::error::IntervalError;
use crate::types::{IntervalConfig, IntervalResult, Rep, Segment, SegmentKind};
use crate::{intensity, stats, IntervalParsingAlgorithm};

/// Interval parser based on manually created laps.
///
/// This parser treats each lap as one segment and classifies laps into
/// `Work` vs `Recovery` using a simple speed split.
#[derive(Debug, Clone, Copy)]
pub struct ManualLapIntervalAlgorithm<'a> {
    laps: &'a [ActivityLap],
}

impl<'a> ManualLapIntervalAlgorithm<'a> {
    pub fn new(laps: &'a [ActivityLap]) -> Self {
        Self { laps }
    }
}

impl IntervalParsingAlgorithm for ManualLapIntervalAlgorithm<'_> {
    fn parse(
        &self,
        _streams: &[ActivityStream],
        config: &IntervalConfig,
        mas_kmh: Option<f64>,
    ) -> Result<IntervalResult, IntervalError> {
        if self.laps.is_empty() {
            return Err(IntervalError::InsufficientData(
                "No laps provided".to_string(),
            ));
        }

        let mut laps = self.laps.to_vec();
        laps.sort_by_key(|lap| lap.lap_index);

        let speeds: Vec<f64> = laps.iter().map(|lap| lap.average_speed.max(0.0)).collect();
        let (cluster_low_mps, cluster_high_mps, split_threshold_mps) = if speeds.len() >= 2 {
            stats::kmeans_k2(&speeds, 50)
        } else {
            let s = speeds[0];
            (s, s, s)
        };

        // If low/high clusters are too close, this likely isn't an interval split.
        let min_cluster_gap = (config.hysteresis_delta_mps * 2.0).max(0.25);
        let has_work_recovery_split = (cluster_high_mps - cluster_low_mps) >= min_cluster_gap;
        // Midpoint can be too permissive when recoveries are very slow.
        // Require laps to be clearly in the high-speed cluster to be "work".
        let work_threshold_mps =
            split_threshold_mps + (cluster_high_mps - split_threshold_mps) * 0.35;

        let mut t_cursor = 0.0;
        let mut segments = Vec::with_capacity(laps.len());

        for lap in &laps {
            let duration_s = lap.elapsed_time.max(lap.moving_time).max(1) as f64;
            let distance_m = lap.distance.max(0.0);
            let avg_speed_mps = lap.average_speed.max(0.0);
            let is_work = has_work_recovery_split
                && avg_speed_mps >= work_threshold_mps
                && duration_s >= config.min_work_duration_s
                && distance_m >= config.min_work_distance_m;

            let start_t = t_cursor;
            let end_t = start_t + duration_s;
            t_cursor = end_t;

            segments.push(Segment {
                kind: if is_work {
                    SegmentKind::Work
                } else {
                    SegmentKind::Recovery
                },
                start_t,
                end_t,
                duration_s,
                distance_m,
                avg_speed_mps,
                speed_std_mps: 0.0,
                max_speed_mps: lap.max_speed.max(avg_speed_mps),
                avg_hr: lap.average_heartrate,
                avg_cadence: None,
            });
        }

        // Guard against a common false positive:
        // an easy opening lap classified as work just because midpoint threshold is low.
        if matches!(segments.first().map(|s| s.kind), Some(SegmentKind::Work)) {
            let work_speeds: Vec<f64> = segments
                .iter()
                .filter(|s| s.kind == SegmentKind::Work)
                .map(|s| s.avg_speed_mps)
                .collect();
            if work_speeds.len() >= 2 {
                let median_work_speed = stats::median(&work_speeds);
                if median_work_speed > 0.0 && segments[0].avg_speed_mps < median_work_speed * 0.85 {
                    segments[0].kind = SegmentKind::Recovery;
                }
            }
        }

        label_lap_warmup_cooldown(&mut segments, config);

        let mut reps_list = build_reps_from_work_segments(&segments);
        intensity::compute_intensity(&mut reps_list, mas_kmh);

        // TODO: scoring logic could be factorized into the trait, since it is the same between both
        // `IntervalParsingAlgorithm`
        let interval_score = compute_interval_score(
            &reps_list,
            &segments,
            cluster_low_mps,
            cluster_high_mps,
            config,
        );
        let is_interval_workout =
            reps_list.len() >= config.min_work_segments && interval_score >= 0.55;

        Ok(IntervalResult {
            segments,
            reps: reps_list,
            is_interval_workout,
            interval_score,
            threshold_speed_mps: work_threshold_mps,
            cluster_low_mps,
            cluster_high_mps,
        })
    }
}

fn label_lap_warmup_cooldown(segments: &mut [Segment], config: &IntervalConfig) {
    let first_work = segments.iter().position(|s| s.kind == SegmentKind::Work);
    let last_work = segments.iter().rposition(|s| s.kind == SegmentKind::Work);

    if let Some(first) = first_work {
        let warmup_duration: f64 = segments[..first].iter().map(|s| s.duration_s).sum();
        if warmup_duration >= config.warmup_min_s {
            for seg in &mut segments[..first] {
                if seg.kind == SegmentKind::Recovery {
                    seg.kind = SegmentKind::Warmup;
                }
            }
        }
    }

    if let Some(last) = last_work {
        if last + 1 < segments.len() {
            let cooldown_duration: f64 = segments[last + 1..].iter().map(|s| s.duration_s).sum();
            if cooldown_duration >= config.cooldown_min_s {
                for seg in &mut segments[last + 1..] {
                    if seg.kind == SegmentKind::Recovery {
                        seg.kind = SegmentKind::Cooldown;
                    }
                }
            }
        }
    }
}

fn build_reps_from_work_segments(segments: &[Segment]) -> Vec<Rep> {
    let work_segments: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.kind == SegmentKind::Work)
        .collect();

    work_segments
        .iter()
        .enumerate()
        .map(|(i, work)| {
            let recovery_duration_s = work_segments
                .get(i + 1)
                .map(|next| next.start_t - work.end_t);
            let avg_speed_mps = work.avg_speed_mps;
            let avg_pace_s_per_km = if avg_speed_mps > 0.0 {
                1000.0 / avg_speed_mps
            } else {
                0.0
            };

            Rep {
                work: (*work).clone(),
                rep_index: i,
                set_index: None,
                distance_m: work.distance_m,
                duration_s: work.duration_s,
                avg_pace_s_per_km,
                avg_speed_mps,
                pace_std: 0.0,
                pct_mas: None,
                steadiness: 1.0,
                fade: 0.0,
                recovery_duration_s,
            }
        })
        .collect()
}
