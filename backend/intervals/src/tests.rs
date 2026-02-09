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

// ---------------------------------------------------------------------------
// Configurable synthetic stream builder
// ---------------------------------------------------------------------------

/// Configuration for generating synthetic interval streams.
struct SyntheticSession {
    /// Warmup duration in seconds
    warmup_s: u32,
    /// Warmup speed in m/s
    warmup_speed: f64,
    /// Number of reps
    reps: u32,
    /// Work duration per rep in seconds
    work_s: u32,
    /// Work speed in m/s
    work_speed: f64,
    /// Recovery duration per rep in seconds
    recovery_s: u32,
    /// Recovery speed in m/s (0.0 = standing stop)
    recovery_speed: f64,
    /// Cooldown duration in seconds
    cooldown_s: u32,
    /// Cooldown speed in m/s
    cooldown_speed: f64,
    /// Seed for deterministic pseudo-random noise
    seed: u64,
}

/// Simple deterministic LCG PRNG returning values in [-1, 1].
struct Noise {
    state: u64,
}

impl Noise {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Return a value in [-amplitude, +amplitude].
    fn next(&mut self, amplitude: f64) -> f64 {
        // LCG parameters (Numerical Recipes)
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let normalized = ((self.state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0;
        normalized * amplitude
    }
}

/// Build synthetic activity streams from a session config.
///
/// Adds GPS-like noise to velocity and distance, and simulates
/// realistic `moving` flag behavior (flickering during standing stops).
fn build_synthetic_streams(cfg: &SyntheticSession) -> Vec<ActivityStream> {
    let mut time = Vec::new();
    let mut distance = Vec::new();
    let mut velocity = Vec::new();
    let mut moving = Vec::new();
    let mut noise = Noise::new(cfg.seed);

    let mut t = 0.0;
    let mut d = 0.0;

    let speed_noise = 0.15; // ±0.15 m/s GPS jitter on speed
    let stop_drift = 0.3; // GPS drift when standing still

    // Helper: push one second of data at a target speed
    let push = |target_speed: f64,
                is_stop: bool,
                t: &mut f64,
                d: &mut f64,
                time: &mut Vec<f64>,
                distance: &mut Vec<f64>,
                velocity: &mut Vec<f64>,
                moving: &mut Vec<bool>,
                noise: &mut Noise| {
        let actual_speed = if is_stop {
            // Standing: GPS drift around 0, occasionally spikes
            (stop_drift + noise.next(stop_drift)).max(0.0)
        } else {
            (target_speed + noise.next(speed_noise)).max(0.0)
        };
        time.push(*t);
        distance.push(*d);
        velocity.push(actual_speed);
        // moving flag: false when stopped, with occasional flicker
        if is_stop {
            // ~70% of the time Strava says not moving, 30% GPS flicker says moving
            moving.push(noise.next(1.0) > 0.4);
        } else {
            moving.push(true);
        }
        *t += 1.0;
        *d += actual_speed;
    };

    let is_stop = cfg.recovery_speed < 0.5;

    // Warmup
    for _ in 0..cfg.warmup_s {
        push(
            cfg.warmup_speed,
            false,
            &mut t,
            &mut d,
            &mut time,
            &mut distance,
            &mut velocity,
            &mut moving,
            &mut noise,
        );
    }

    // Reps
    for _ in 0..cfg.reps {
        // Work phase
        for _ in 0..cfg.work_s {
            push(
                cfg.work_speed,
                false,
                &mut t,
                &mut d,
                &mut time,
                &mut distance,
                &mut velocity,
                &mut moving,
                &mut noise,
            );
        }
        // Recovery phase
        for _ in 0..cfg.recovery_s {
            push(
                cfg.recovery_speed,
                is_stop,
                &mut t,
                &mut d,
                &mut time,
                &mut distance,
                &mut velocity,
                &mut moving,
                &mut noise,
            );
        }
    }

    // Cooldown
    for _ in 0..cfg.cooldown_s {
        push(
            cfg.cooldown_speed,
            false,
            &mut t,
            &mut d,
            &mut time,
            &mut distance,
            &mut velocity,
            &mut moving,
            &mut noise,
        );
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_5x60s_jog_recovery() {
    let streams = build_synthetic_streams(&SyntheticSession {
        warmup_s: 420,
        warmup_speed: 3.0,
        reps: 5,
        work_s: 60,
        work_speed: 5.0,
        recovery_s: 90,
        recovery_speed: 2.5,
        cooldown_s: 360,
        cooldown_speed: 3.0,
        seed: 42,
    });
    let config = IntervalConfig::default();
    let result = parse_intervals(&streams, &config, Some(18.0)).unwrap();

    assert!(
        result.is_interval_workout,
        "Should detect as interval workout"
    );
    assert_eq!(
        result.reps.len(),
        5,
        "Expected 5 reps, got {}",
        result.reps.len()
    );
    assert!(
        result.interval_score > 0.5,
        "Score too low: {}",
        result.interval_score
    );

    for rep in &result.reps {
        assert!(rep.pct_mas.is_some());
        assert!(
            rep.distance_m > 200.0 && rep.distance_m < 400.0,
            "Rep distance out of range: {:.0}m",
            rep.distance_m
        );
        assert!(
            rep.duration_s > 40.0 && rep.duration_s < 80.0,
            "Rep duration out of range: {:.0}s",
            rep.duration_s
        );
    }
}

#[test]
fn test_12x80s_stop_recovery() {
    // Simulates 12x400m with standing recovery (like the real session)
    let streams = build_synthetic_streams(&SyntheticSession {
        warmup_s: 500,
        warmup_speed: 2.8,
        reps: 12,
        work_s: 80,
        work_speed: 5.0,
        recovery_s: 60,
        recovery_speed: 0.0, // standing stop
        cooldown_s: 400,
        cooldown_speed: 2.8,
        seed: 123,
    });
    let config = IntervalConfig::default();
    let result = parse_intervals(&streams, &config, Some(18.0)).unwrap();

    assert!(
        result.is_interval_workout,
        "Should detect as interval workout"
    );
    assert_eq!(
        result.reps.len(),
        12,
        "Expected 12 reps, got {}",
        result.reps.len()
    );

    for rep in &result.reps {
        assert!(
            rep.distance_m > 300.0 && rep.distance_m < 500.0,
            "Rep distance out of range: {:.0}m",
            rep.distance_m
        );
        // Standing recovery should be classified as Stop
        if let Some(style) = &rep.recovery_style {
            assert_eq!(
                *style,
                types::RecoveryStyle::Stop,
                "Expected Stop recovery, got {:?}",
                style
            );
        }
    }
}

#[test]
fn test_6x3min_jog_recovery() {
    // 6 x 3min hard / 2min jog
    let streams = build_synthetic_streams(&SyntheticSession {
        warmup_s: 600,
        warmup_speed: 3.0,
        reps: 6,
        work_s: 180,
        work_speed: 4.5,
        recovery_s: 120,
        recovery_speed: 2.5,
        cooldown_s: 480,
        cooldown_speed: 3.0,
        seed: 999,
    });
    let config = IntervalConfig::default();
    let result = parse_intervals(&streams, &config, None).unwrap();

    assert!(result.is_interval_workout);
    assert_eq!(
        result.reps.len(),
        6,
        "Expected 6 reps, got {}",
        result.reps.len()
    );

    for rep in &result.reps {
        assert!(
            rep.duration_s > 140.0 && rep.duration_s < 220.0,
            "Rep duration out of range: {:.0}s",
            rep.duration_s
        );
        if let Some(style) = &rep.recovery_style {
            assert_eq!(
                *style,
                types::RecoveryStyle::Jog,
                "Expected Jog recovery, got {:?}",
                style
            );
        }
    }
}

#[test]
fn test_steady_run_not_intervals() {
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

#[test]
fn test_short_reps_strides() {
    // 8 x 20s strides with jog recovery — short but should still detect
    let streams = build_synthetic_streams(&SyntheticSession {
        warmup_s: 600,
        warmup_speed: 3.0,
        reps: 8,
        work_s: 20,
        work_speed: 5.5,
        recovery_s: 60,
        recovery_speed: 2.5,
        cooldown_s: 300,
        cooldown_speed: 3.0,
        seed: 7,
    });
    let config = IntervalConfig::default();
    let result = parse_intervals(&streams, &config, None).unwrap();

    assert!(result.is_interval_workout);
    assert_eq!(
        result.reps.len(),
        8,
        "Expected 8 reps, got {}",
        result.reps.len()
    );

    for rep in &result.reps {
        assert!(
            rep.duration_s > 10.0 && rep.duration_s < 35.0,
            "Rep duration out of range: {:.0}s",
            rep.duration_s
        );
    }
}
