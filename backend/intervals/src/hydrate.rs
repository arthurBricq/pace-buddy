use domain::{ActivityStream, StreamType};

use crate::error::IntervalError;

/// Typed arrays extracted from activity streams.
#[derive(Debug, Clone)]
pub struct HydratedStreams {
    /// Time in seconds since activity start (required)
    pub time: Vec<f64>,
    /// Cumulative distance in meters (required)
    pub distance: Vec<f64>,
    /// Smoothed velocity in m/s (required)
    pub velocity: Vec<f64>,
    /// Whether the athlete is moving at each sample (optional)
    pub moving: Option<Vec<bool>>,
    /// Heart rate in bpm (optional)
    pub heartrate: Option<Vec<f64>>,
    /// Cadence (optional)
    pub cadence: Option<Vec<f64>>,
    /// Altitude in meters (optional)
    pub altitude: Option<Vec<f64>>,
}

impl HydratedStreams {
    pub fn len(&self) -> usize {
        self.time.len()
    }
}

/// Parse a JSON array of numbers into Vec<f64>.
fn parse_f64_array(json: &str) -> Result<Vec<f64>, IntervalError> {
    let values: Vec<serde_json::Value> =
        serde_json::from_str(json).map_err(|e| IntervalError::ParseError(e.to_string()))?;
    values
        .into_iter()
        .map(|v| {
            v.as_f64()
                .ok_or_else(|| IntervalError::ParseError(format!("Expected number, got: {v}")))
        })
        .collect()
}

/// Parse a JSON array of booleans into Vec<bool>.
fn parse_bool_array(json: &str) -> Result<Vec<bool>, IntervalError> {
    serde_json::from_str(json).map_err(|e| IntervalError::ParseError(e.to_string()))
}

/// Extract typed arrays from raw ActivityStream objects.
///
/// Requires `time`, `distance`, and `velocity_smooth` streams.
/// Optionally uses `moving`, `heartrate`, `cadence`, `altitude`.
pub fn hydrate(streams: &[ActivityStream]) -> Result<HydratedStreams, IntervalError> {
    let find = |st: StreamType| -> Option<&ActivityStream> {
        streams.iter().find(|s| s.stream_type == st)
    };

    let time_stream = find(StreamType::Time)
        .ok_or_else(|| IntervalError::MissingStream("time".into()))?;
    let distance_stream = find(StreamType::Distance)
        .ok_or_else(|| IntervalError::MissingStream("distance".into()))?;
    let velocity_stream = find(StreamType::VelocitySmooth)
        .ok_or_else(|| IntervalError::MissingStream("velocity_smooth".into()))?;

    let time = parse_f64_array(&time_stream.data_json)?;
    let distance = parse_f64_array(&distance_stream.data_json)?;
    let velocity = parse_f64_array(&velocity_stream.data_json)?;

    // Validate consistent lengths
    let n = time.len();
    if distance.len() != n || velocity.len() != n {
        return Err(IntervalError::InconsistentLengths {
            expected: n,
            got_distance: distance.len(),
            got_velocity: velocity.len(),
        });
    }
    if n == 0 {
        return Err(IntervalError::EmptyStreams);
    }

    let moving = if let Some(s) = find(StreamType::Moving) {
        let m = parse_bool_array(&s.data_json)?;
        if m.len() != n {
            return Err(IntervalError::ParseError(format!(
                "moving stream length {} != expected {n}",
                m.len()
            )));
        }
        Some(m)
    } else {
        None
    };

    let heartrate = if let Some(s) = find(StreamType::HeartRate) {
        let h = parse_f64_array(&s.data_json)?;
        if h.len() == n {
            Some(h)
        } else {
            None // silently ignore mismatched optional stream
        }
    } else {
        None
    };

    let cadence = if let Some(s) = find(StreamType::Cadence) {
        let c = parse_f64_array(&s.data_json)?;
        if c.len() == n {
            Some(c)
        } else {
            None
        }
    } else {
        None
    };

    let altitude = if let Some(s) = find(StreamType::Altitude) {
        let a = parse_f64_array(&s.data_json)?;
        if a.len() == n {
            Some(a)
        } else {
            None
        }
    } else {
        None
    };

    Ok(HydratedStreams {
        time,
        distance,
        velocity,
        moving,
        heartrate,
        cadence,
        altitude,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_stream(stream_type: StreamType, data_json: &str) -> ActivityStream {
        ActivityStream {
            activity_id: Uuid::nil(),
            stream_type,
            data_json: data_json.to_string(),
        }
    }

    #[test]
    fn test_hydrate_basic() {
        let streams = vec![
            make_stream(StreamType::Time, "[0, 1, 2, 3, 4]"),
            make_stream(StreamType::Distance, "[0, 3, 6, 9, 12]"),
            make_stream(StreamType::VelocitySmooth, "[3.0, 3.0, 3.0, 3.0, 3.0]"),
        ];
        let h = hydrate(&streams).unwrap();
        assert_eq!(h.len(), 5);
        assert!(h.moving.is_none());
        assert!(h.heartrate.is_none());
    }

    #[test]
    fn test_hydrate_with_optional() {
        let streams = vec![
            make_stream(StreamType::Time, "[0, 1, 2]"),
            make_stream(StreamType::Distance, "[0, 3, 6]"),
            make_stream(StreamType::VelocitySmooth, "[3.0, 3.0, 3.0]"),
            make_stream(StreamType::Moving, "[true, true, false]"),
            make_stream(StreamType::HeartRate, "[140, 145, 150]"),
        ];
        let h = hydrate(&streams).unwrap();
        assert!(h.moving.is_some());
        assert!(h.heartrate.is_some());
    }

    #[test]
    fn test_hydrate_missing_required() {
        let streams = vec![
            make_stream(StreamType::Time, "[0, 1, 2]"),
            make_stream(StreamType::Distance, "[0, 3, 6]"),
            // missing velocity_smooth
        ];
        assert!(hydrate(&streams).is_err());
    }

    #[test]
    fn test_hydrate_inconsistent_lengths() {
        let streams = vec![
            make_stream(StreamType::Time, "[0, 1, 2]"),
            make_stream(StreamType::Distance, "[0, 3]"),
            make_stream(StreamType::VelocitySmooth, "[3.0, 3.0, 3.0]"),
        ];
        assert!(hydrate(&streams).is_err());
    }
}
