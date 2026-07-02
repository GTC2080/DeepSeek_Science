//! Error types for shared scientific utilities.

use thiserror::Error;

/// Errors raised by small deterministic scientific helpers.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CommonError {
    /// A calculation received an empty slice.
    #[error("input slice must not be empty")]
    EmptyInput,

    /// A table column name was empty.
    #[error("column name must not be empty")]
    EmptyColumnName,

    /// A calculation received NaN or infinity.
    #[error("input values must be finite")]
    NonFiniteValue,

    /// Related slices did not have the same length.
    #[error("input slices must have the same length")]
    LengthMismatch,

    /// A table was created without columns.
    #[error("table must contain at least one column")]
    EmptyTable,

    /// A table contained the same column name more than once.
    #[error("duplicate column name: {name}")]
    DuplicateColumnName {
        /// Duplicate column name.
        name: String,
    },

    /// A table column length did not match the first column length.
    #[error("column `{name}` has length {actual}, expected {expected}")]
    ColumnLengthMismatch {
        /// Column name.
        name: String,
        /// Expected length.
        expected: usize,
        /// Actual length.
        actual: usize,
    },

    /// A requested column was not present.
    #[error("missing column: {name}")]
    MissingColumn {
        /// Requested column name.
        name: String,
    },

    /// A calculation requires at least two observations.
    #[error("at least two observations are required")]
    TooFewObservations,

    /// The independent variable has no variance.
    #[error("x values must not all be identical")]
    ZeroVariance,
}
