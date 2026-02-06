use thiserror::Error;

#[derive(Debug, Error)]
pub enum IntervalError {
    #[error("Missing required stream: {0}")]
    MissingStream(String),

    #[error("Failed to parse stream data: {0}")]
    ParseError(String),

    #[error("Inconsistent stream lengths: expected {expected}, got distance={got_distance}, velocity={got_velocity}")]
    InconsistentLengths {
        expected: usize,
        got_distance: usize,
        got_velocity: usize,
    },

    #[error("Empty streams")]
    EmptyStreams,

    #[error("Insufficient data for segmentation: {0}")]
    InsufficientData(String),
}
