use crate::types::Rep;

/// Compute %MAS for each rep given MAS in km/h.
/// Updates rep.pct_mas in place.
pub fn compute_intensity(reps: &mut [Rep], mas_kmh: Option<f64>) {
    let mas_mps = match mas_kmh {
        Some(v) if v > 0.0 => v / 3.6,
        _ => return,
    };

    for rep in reps.iter_mut() {
        rep.pct_mas = Some(rep.avg_speed_mps / mas_mps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Segment, SegmentKind};

    fn make_rep(avg_speed_mps: f64) -> Rep {
        Rep {
            work: Segment {
                kind: SegmentKind::Work,
                start_t: 0.0,
                end_t: 80.0,
                duration_s: 80.0,
                distance_m: avg_speed_mps * 80.0,
                avg_speed_mps,
                speed_std_mps: 0.1,
                max_speed_mps: avg_speed_mps + 0.3,
                avg_hr: None,
                avg_cadence: None,
            },
            rep_index: 0,
            set_index: None,
            distance_m: avg_speed_mps * 80.0,
            duration_s: 80.0,
            avg_pace_s_per_km: 1000.0 / avg_speed_mps,
            avg_speed_mps,
            pace_std: 0.0,
            pct_mas: None,
            steadiness: 0.95,
            fade: 0.02,
            recovery_duration_s: None,
        }
    }

    #[test]
    fn test_compute_intensity() {
        let mut reps = vec![make_rep(5.0), make_rep(4.5)];
        // MAS = 18 km/h = 5.0 m/s
        compute_intensity(&mut reps, Some(18.0));
        assert!((reps[0].pct_mas.unwrap() - 1.0).abs() < 0.01);
        assert!((reps[1].pct_mas.unwrap() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_no_mas() {
        let mut reps = vec![make_rep(5.0)];
        compute_intensity(&mut reps, None);
        assert!(reps[0].pct_mas.is_none());
    }
}
