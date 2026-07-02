//! Minimal table metadata for future parsers.

use serde::{Deserialize, Serialize};

/// Shape of a small in-memory table.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TableShape {
    /// Number of rows.
    pub rows: usize,
    /// Number of columns.
    pub columns: usize,
}

impl TableShape {
    /// Creates table shape metadata.
    pub fn new(rows: usize, columns: usize) -> Self {
        Self { rows, columns }
    }
}
