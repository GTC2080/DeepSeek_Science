#![forbid(unsafe_code)]
//! Chemistry-specific workflow contracts.
//!
//! Chemistry logic belongs in this crate so generic kernel crates stay
//! domain-neutral and must not depend on chemistry.
//!
//! Phase 2.2 only defines deterministic, in-memory kinetics input validation
//! for the future `chemistry.kinetics_csv` workflow. CSV parsing is deferred to
//! a later adapter and is not part of this crate boundary yet.

pub mod error;
pub mod kinetics;

pub use error::KineticsError;
pub use kinetics::{
    KineticsColumns, KineticsPoint, RejectedKineticsRow, RejectedKineticsRowReason,
    ValidatedKineticsInput, CHEMISTRY_KINETICS_CSV_WORKFLOW_ID,
};
