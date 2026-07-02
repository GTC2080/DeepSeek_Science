#![forbid(unsafe_code)]
//! Small pure-Rust scientific utilities shared by future domain packs.
//!
//! This crate stays lightweight. It does not parse CSV, call external tools, or
//! encode domain workflows in Phase 1.

pub mod error;
pub mod fitting;
pub mod statistics;
pub mod table;
pub mod units;

pub use error::CommonError;
pub use fitting::{simple_linear_regression, LinearRegression};
pub use statistics::mean;
pub use table::TableShape;
pub use units::Unit;
