//! Error types for chemistry workflow contracts.

use deepseek_science_common::CommonError;
use thiserror::Error;

/// Errors raised while validating chemistry kinetics inputs.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum KineticsError {
    /// The caller-provided time column name was not present in the table.
    #[error("missing kinetics time column: {name}")]
    MissingTimeColumn {
        /// Requested time column name.
        name: String,
    },

    /// The caller-provided concentration column name was not present in the table.
    #[error("missing kinetics concentration column: {name}")]
    MissingConcentrationColumn {
        /// Requested concentration column name.
        name: String,
    },

    /// Too few positive-concentration rows remained after validation.
    #[error(
        "not enough valid kinetics points: {valid_count}, minimum required: {minimum_required}"
    )]
    NotEnoughValidPoints {
        /// Number of rows accepted after validation.
        valid_count: usize,
        /// Minimum number of rows required for later deterministic fitting.
        minimum_required: usize,
    },

    /// Error raised by shared in-memory table contracts.
    #[error(transparent)]
    Common(#[from] CommonError),
}
