#![forbid(unsafe_code)]
//! Small pure-Rust scientific utilities shared by future domain packs.
//!
//! This crate stays lightweight. It contains small in-memory numeric helpers
//! and table adapters, without calling external tools or encoding domain
//! workflows.

pub mod csv;
pub mod delimited;
pub mod encoding;
pub mod error;
pub mod fitting;
pub mod statistics;
pub mod table;
pub mod units;

pub use csv::parse_simple_numeric_csv;
pub use delimited::{
    assess_simple_csv_compatibility, inspect_delimited_text, BoundedLineEvidence,
    DelimitedInspectionError, DelimitedTextInspection, DelimiterFinding, GenericTableShape,
    SimpleCsvCompatibility, TableRegionInspection, TableShapeReason,
};
pub use encoding::{
    inspect_text_encoding, ByteOrderMark, EncodingInspection, EncodingInspectionError,
    TextEncoding, MAX_INSPECTION_BYTES,
};
pub use error::CommonError;
pub use fitting::{simple_linear_regression, LinearRegression};
pub use statistics::mean;
pub use table::{ColumnName, DataColumn, DataTable, TableShape};
pub use units::Unit;
