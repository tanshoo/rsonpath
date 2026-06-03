//! Error definitions and utilities for validator engine execution.
use crate::engine::error::EngineError as RsonpathEngineError;
use crate::input::error::InputError;
use thiserror::Error;

/// Error enum for all types of errors that can be reported
/// during validator engine execution.
#[derive(Debug, Error)]
pub enum ValidatorEngineError {
    /// Engine error that occurred during execution.
    #[error(transparent)]
    RsonpathEngineError(#[from] RsonpathEngineError),
    /// Error while reading from the supplied [`Input`](crate::input::Input) implementation.
    #[error(transparent)]
    InputError(#[from] InputError),
    /// Property that is not allowed by the schema was found in the document.
    /// The inner [`usize`] value indicates the position of first character
    /// of the disallowed property name.
    #[error("Disallowed property at position {0}")]
    DisallowedProperty(usize),
    /// Type mismatch between the schema and the document.
    /// The inner [`usize`] value indicates the position of the value that caused the mismatch.
    #[error("Type mismatch at position {0}")]
    TypeMismatch(usize),
    /// Unexpected structural character in the document.
    /// The inner [`usize`] value indicates the position of the character.
    #[error("Unexpected structural character at position {0}")]
    UnexpectedStructural(usize),
    /// Unsupported JSON Schema feature.
    #[error("Unsupported JSON Schema feature: {0}")]
    UnsupportedFeature(&'static str),
}
