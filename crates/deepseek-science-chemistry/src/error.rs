//! Error types for chemistry workflow contracts.

use deepseek_science_artifacts::ArtifactError;
use deepseek_science_common::CommonError;
use deepseek_science_core::CoreError;
use thiserror::Error;

use crate::kinetics::KineticsModelKind;

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

    /// A fitting transform received an invalid concentration defensively.
    #[error("invalid concentration for {model_kind:?} transform at row {row_index}")]
    InvalidConcentrationForTransform {
        /// Kinetic model being transformed.
        model_kind: KineticsModelKind,
        /// Zero-based row index in the input table.
        row_index: usize,
    },

    /// A transformed value was NaN or infinity.
    #[error("non-finite transformed value for {model_kind:?} at row {row_index}")]
    NonFiniteTransformedValue {
        /// Kinetic model being transformed.
        model_kind: KineticsModelKind,
        /// Zero-based row index in the input table.
        row_index: usize,
    },

    /// Linear regression failed for transformed kinetics data.
    #[error("linear regression failed for {model_kind:?}: {source}")]
    RegressionFailed {
        /// Kinetic model being fitted.
        model_kind: KineticsModelKind,
        /// Regression error from shared numerical helpers.
        source: CommonError,
    },

    /// Regression returned a non-finite fit value.
    #[error("non-finite fit result for {model_kind:?}")]
    NonFiniteFitResult {
        /// Kinetic model being fitted.
        model_kind: KineticsModelKind,
    },

    /// Comparison received a non-finite MVP metric from a fit result.
    #[error("non-finite comparison metric for {model_kind:?}")]
    NonFiniteComparisonMetric {
        /// Kinetic model being compared.
        model_kind: KineticsModelKind,
    },

    /// Artifact mapping received a non-finite analysis value.
    #[error("non-finite artifact mapping value: {field}")]
    NonFiniteArtifactValue {
        /// Canonical field being encoded.
        field: &'static str,
    },

    /// Artifact proposal cannot be converted into a generic manifest.
    #[error("invalid artifact proposal field: {field}")]
    InvalidArtifactProposal {
        /// Invalid proposal field.
        field: &'static str,
    },

    /// The existing review finding count exceeded the artifact contract range.
    #[error("kinetics artifact review finding count exceeds the supported range")]
    ArtifactReviewFindingCountOverflow,

    /// Error raised while constructing generic in-memory artifact metadata.
    #[error(transparent)]
    Artifact(#[from] ArtifactError),

    /// Error raised by shared in-memory table contracts.
    #[error(transparent)]
    Common(#[from] CommonError),

    /// Error raised while constructing a generic workflow plan.
    #[error(transparent)]
    Core(#[from] CoreError),
}
