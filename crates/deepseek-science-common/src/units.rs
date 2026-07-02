//! Minimal unit labels for shared examples.

use serde::{Deserialize, Serialize};

/// Small placeholder unit enum for future typed quantities.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Unit {
    /// Unitless value.
    Dimensionless,
    /// Seconds.
    Second,
    /// Moles per liter.
    MolePerLiter,
}
