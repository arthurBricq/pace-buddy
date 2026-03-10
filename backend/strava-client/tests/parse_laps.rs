use chrono::{Datelike, Timelike};
use uuid::Uuid;

#[test]
fn test_parse_laps_and_convert_to_domain() {
    let json = r#"
[
  {
    "id": 123,
    "name": "Lap 1",
    "elapsed_time": 92,
    "moving_time": 90,
    "start_date": "2026-02-01T09:30:00Z",
    "distance": 400.0,
    "average_speed": 4.35,
    "max_speed": 5.10,
    "total_elevation_gain": 1.2,
    "average_heartrate": 168.0,
    "max_heartrate": 176.0,
    "split": 1
  },
  {
    "id": 124,
    "name": "Lap 2",
    "elapsed_time": 91,
    "moving_time": 89,
    "start_date": "2026-02-01T09:32:00Z",
    "distance": 400.0,
    "average_speed": 4.39,
    "max_speed": 5.22,
    "total_elevation_gain": 1.0,
    "average_heartrate": 170.0,
    "max_heartrate": 178.0,
    "lap_index": 2
  }
]
"#;

    let parsed: Vec<strava_client::StravaLap> =
        serde_json::from_str(json).expect("valid laps payload");
    let activity_id = Uuid::nil();
    let laps = strava_client::strava_laps_to_domain(parsed, activity_id);

    assert_eq!(laps.len(), 2);
    assert_eq!(laps[0].activity_id, activity_id);
    assert_eq!(laps[0].lap_index, 1);
    assert_eq!(laps[0].distance, 400.0);
    assert_eq!(laps[1].lap_index, 2);
    assert_eq!(laps[1].average_speed, 4.39);
    assert_eq!(laps[0].start_date.year(), 2026);
    assert_eq!(laps[0].start_date.month(), 2);
    assert_eq!(laps[0].start_date.day(), 1);
    assert_eq!(laps[0].start_date.hour(), 9);
    assert_eq!(laps[0].start_date.minute(), 30);
}
