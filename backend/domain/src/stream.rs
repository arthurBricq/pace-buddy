use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamType {
    Time,
    Distance,
    #[serde(rename = "latlng")]
    LatLng,
    Altitude,
    #[serde(rename = "heartrate")]
    HeartRate,
    Cadence,
    Watts,
    #[serde(rename = "velocity_smooth")]
    VelocitySmooth,
    Moving,
}

impl std::fmt::Display for StreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamType::Time => write!(f, "time"),
            StreamType::Distance => write!(f, "distance"),
            StreamType::LatLng => write!(f, "latlng"),
            StreamType::Altitude => write!(f, "altitude"),
            StreamType::HeartRate => write!(f, "heartrate"),
            StreamType::Cadence => write!(f, "cadence"),
            StreamType::Watts => write!(f, "watts"),
            StreamType::VelocitySmooth => write!(f, "velocity_smooth"),
            StreamType::Moving => write!(f, "moving"),
        }
    }
}

impl std::str::FromStr for StreamType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "time" => Ok(StreamType::Time),
            "distance" => Ok(StreamType::Distance),
            "latlng" => Ok(StreamType::LatLng),
            "altitude" => Ok(StreamType::Altitude),
            "heartrate" => Ok(StreamType::HeartRate),
            "cadence" => Ok(StreamType::Cadence),
            "watts" => Ok(StreamType::Watts),
            "velocity_smooth" => Ok(StreamType::VelocitySmooth),
            "moving" => Ok(StreamType::Moving),
            other => Err(format!("Unknown stream type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStream {
    pub activity_id: Uuid,
    pub stream_type: StreamType,
    pub data_json: String,
}
