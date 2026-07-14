//! Error types for artifact metadata.

use thiserror::Error;

/// Errors raised while constructing or validating artifact metadata.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ArtifactError {
    /// Artifact content hash was empty.
    #[error("artifact content hash must not be empty")]
    EmptyContentHash,
    /// A required string field contained only whitespace or no characters.
    #[error("artifact field {field} must not be empty")]
    EmptyField {
        /// Stable field name associated with the invalid value.
        field: &'static str,
    },
    /// Artifact metadata did not describe any source inputs.
    #[error("artifact inputs must not be empty")]
    EmptyInputs,
    /// The envelope payload was empty.
    #[error("artifact payload must not be empty")]
    EmptyPayload,
    /// The envelope payload began with a UTF-8 byte-order mark.
    #[error("artifact payload must not begin with a UTF-8 BOM")]
    PayloadHasUtf8Bom,
    /// The envelope payload contained a carriage return.
    #[error("artifact payload must use LF line endings")]
    PayloadContainsCarriageReturn,
    /// The envelope payload did not end with an LF.
    #[error("artifact payload must end with one LF")]
    PayloadMissingFinalLf,
    /// The envelope payload ended with more than one LF.
    #[error("artifact payload must end with exactly one LF")]
    PayloadHasMultipleTrailingLf,
    /// The content descriptor did not declare UTF-8 encoding.
    #[error("artifact payload encoding does not match its content descriptor")]
    PayloadEncodingMismatch,
    /// The content descriptor byte length did not match the exact payload bytes.
    #[error("artifact payload byte length does not match its content descriptor")]
    PayloadByteLengthMismatch,
    /// The content descriptor hash did not match the exact payload bytes.
    #[error("artifact payload hash does not match its content descriptor")]
    PayloadHashMismatch,
    /// An in-memory byte length could not be represented by the contract.
    #[error("artifact byte length exceeds the supported range")]
    ByteLengthOverflow,
    /// Deterministic JSON serialization failed for an internal reason.
    #[error("artifact envelope serialization failed")]
    SerializationFailed,
    /// Deterministic JSON serialization exceeded the caller-provided limit.
    #[error("serialized artifact envelope exceeds {maximum} bytes")]
    SerializedEnvelopeTooLarge {
        /// Inclusive maximum serialized envelope length.
        maximum: usize,
    },
}
