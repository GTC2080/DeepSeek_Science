#![forbid(unsafe_code)]
//! Chemistry-specific workflow contracts.
//!
//! Chemistry logic belongs in this crate so generic kernel crates stay
//! domain-neutral and must not depend on chemistry.
//!
//! Phase 2 currently defines deterministic, in-memory kinetics validation,
//! linearized fitting, MVP comparison, reviewer checks, and structured analysis
//! results for the future `chemistry.kinetics_csv` workflow. CSV parsing is
//! deferred to a later adapter and is not part of this crate boundary yet.

pub mod error;
pub mod kinetics;
pub mod kinetics_plot;

pub use error::KineticsError;
pub use kinetics::{
    kinetics_csv_workflow_plan, KineticsAnalysisResult, KineticsArtifactProposal, KineticsColumns,
    KineticsComparisonBasis, KineticsFitResult, KineticsModelComparison, KineticsModelKind,
    KineticsPoint, KineticsReview, KineticsReviewCheckKind, KineticsReviewFinding,
    KineticsReviewSeverity, KineticsReviewStatus, RejectedKineticsRow, RejectedKineticsRowReason,
    ValidatedKineticsInput, CHEMISTRY_KINETICS_CSV_WORKFLOW_ID,
};
pub use kinetics_plot::{
    KineticsCurveSegment, KineticsPlotData, KineticsPlotDataError, KineticsPlotModelData,
    KineticsPredictionPoint, KineticsVisualizationWarning,
};
