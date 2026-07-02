//! Error types for shared scientific utilities.

use thiserror::Error;

/// Errors raised by small deterministic scientific helpers.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CommonError {
    /// A calculation received an empty slice.
    #[error("input slice must not be empty")]
    EmptyInput,

    /// Related slices did not have the same length.
    #[error("input slices must have the same length")]
    LengthMismatch,

    /// A calculation requires at least two observations.
    #[error("at least two observations are required")]
    TooFewObservations,

    /// The independent variable has no variance.
    #[error("x values must not all be identical")]
    ZeroVariance,
}
