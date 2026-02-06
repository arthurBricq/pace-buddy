use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub athlete: Option<AthleteResponse>,
}

#[derive(Debug, Deserialize)]
pub struct AthleteResponse {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct StravaActivity {
    pub id: i64,
    pub name: String,
    pub sport_type: Option<String>,
    #[serde(rename = "type")]
    pub activity_type: Option<String>,
    pub start_date: String,
    pub elapsed_time: i32,
    pub moving_time: i32,
    pub distance: f64,
    pub total_elevation_gain: f64,
    pub average_speed: f64,
    pub max_speed: f64,
    pub average_heartrate: Option<f64>,
    pub max_heartrate: Option<f64>,
    pub average_cadence: Option<f64>,
    pub average_watts: Option<f64>,
    pub calories: Option<f64>,
    pub workout_type: Option<i32>,
    pub map: Option<StravaMap>,
}

#[derive(Debug, Deserialize)]
pub struct StravaMap {
    pub summary_polyline: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StravaStreamSet {
    // Each stream in the response array has type and data
}

// Strava returns streams as an array of objects like:
// [{"type": "time", "data": [0, 1, 2, ...]}, {"type": "heartrate", "data": [120, 121, ...]}]
#[derive(Debug, Deserialize)]
pub struct StravaStream {
    #[serde(rename = "type")]
    pub stream_type: String,
    pub data: serde_json::Value,
}

impl StravaStream {
    /// Try to parse the stream_type string into a domain StreamType.
    /// Returns None for unknown/unsupported stream types.
    pub fn parsed_type(&self) -> Option<domain::StreamType> {
        self.stream_type.parse().ok()
    }
}
