mod algorithms;
pub mod error;
pub mod fixtures;
pub mod hydrate;
pub mod intensity;
pub mod preprocess;
pub mod reps;
pub mod segment;
pub mod stats;
pub mod types;

#[cfg(test)]
mod tests;

pub use algorithms::{AutoSpeedSegmentationAlgorithm, ManualLapIntervalAlgorithm};
use domain::ActivityStream;
use error::IntervalError;
use types::{IntervalConfig, IntervalResult};

/// Abstraction for interval parsing strategies.
///
/// Different implementations can parse intervals from different signals
/// (for example speed-based segmentation or manual laps).
pub trait IntervalParsingAlgorithm {
    fn parse(
        &self,
        streams: &[ActivityStream],
        config: &IntervalConfig,
        mas_kmh: Option<f64>,
    ) -> Result<IntervalResult, IntervalError>;
}

/// Parse intervals with a specific algorithm implementation.
pub fn parse_intervals_with_algorithm(
    algorithm: &dyn IntervalParsingAlgorithm,
    streams: &[ActivityStream],
    config: &IntervalConfig,
    mas_kmh: Option<f64>,
) -> Result<IntervalResult, IntervalError> {
    algorithm.parse(streams, config, mas_kmh)
}
