//! Scripted sequence LLM client for testing/debugging.
//!
//! This module provides an LLM client that plays a predetermined sequence
//! of moves with a configurable delay between each move. Useful for debugging
//! the chess game flow with known game outcomes.

use crate::{ChatMessage, LlmClient, LlmError, ModelInfo};
use async_trait::async_trait;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

/// An LLM client that plays a predetermined sequence of moves.
///
/// This is useful for debugging and testing, as it allows you to
/// replay known games or test specific positions without needing
/// an actual LLM API.
pub struct SequenceLlmClient {
    /// The sequence of moves to play (in UCI format)
    moves: Vec<String>,
    /// Delay before returning each move
    delay: Duration,
    /// Current move index (for this player's moves only)
    current_index: AtomicUsize,
    /// Display name for this client
    name: String,
}

impl SequenceLlmClient {
    /// Creates a new sequence client with the given moves and delay.
    ///
    /// # Arguments
    /// * `moves` - A vector of moves in UCI format (e.g., ["e2e4", "d7d5", "e4d5"])
    /// * `delay` - Duration to wait before returning each move
    /// * `name` - Display name for this client (e.g., "White Sequence" or "Black Sequence")
    pub fn new(moves: Vec<String>, delay: Duration, name: impl Into<String>) -> Self {
        Self {
            moves,
            delay,
            current_index: AtomicUsize::new(0),
            name: name.into(),
        }
    }

    /// Creates a Fool's Mate sequence for testing (4 moves total, checkmate in 2).
    ///
    /// The sequence is:
    /// 1. f2f3 (White blunders)
    /// 2. e7e5 (Black develops)
    /// 3. g2g4 (White blunders again)
    /// 4. d8h4 (Black delivers checkmate)
    ///
    /// Returns a tuple of (white_client, black_client)
    pub fn fools_mate(delay: Duration) -> (Self, Self) {
        let white = Self::new(
            vec!["f2f3".to_string(), "g2g4".to_string()],
            delay,
            "White (Fool's Mate)",
        );
        let black = Self::new(
            vec!["e7e5".to_string(), "d8h4".to_string()],
            delay,
            "Black (Fool's Mate)",
        );
        (white, black)
    }

    /// Creates a Scholar's Mate sequence for testing (7 moves total, checkmate in 4).
    ///
    /// The sequence is:
    /// 1. e2e4 e7e5
    /// 2. f1c4 b8c6
    /// 3. d1h5 g8f6
    /// 4. h5f7# (checkmate)
    ///
    /// Returns a tuple of (white_client, black_client)
    pub fn scholars_mate(delay: Duration) -> (Self, Self) {
        let white = Self::new(
            vec![
                "e2e4".to_string(),
                "f1c4".to_string(),
                "d1h5".to_string(),
                "h5f7".to_string(),
            ],
            delay,
            "White (Scholar's Mate)",
        );
        let black = Self::new(
            vec!["e7e5".to_string(), "b8c6".to_string(), "g8f6".to_string()],
            delay,
            "Black (Scholar's Mate)",
        );
        (white, black)
    }
}

#[async_trait]
impl LlmClient for SequenceLlmClient {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        Ok(vec![ModelInfo {
            id: "sequence/scripted".to_string(),
            name: format!("Scripted Sequence ({})", self.name),
            description: Some(format!(
                "A scripted sequence of {} moves with {:?} delay",
                self.moves.len(),
                self.delay
            )),
            pricing: None,
            context_length: None,
        }])
    }

    async fn chat_completion(
        &self,
        _model: &str,
        _messages: Vec<ChatMessage>,
        _reasoning_effort: Option<&str>,
    ) -> Result<String, LlmError> {
        // Get current index and increment
        let index = self.current_index.fetch_add(1, Ordering::SeqCst);

        // Check if we have a move at this index
        if index >= self.moves.len() {
            log::warn!(
                "[{}] Sequence exhausted at index {}, no more moves available",
                self.name,
                index
            );
            return Err(LlmError::NoContent);
        }

        let next_move = &self.moves[index];

        log::info!(
            "[{}] Playing move {}/{}: {} (waiting {:?})",
            self.name,
            index + 1,
            self.moves.len(),
            next_move,
            self.delay
        );

        // Wait for the configured delay
        // tokio::time::sleep(self.delay).await;

        Ok(next_move.clone())
    }
}

/// Model ID for white player in sequence mode
pub const SEQUENCE_WHITE_MODEL: &str = "sequence/white";
/// Model ID for black player in sequence mode
pub const SEQUENCE_BLACK_MODEL: &str = "sequence/black";

/// A dual LLM client that alternates between white and black moves.
///
/// This client wraps two `SequenceLlmClient` instances and automatically
/// alternates between them based on the total move count (white plays on
/// odd calls, black plays on even calls). This makes it work regardless
/// of which model the user selects in the UI.
pub struct DualSequenceClient {
    white: SequenceLlmClient,
    black: SequenceLlmClient,
    /// Total number of moves made (used to determine whose turn it is)
    move_count: AtomicUsize,
}

impl DualSequenceClient {
    /// Creates a new dual sequence client with separate white and black move sequences.
    pub const fn new(white: SequenceLlmClient, black: SequenceLlmClient) -> Self {
        Self {
            white,
            black,
            move_count: AtomicUsize::new(0),
        }
    }

    /// Creates a Fool's Mate sequence (fastest checkmate).
    ///
    /// Moves: 1. f3 e5 2. g4 Qh4#
    pub fn fools_mate(delay: Duration) -> Self {
        let (white, black) = SequenceLlmClient::fools_mate(delay);
        Self::new(white, black)
    }

    /// Creates a Scholar's Mate sequence.
    ///
    /// Moves: 1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6 4. Qxf7#
    pub fn scholars_mate(delay: Duration) -> Self {
        let (white, black) = SequenceLlmClient::scholars_mate(delay);
        Self::new(white, black)
    }
}

#[async_trait]
impl LlmClient for DualSequenceClient {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        Ok(vec![
            ModelInfo {
                id: SEQUENCE_WHITE_MODEL.to_string(),
                name: "Scripted Sequence (White)".to_string(),
                description: Some(format!(
                    "White's scripted sequence of {} moves",
                    self.white.moves.len()
                )),
                pricing: None,
                context_length: None,
            },
            ModelInfo {
                id: SEQUENCE_BLACK_MODEL.to_string(),
                name: "Scripted Sequence (Black)".to_string(),
                description: Some(format!(
                    "Black's scripted sequence of {} moves",
                    self.black.moves.len()
                )),
                pricing: None,
                context_length: None,
            },
        ])
    }

    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        reasoning_effort: Option<&str>,
    ) -> Result<String, LlmError> {
        // Get current move count and increment
        let move_num = self.move_count.fetch_add(1, Ordering::SeqCst);

        // Alternate between white (even) and black (odd)
        // Move 0 = White's 1st move, Move 1 = Black's 1st move, etc.
        let is_white_turn = move_num.is_multiple_of(2);

        log::info!(
            "DualSequenceClient: move {} ({}), model requested: '{}'",
            move_num + 1,
            if is_white_turn { "White" } else { "Black" },
            model
        );

        if is_white_turn {
            self.white
                .chat_completion(model, messages, reasoning_effort)
                .await
        } else {
            self.black
                .chat_completion(model, messages, reasoning_effort)
                .await
        }
    }
}
