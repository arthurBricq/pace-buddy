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
