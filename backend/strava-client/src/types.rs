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
pub struct WebhookSubscriptionResponse {
    pub id: i64,
    pub callback_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StravaLap {
    pub id: i64,
    pub name: Option<String>,
    pub elapsed_time: i32,
    pub moving_time: i32,
    pub start_date: String,
    pub distance: f64,
    pub average_speed: f64,
    pub max_speed: f64,
    pub total_elevation_gain: f64,
    pub average_heartrate: Option<f64>,
    pub max_heartrate: Option<f64>,
    pub split: Option<i32>,
    pub lap_index: Option<i32>,
}

/// A single stream entry inside the keyed response: `{"data": [...], ...}`
#[derive(Debug, Deserialize)]
pub struct StravaStreamEntry {
    pub data: serde_json::Value,
}

/// Parsed stream with its type name attached.
#[derive(Debug)]
pub struct StravaStream {
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

/// Parse the Strava streams response body.
///
/// Strava returns a map keyed by stream type:
/// `{"time": {"data": [0,1,2,...]}, "heartrate": {"data": [120,121,...]}, ...}`
pub fn parse_streams_response(body: &str) -> Result<Vec<StravaStream>, String> {
    let map: std::collections::HashMap<String, StravaStreamEntry> = serde_json::from_str(body)
        .map_err(|e| {
            let preview = if body.len() > 500 {
                format!("{}...", &body[..500])
            } else {
                body.to_string()
            };
            format!("Failed to parse streams response: {e}\nResponse preview: {preview}")
        })?;

    Ok(map
        .into_iter()
        .map(|(key, entry)| StravaStream {
            stream_type: key,
            data: entry.data,
        })
        .collect())
}
