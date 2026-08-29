//! What a failed parse reports.

use alloc::string::String;
use thiserror::Error;

/// Why a Maxwell message failed to parse.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The message carries a `type` this crate does not model, most likely because it was
    /// added by a newer Maxwell. Callers tailing a stream may choose to skip these.
    #[error("unrecognised Maxwell message type `{0}`")]
    UnknownMessageType(String),
    /// The input is not valid JSON, or is not a valid message of its declared type.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// A parse failure at a known line of a JSON Lines stream.
#[derive(Debug, Error)]
#[error("line {line}: {source}")]
pub struct LineError {
    /// Line number, counting from one.
    pub line: usize,
    /// The underlying failure.
    #[source]
    pub source: ParseError,
}
