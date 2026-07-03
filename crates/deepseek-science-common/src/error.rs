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

    /// CSV input did not contain a header row.
    #[error("CSV input must contain a header row")]
    MissingCsvHeader,

    /// A CSV header cell was empty after trimming.
    #[error("CSV header at column {column_index} must not be empty")]
    EmptyCsvHeaderName {
        /// Zero-based column index in the header row.
        column_index: usize,
    },

    /// CSV input contained a header but no data rows.
    #[error("CSV input must contain at least one data row")]
    NoCsvDataRows,

    /// A CSV data row did not have the header field count.
    #[error("CSV data row {row_index} has {actual} fields, expected {expected}")]
    InconsistentCsvFieldCount {
        /// Zero-based data row index, excluding the header row.
        row_index: usize,
        /// Field count from the header row.
        expected: usize,
        /// Field count found in the data row.
        actual: usize,
    },

    /// A CSV numeric cell was empty after trimming.
    #[error("CSV data row {row_index}, column `{column_name}` must not be empty")]
    EmptyCsvNumericCell {
        /// Zero-based data row index, excluding the header row.
        row_index: usize,
        /// Zero-based column index.
        column_index: usize,
        /// Column name from the header row.
        column_name: String,
    },

    /// A CSV numeric cell could not be parsed as an f64.
    #[error("CSV data row {row_index}, column `{column_name}` has invalid float value `{value}`")]
    InvalidCsvFloat {
        /// Zero-based data row index, excluding the header row.
        row_index: usize,
        /// Zero-based column index.
        column_index: usize,
        /// Column name from the header row.
        column_name: String,
        /// Trimmed cell value.
        value: String,
    },

    /// A CSV numeric cell parsed to NaN or infinity.
    #[error(
        "CSV data row {row_index}, column `{column_name}` has non-finite float value `{value}`"
    )]
    NonFiniteCsvFloat {
        /// Zero-based data row index, excluding the header row.
        row_index: usize,
        /// Zero-based column index.
        column_index: usize,
        /// Column name from the header row.
        column_name: String,
        /// Trimmed cell value.
        value: String,
    },

    /// The minimal CSV adapter does not support quoted fields.
    #[error("quoted CSV fields are not supported")]
    UnsupportedCsvQuotedField {
        /// Zero-based data row index, or None when the header contains quotes.
        row_index: Option<usize>,
        /// Zero-based column index.
        column_index: usize,
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
