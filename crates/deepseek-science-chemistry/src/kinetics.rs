//! In-memory input validation for chemistry kinetics workflows.

use deepseek_science_common::{
    simple_linear_regression, ColumnName, CommonError, DataColumn, DataTable,
};
use deepseek_science_core::{
    WorkflowId, WorkflowPlan, WorkflowStepKey, WorkflowStepKind, WorkflowStepPlan,
};

use crate::error::KineticsError;

/// Workflow identifier reserved for the future chemistry kinetics CSV workflow.
///
/// The first implementation accepts an in-memory [`DataTable`]. The `_csv`
/// suffix names the expected future user-facing adapter, not file IO here.
pub const CHEMISTRY_KINETICS_CSV_WORKFLOW_ID: &str = "chemistry.kinetics_csv";

const MINIMUM_VALID_POINTS: usize = 2;
const REVIEW_TOLERANCE: f64 = 1.0e-12;

/// Returns the deterministic generic workflow plan for `chemistry.kinetics_csv`.
///
/// The plan is a pure in-memory description for future orchestration. It does
/// not validate tables, fit models, call models or tools, create artifacts,
/// persist storage, read files, or write files.
pub fn kinetics_csv_workflow_plan() -> Result<WorkflowPlan, KineticsError> {
    WorkflowPlan::new(
        WorkflowId::new(CHEMISTRY_KINETICS_CSV_WORKFLOW_ID)?,
        "Chemistry kinetics CSV",
        vec![
            workflow_step(
                "inspect_input",
                WorkflowStepKind::InspectInput,
                "Inspect input",
            )?,
            workflow_step(
                "validate_kinetics_input",
                WorkflowStepKind::Custom,
                "Validate kinetics input",
            )?,
            workflow_step(
                "fit_first_order",
                WorkflowStepKind::Custom,
                "Fit first-order model",
            )?,
            workflow_step(
                "fit_second_order",
                WorkflowStepKind::Custom,
                "Fit second-order model",
            )?,
            workflow_step("compare_models", WorkflowStepKind::Review, "Compare models")?,
            workflow_step("review_result", WorkflowStepKind::Review, "Review result")?,
            workflow_step(
                "produce_analysis_result",
                WorkflowStepKind::ProduceArtifact,
                "Produce analysis result",
            )?,
            workflow_step("complete", WorkflowStepKind::Complete, "Complete")?,
        ],
    )
    .map_err(KineticsError::from)
}

/// Supported deterministic linearized kinetics models.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KineticsModelKind {
    /// First-order model: `ln(concentration)` vs `time`.
    FirstOrder,
    /// Second-order model: `1 / concentration` vs `time`.
    SecondOrder,
}

/// Deterministic linearized fit result for one kinetics model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticsFitResult {
    /// Model fitted.
    pub model_kind: KineticsModelKind,
    /// Fitted linearized regression slope.
    pub slope: f64,
    /// Fitted linearized regression intercept.
    pub intercept: f64,
    /// Model-specific rate constant derived from the slope.
    pub rate_constant_k: f64,
    /// Coefficient of determination from the linearized regression.
    pub r_squared: f64,
    /// Number of validated input points used by the fit.
    pub valid_point_count: usize,
}

impl KineticsFitResult {
    /// Fits one deterministic linearized kinetics model over validated input.
    pub fn fit(
        input: &ValidatedKineticsInput,
        model_kind: KineticsModelKind,
    ) -> Result<Self, KineticsError> {
        if input.valid_count() < MINIMUM_VALID_POINTS {
            return Err(KineticsError::NotEnoughValidPoints {
                valid_count: input.valid_count(),
                minimum_required: MINIMUM_VALID_POINTS,
            });
        }

        let mut x_values = Vec::with_capacity(input.valid_count());
        let mut y_values = Vec::with_capacity(input.valid_count());

        for point in input.valid_points() {
            x_values.push(point.time);
            y_values.push(transformed_y(*point, model_kind)?);
        }

        let regression = simple_linear_regression(&x_values, &y_values)
            .map_err(|source| KineticsError::RegressionFailed { model_kind, source })?;
        let rate_constant_k = match model_kind {
            KineticsModelKind::FirstOrder => -regression.slope,
            KineticsModelKind::SecondOrder => regression.slope,
        };

        if [
            regression.slope,
            regression.intercept,
            regression.r_squared,
            rate_constant_k,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(KineticsError::NonFiniteFitResult { model_kind });
        }

        Ok(Self {
            model_kind,
            slope: regression.slope,
            intercept: regression.intercept,
            rate_constant_k,
            r_squared: regression.r_squared,
            valid_point_count: input.valid_count(),
        })
    }
}

/// Basis used by the deterministic MVP comparison summary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KineticsComparisonBasis {
    /// Compare finite `r_squared` values only as an MVP heuristic.
    FiniteRSquaredMvpHeuristic,
}

/// Cautious comparison summary for the two MVP linearized kinetics fits.
///
/// This is not definitive scientific model selection. It records which model
/// is preferred by the finite `r_squared` MVP heuristic. Exact ties prefer
/// [`KineticsModelKind::FirstOrder`] for deterministic replay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticsModelComparison {
    /// First-order linearized fit result.
    pub first_order: KineticsFitResult,
    /// Second-order linearized fit result.
    pub second_order: KineticsFitResult,
    /// Model preferred by the MVP comparison heuristic.
    pub preferred_model: KineticsModelKind,
    /// Comparison basis used to derive the preference.
    pub basis: KineticsComparisonBasis,
}

impl KineticsModelComparison {
    /// Fits and compares first-order and second-order MVP linearized models.
    pub fn from_input(input: &ValidatedKineticsInput) -> Result<Self, KineticsError> {
        let first_order = KineticsFitResult::fit(input, KineticsModelKind::FirstOrder)?;
        let second_order = KineticsFitResult::fit(input, KineticsModelKind::SecondOrder)?;
        let first_r_squared = comparison_metric(first_order)?;
        let second_r_squared = comparison_metric(second_order)?;
        let preferred_model = if second_r_squared > first_r_squared {
            KineticsModelKind::SecondOrder
        } else {
            KineticsModelKind::FirstOrder
        };

        Ok(Self {
            first_order,
            second_order,
            preferred_model,
            basis: KineticsComparisonBasis::FiniteRSquaredMvpHeuristic,
        })
    }
}

/// Deterministic reviewer status for kinetics comparison checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KineticsReviewStatus {
    /// All deterministic checks passed without findings.
    Passed,
    /// Checks passed with non-fatal warnings.
    PassedWithWarnings,
    /// One or more deterministic consistency checks failed.
    Failed,
}

/// Severity of a deterministic kinetics review finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KineticsReviewSeverity {
    /// Non-fatal issue that should remain visible to callers.
    Warning,
    /// Internal consistency failure.
    Error,
}

/// Deterministic check represented by a kinetics review finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KineticsReviewCheckKind {
    /// Rate constant follows the model-specific slope convention.
    RateConstantMatchesSlope,
    /// Fit metrics are finite.
    FiniteMetrics,
    /// Rejected rows remain visible in the review.
    RejectedRowsVisible,
    /// Comparison basis is the finite `r_squared` MVP heuristic.
    ComparisonBasisIsHeuristic,
}

/// Structured finding from deterministic kinetics review checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KineticsReviewFinding {
    /// Finding severity.
    pub severity: KineticsReviewSeverity,
    /// Deterministic check that produced the finding.
    pub check_kind: KineticsReviewCheckKind,
    /// Model associated with the finding, when model-specific.
    pub model_kind: Option<KineticsModelKind>,
    /// Rejected row count associated with the finding, when relevant.
    pub rejected_row_count: Option<usize>,
    /// Short stable explanation for the finding.
    pub message: &'static str,
}

/// Deterministic review of an in-memory kinetics comparison.
///
/// This review checks internal consistency only. It does not make definitive
/// scientific model-selection claims and does not call models, tools, files,
/// storage, or artifacts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KineticsReview {
    /// Overall deterministic review status.
    pub status: KineticsReviewStatus,
    /// Structured warnings or errors found by the reviewer.
    pub findings: Vec<KineticsReviewFinding>,
    /// Number of deterministic checks performed.
    pub checks_performed: usize,
    /// Rejected row count preserved from the validated input.
    pub rejected_row_count: usize,
}

impl KineticsReview {
    /// Reviews an existing comparison without rerunning fitting.
    pub fn from_input_and_comparison(
        input: &ValidatedKineticsInput,
        comparison: &KineticsModelComparison,
    ) -> Self {
        let mut findings = Vec::new();
        let mut checks_performed = 0;

        review_fit_metrics(comparison.first_order, &mut findings);
        checks_performed += 1;
        review_fit_metrics(comparison.second_order, &mut findings);
        checks_performed += 1;
        review_rate_constant(comparison.first_order, &mut findings);
        checks_performed += 1;
        review_rate_constant(comparison.second_order, &mut findings);
        checks_performed += 1;
        review_comparison_basis(comparison.basis, &mut findings);
        checks_performed += 1;
        review_rejected_rows(input.rejected_count(), &mut findings);
        checks_performed += 1;

        let status = review_status(&findings);

        Self {
            status,
            findings,
            checks_performed,
            rejected_row_count: input.rejected_count(),
        }
    }
}

/// Structured in-memory kinetics analysis result.
///
/// This type composes validated input metadata, MVP linearized model
/// comparison, and deterministic reviewer output. The preferred model is only
/// the Phase 2 finite `r_squared` heuristic preference, not a definitive
/// reaction-order determination.
#[derive(Clone, Debug, PartialEq)]
pub struct KineticsAnalysisResult {
    /// First-order and second-order comparison summary.
    pub comparison: KineticsModelComparison,
    /// Deterministic consistency review for the comparison.
    pub review: KineticsReview,
    /// Count of validated positive-concentration input points.
    pub valid_point_count: usize,
    /// Count of rows rejected during input validation.
    pub rejected_row_count: usize,
    /// Model preferred by the MVP comparison heuristic.
    pub preferred_model: KineticsModelKind,
    /// Basis used for the MVP comparison preference.
    pub comparison_basis: KineticsComparisonBasis,
}

impl KineticsAnalysisResult {
    /// Runs deterministic in-memory kinetics analysis over validated input.
    ///
    /// This does not parse CSV, read or write files, call models or tools,
    /// create artifacts, persist storage, or generate prose reports.
    pub fn analyze(input: &ValidatedKineticsInput) -> Result<Self, KineticsError> {
        let comparison = KineticsModelComparison::from_input(input)?;
        let review = KineticsReview::from_input_and_comparison(input, &comparison);

        Ok(Self {
            valid_point_count: input.valid_count(),
            rejected_row_count: input.rejected_count(),
            preferred_model: comparison.preferred_model,
            comparison_basis: comparison.basis,
            comparison,
            review,
        })
    }

    /// Returns the number of valid points analyzed.
    pub fn valid_point_count(&self) -> usize {
        self.valid_point_count
    }

    /// Returns the number of rejected input rows.
    pub fn rejected_row_count(&self) -> usize {
        self.rejected_row_count
    }

    /// Returns the model preferred by the MVP heuristic.
    pub fn preferred_model(&self) -> KineticsModelKind {
        self.preferred_model
    }

    /// Returns the comparison basis used for the MVP preference.
    pub fn comparison_basis(&self) -> KineticsComparisonBasis {
        self.comparison_basis
    }

    /// Returns the deterministic review status.
    pub fn review_status(&self) -> KineticsReviewStatus {
        self.review.status
    }

    /// Returns whether the deterministic review contains warnings.
    pub fn has_warnings(&self) -> bool {
        self.review
            .findings
            .iter()
            .any(|finding| finding.severity == KineticsReviewSeverity::Warning)
    }
}

/// Exact caller-provided column names for kinetics input data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KineticsColumns {
    time: ColumnName,
    concentration: ColumnName,
}

impl KineticsColumns {
    /// Creates exact column bindings for time and concentration.
    ///
    /// No fuzzy matching, case folding, or semantic detection is performed.
    pub fn new(
        time: impl Into<String>,
        concentration: impl Into<String>,
    ) -> Result<Self, KineticsError> {
        Ok(Self {
            time: ColumnName::new(time)?,
            concentration: ColumnName::new(concentration)?,
        })
    }

    /// Returns the exact time column name.
    pub fn time(&self) -> &ColumnName {
        &self.time
    }

    /// Returns the exact concentration column name.
    pub fn concentration(&self) -> &ColumnName {
        &self.concentration
    }
}

/// One validated kinetics observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticsPoint {
    /// Zero-based row index in the input [`DataTable`].
    pub row_index: usize,
    /// Time value from the caller-selected time column.
    pub time: f64,
    /// Positive concentration value from the caller-selected concentration column.
    pub concentration: f64,
}

/// Reason a kinetics input row was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectedKineticsRowReason {
    /// Concentration was less than or equal to zero.
    NonPositiveConcentration,
}

/// One rejected kinetics input row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RejectedKineticsRow {
    /// Zero-based row index in the input [`DataTable`].
    pub row_index: usize,
    /// Deterministic rejection reason.
    pub reason: RejectedKineticsRowReason,
}

/// Validated in-memory kinetics input for later deterministic workflows.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedKineticsInput {
    valid_points: Vec<KineticsPoint>,
    rejected_rows: Vec<RejectedKineticsRow>,
}

impl ValidatedKineticsInput {
    /// Validates caller-selected columns from an in-memory table.
    ///
    /// This does not parse CSV, read files, fit kinetic models, or create
    /// artifacts. It only preserves valid positive-concentration rows and
    /// records rejected row indices.
    pub fn from_table(table: &DataTable, columns: &KineticsColumns) -> Result<Self, KineticsError> {
        let time = time_column(table, columns.time())?;
        let concentration = concentration_column(table, columns.concentration())?;

        let mut valid_points = Vec::with_capacity(table.row_count());
        let mut rejected_rows = Vec::new();

        for (row_index, (&time, &concentration)) in time
            .values()
            .iter()
            .zip(concentration.values().iter())
            .enumerate()
        {
            if concentration <= 0.0 {
                rejected_rows.push(RejectedKineticsRow {
                    row_index,
                    reason: RejectedKineticsRowReason::NonPositiveConcentration,
                });
                continue;
            }

            valid_points.push(KineticsPoint {
                row_index,
                time,
                concentration,
            });
        }

        if valid_points.len() < MINIMUM_VALID_POINTS {
            return Err(KineticsError::NotEnoughValidPoints {
                valid_count: valid_points.len(),
                minimum_required: MINIMUM_VALID_POINTS,
            });
        }

        Ok(Self {
            valid_points,
            rejected_rows,
        })
    }

    /// Returns accepted points in caller row order.
    pub fn valid_points(&self) -> &[KineticsPoint] {
        &self.valid_points
    }

    /// Returns rejected rows in caller row order.
    pub fn rejected_rows(&self) -> &[RejectedKineticsRow] {
        &self.rejected_rows
    }

    /// Returns the accepted point count.
    pub fn valid_count(&self) -> usize {
        self.valid_points.len()
    }

    /// Returns the rejected row count.
    pub fn rejected_count(&self) -> usize {
        self.rejected_rows.len()
    }
}

fn time_column<'a>(
    table: &'a DataTable,
    name: &ColumnName,
) -> Result<&'a DataColumn, KineticsError> {
    table
        .numeric_column(name.as_str())
        .map_err(|error| match error {
            CommonError::MissingColumn { name } => KineticsError::MissingTimeColumn { name },
            error => KineticsError::Common(error),
        })
}

fn concentration_column<'a>(
    table: &'a DataTable,
    name: &ColumnName,
) -> Result<&'a DataColumn, KineticsError> {
    table
        .numeric_column(name.as_str())
        .map_err(|error| match error {
            CommonError::MissingColumn { name } => {
                KineticsError::MissingConcentrationColumn { name }
            }
            error => KineticsError::Common(error),
        })
}

fn transformed_y(
    point: KineticsPoint,
    model_kind: KineticsModelKind,
) -> Result<f64, KineticsError> {
    if point.concentration <= 0.0 {
        return Err(KineticsError::InvalidConcentrationForTransform {
            model_kind,
            row_index: point.row_index,
        });
    }

    let transformed = match model_kind {
        KineticsModelKind::FirstOrder => point.concentration.ln(),
        KineticsModelKind::SecondOrder => 1.0 / point.concentration,
    };

    if !transformed.is_finite() {
        return Err(KineticsError::NonFiniteTransformedValue {
            model_kind,
            row_index: point.row_index,
        });
    }

    Ok(transformed)
}

fn comparison_metric(fit: KineticsFitResult) -> Result<f64, KineticsError> {
    if !fit.r_squared.is_finite() || !fit.rate_constant_k.is_finite() {
        return Err(KineticsError::NonFiniteComparisonMetric {
            model_kind: fit.model_kind,
        });
    }

    Ok(fit.r_squared)
}

fn review_fit_metrics(fit: KineticsFitResult, findings: &mut Vec<KineticsReviewFinding>) {
    if [fit.slope, fit.intercept, fit.rate_constant_k, fit.r_squared]
        .iter()
        .any(|value| !value.is_finite())
    {
        findings.push(KineticsReviewFinding {
            severity: KineticsReviewSeverity::Error,
            check_kind: KineticsReviewCheckKind::FiniteMetrics,
            model_kind: Some(fit.model_kind),
            rejected_row_count: None,
            message: "fit metrics must be finite",
        });
    }
}

fn review_rate_constant(fit: KineticsFitResult, findings: &mut Vec<KineticsReviewFinding>) {
    let expected = match fit.model_kind {
        KineticsModelKind::FirstOrder => -fit.slope,
        KineticsModelKind::SecondOrder => fit.slope,
    };

    if (fit.rate_constant_k - expected).abs() > REVIEW_TOLERANCE {
        findings.push(KineticsReviewFinding {
            severity: KineticsReviewSeverity::Error,
            check_kind: KineticsReviewCheckKind::RateConstantMatchesSlope,
            model_kind: Some(fit.model_kind),
            rejected_row_count: None,
            message: "rate constant must match the model slope convention",
        });
    }
}

fn review_comparison_basis(
    basis: KineticsComparisonBasis,
    findings: &mut Vec<KineticsReviewFinding>,
) {
    if basis != KineticsComparisonBasis::FiniteRSquaredMvpHeuristic {
        findings.push(KineticsReviewFinding {
            severity: KineticsReviewSeverity::Error,
            check_kind: KineticsReviewCheckKind::ComparisonBasisIsHeuristic,
            model_kind: None,
            rejected_row_count: None,
            message: "comparison basis must remain the MVP r_squared heuristic",
        });
    }
}

fn review_rejected_rows(rejected_count: usize, findings: &mut Vec<KineticsReviewFinding>) {
    if rejected_count > 0 {
        findings.push(KineticsReviewFinding {
            severity: KineticsReviewSeverity::Warning,
            check_kind: KineticsReviewCheckKind::RejectedRowsVisible,
            model_kind: None,
            rejected_row_count: Some(rejected_count),
            message: "rejected rows are present and remain visible",
        });
    }
}

fn review_status(findings: &[KineticsReviewFinding]) -> KineticsReviewStatus {
    if findings
        .iter()
        .any(|finding| finding.severity == KineticsReviewSeverity::Error)
    {
        KineticsReviewStatus::Failed
    } else if findings
        .iter()
        .any(|finding| finding.severity == KineticsReviewSeverity::Warning)
    {
        KineticsReviewStatus::PassedWithWarnings
    } else {
        KineticsReviewStatus::Passed
    }
}

fn workflow_step(
    key: &str,
    kind: WorkflowStepKind,
    label: &str,
) -> Result<WorkflowStepPlan, KineticsError> {
    WorkflowStepPlan::new(WorkflowStepKey::new(key)?, kind, label, None)
        .map_err(KineticsError::from)
}

#[cfg(test)]
mod tests {
    use deepseek_science_common::{DataColumn, DataTable};
    use deepseek_science_core::WorkflowStepKind;

    use crate::{
        kinetics_csv_workflow_plan, KineticsAnalysisResult, KineticsColumns,
        KineticsComparisonBasis, KineticsError, KineticsFitResult, KineticsModelComparison,
        KineticsModelKind, KineticsReview, KineticsReviewCheckKind, KineticsReviewSeverity,
        KineticsReviewStatus, RejectedKineticsRowReason, ValidatedKineticsInput,
        CHEMISTRY_KINETICS_CSV_WORKFLOW_ID,
    };

    fn numeric_column(name: &str, values: &[f64]) -> DataColumn {
        DataColumn::numeric(name, values.to_vec()).expect("test column should be valid")
    }

    fn kinetics_table(time: &[f64], concentration: &[f64]) -> DataTable {
        DataTable::new(vec![
            numeric_column("time", time),
            numeric_column("concentration", concentration),
        ])
        .expect("test table should be valid")
    }

    fn kinetics_columns() -> KineticsColumns {
        KineticsColumns::new("time", "concentration").expect("test columns should be valid")
    }

    fn validated_input(time: &[f64], concentration: &[f64]) -> ValidatedKineticsInput {
        let table = kinetics_table(time, concentration);
        ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("test input should be valid")
    }

    fn assert_near(actual: f64, expected: f64) {
        let tolerance = 1.0e-12;
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}"
        );
    }

    #[test]
    fn exposes_workflow_id() {
        assert_eq!(CHEMISTRY_KINETICS_CSV_WORKFLOW_ID, "chemistry.kinetics_csv");
    }

    #[test]
    fn kinetics_columns_preserve_exact_names() {
        let columns = KineticsColumns::new("time_s", "substrate_mmol_l")
            .expect("column names should be valid");

        assert_eq!(columns.time().as_str(), "time_s");
        assert_eq!(columns.concentration().as_str(), "substrate_mmol_l");
    }

    #[test]
    fn column_matching_is_exact_and_case_sensitive() {
        let table = kinetics_table(&[0.0, 1.0], &[2.0, 1.0]);
        let columns =
            KineticsColumns::new("Time", "concentration").expect("column names should be valid");

        let result = ValidatedKineticsInput::from_table(&table, &columns);

        assert_eq!(
            result,
            Err(KineticsError::MissingTimeColumn {
                name: "Time".to_string()
            })
        );
    }

    #[test]
    fn extracts_valid_points_from_data_table() {
        let table = kinetics_table(&[0.0, 1.0, 2.0], &[3.0, 2.0, 1.0]);
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("input should be valid");

        assert_eq!(input.valid_count(), 3);
        assert_eq!(input.rejected_count(), 0);
        assert_eq!(input.valid_points()[0].time, 0.0);
        assert_eq!(input.valid_points()[0].concentration, 3.0);
    }

    #[test]
    fn valid_points_preserve_zero_based_row_indices_and_order() {
        let table = kinetics_table(&[3.0, 1.0, 2.0], &[8.0, 4.0, 2.0]);
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("input should be valid");

        let points = input.valid_points();
        assert_eq!(points[0].row_index, 0);
        assert_eq!(points[0].time, 3.0);
        assert_eq!(points[1].row_index, 1);
        assert_eq!(points[1].time, 1.0);
        assert_eq!(points[2].row_index, 2);
        assert_eq!(points[2].time, 2.0);
    }

    #[test]
    fn rejects_non_positive_concentration_rows() {
        let table = kinetics_table(&[0.0, 1.0, 2.0, 3.0], &[3.0, 0.0, -1.0, 1.0]);
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("two positive concentrations should remain valid");

        assert_eq!(input.valid_count(), 2);
        assert_eq!(input.rejected_count(), 2);
        assert_eq!(input.valid_points()[0].row_index, 0);
        assert_eq!(input.valid_points()[1].row_index, 3);
        assert_eq!(input.rejected_rows()[0].row_index, 1);
        assert_eq!(input.rejected_rows()[1].row_index, 2);
    }

    #[test]
    fn rejected_reason_is_non_positive_concentration() {
        let table = kinetics_table(&[0.0, 1.0, 2.0], &[3.0, 0.0, 1.0]);
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("two positive concentrations should remain valid");

        assert_eq!(
            input.rejected_rows()[0].reason,
            RejectedKineticsRowReason::NonPositiveConcentration
        );
    }

    #[test]
    fn fewer_than_two_valid_points_returns_structured_error() {
        let table = kinetics_table(&[0.0, 1.0, 2.0], &[0.0, -1.0, 1.0]);
        let result = ValidatedKineticsInput::from_table(&table, &kinetics_columns());

        assert_eq!(
            result,
            Err(KineticsError::NotEnoughValidPoints {
                valid_count: 1,
                minimum_required: 2
            })
        );
    }

    #[test]
    fn missing_time_column_returns_structured_error() {
        let table = DataTable::new(vec![
            numeric_column("elapsed", &[0.0, 1.0]),
            numeric_column("concentration", &[2.0, 1.0]),
        ])
        .expect("test table should be valid");
        let result = ValidatedKineticsInput::from_table(&table, &kinetics_columns());

        assert_eq!(
            result,
            Err(KineticsError::MissingTimeColumn {
                name: "time".to_string()
            })
        );
    }

    #[test]
    fn missing_concentration_column_returns_structured_error() {
        let table = DataTable::new(vec![
            numeric_column("time", &[0.0, 1.0]),
            numeric_column("signal", &[2.0, 1.0]),
        ])
        .expect("test table should be valid");
        let result = ValidatedKineticsInput::from_table(&table, &kinetics_columns());

        assert_eq!(
            result,
            Err(KineticsError::MissingConcentrationColumn {
                name: "concentration".to_string()
            })
        );
    }

    #[test]
    fn first_order_fit_recovers_expected_rate_constant_for_exact_data() {
        let k = 0.5;
        let input = validated_input(
            &[0.0, 1.0, 2.0, 3.0],
            &[
                1.0,
                (-k * 1.0_f64).exp(),
                (-k * 2.0_f64).exp(),
                (-k * 3.0_f64).exp(),
            ],
        );

        let fit = KineticsFitResult::fit(&input, KineticsModelKind::FirstOrder)
            .expect("first-order fit should succeed");

        assert_eq!(fit.model_kind, KineticsModelKind::FirstOrder);
        assert_near(fit.rate_constant_k, k);
        assert_eq!(fit.valid_point_count, 4);
    }

    #[test]
    fn second_order_fit_recovers_expected_rate_constant_for_exact_data() {
        let intercept = 0.5;
        let k = 0.25;
        let input = validated_input(
            &[0.0, 1.0, 2.0, 3.0],
            &[
                1.0 / (intercept + k * 0.0),
                1.0 / (intercept + k * 1.0),
                1.0 / (intercept + k * 2.0),
                1.0 / (intercept + k * 3.0),
            ],
        );

        let fit = KineticsFitResult::fit(&input, KineticsModelKind::SecondOrder)
            .expect("second-order fit should succeed");

        assert_eq!(fit.model_kind, KineticsModelKind::SecondOrder);
        assert_near(fit.rate_constant_k, k);
        assert_eq!(fit.valid_point_count, 4);
    }

    #[test]
    fn first_order_rate_constant_is_negative_slope() {
        let input = validated_input(&[0.0, 1.0, 2.0], &[1.0, 0.5_f64.exp(), 1.0_f64.exp()]);

        let fit = KineticsFitResult::fit(&input, KineticsModelKind::FirstOrder)
            .expect("first-order fit should succeed");

        assert_near(fit.slope, 0.5);
        assert_near(fit.rate_constant_k, -fit.slope);
    }

    #[test]
    fn second_order_rate_constant_is_slope() {
        let input = validated_input(&[0.0, 1.0, 2.0], &[1.0, 1.0 / 1.5, 0.5]);

        let fit = KineticsFitResult::fit(&input, KineticsModelKind::SecondOrder)
            .expect("second-order fit should succeed");

        assert_near(fit.slope, 0.5);
        assert_near(fit.rate_constant_k, fit.slope);
    }

    #[test]
    fn r_squared_is_finite_and_near_one_for_exact_linearized_data() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );

        let fit = KineticsFitResult::fit(&input, KineticsModelKind::FirstOrder)
            .expect("first-order fit should succeed");

        assert!(fit.r_squared.is_finite());
        assert_near(fit.r_squared, 1.0);
    }

    #[test]
    fn fit_uses_only_valid_points_after_non_positive_concentration_rejection() {
        let table = kinetics_table(
            &[0.0, 99.0, 1.0, 2.0],
            &[1.0, 0.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("two valid positive concentrations should remain");

        let fit = KineticsFitResult::fit(&input, KineticsModelKind::FirstOrder)
            .expect("first-order fit should ignore rejected rows");

        assert_eq!(input.rejected_count(), 1);
        assert_eq!(input.rejected_rows()[0].row_index, 1);
        assert_eq!(fit.valid_point_count, 3);
        assert_near(fit.rate_constant_k, 0.25);
    }

    #[test]
    fn fit_requires_at_least_two_valid_points() {
        let table = kinetics_table(&[0.0, 1.0, 2.0], &[0.0, -1.0, 1.0]);

        let result = ValidatedKineticsInput::from_table(&table, &kinetics_columns());

        assert_eq!(
            result,
            Err(KineticsError::NotEnoughValidPoints {
                valid_count: 1,
                minimum_required: 2
            })
        );
    }

    #[test]
    fn repeated_fit_with_same_input_is_deterministic() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );

        let first = KineticsFitResult::fit(&input, KineticsModelKind::FirstOrder)
            .expect("first fit should succeed");
        let second = KineticsFitResult::fit(&input, KineticsModelKind::FirstOrder)
            .expect("second fit should succeed");

        assert_eq!(first, second);
    }

    #[test]
    fn fitting_uses_in_memory_table_without_csv_or_file_io() {
        let input = validated_input(&[0.0, 1.0], &[1.0, (-0.25_f64).exp()]);

        let fit = KineticsFitResult::fit(&input, KineticsModelKind::FirstOrder)
            .expect("in-memory fit should succeed");

        assert_near(fit.rate_constant_k, 0.25);
    }

    #[test]
    fn fit_result_does_not_select_a_model_comparison_winner() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );

        let first = KineticsFitResult::fit(&input, KineticsModelKind::FirstOrder)
            .expect("first-order fit should succeed");
        let second = KineticsFitResult::fit(&input, KineticsModelKind::SecondOrder)
            .expect("second-order fit should succeed");

        assert_eq!(first.model_kind, KineticsModelKind::FirstOrder);
        assert_eq!(second.model_kind, KineticsModelKind::SecondOrder);
    }

    #[test]
    fn comparison_fits_both_first_order_and_second_order_models() {
        let input = validated_input(
            &[0.0, 1.0, 2.0, 3.0],
            &[1.0, (-0.5_f64).exp(), (-1.0_f64).exp(), (-1.5_f64).exp()],
        );

        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should fit both models");

        assert_eq!(
            comparison.first_order.model_kind,
            KineticsModelKind::FirstOrder
        );
        assert_eq!(
            comparison.second_order.model_kind,
            KineticsModelKind::SecondOrder
        );
    }

    #[test]
    fn comparison_prefers_first_order_when_first_order_r_squared_is_higher() {
        let input = validated_input(
            &[0.0, 1.0, 2.0, 3.0],
            &[1.0, (-0.5_f64).exp(), (-1.0_f64).exp(), (-1.5_f64).exp()],
        );

        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        assert_eq!(comparison.preferred_model, KineticsModelKind::FirstOrder);
        assert!(
            comparison.first_order.r_squared > comparison.second_order.r_squared,
            "first-order r_squared should drive the MVP preference"
        );
    }

    #[test]
    fn comparison_prefers_second_order_when_second_order_r_squared_is_higher() {
        let input = validated_input(&[0.0, 1.0, 2.0, 3.0], &[2.0, 1.0 / 0.75, 1.0, 1.0 / 1.25]);

        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        assert_eq!(comparison.preferred_model, KineticsModelKind::SecondOrder);
        assert!(
            comparison.second_order.r_squared > comparison.first_order.r_squared,
            "second-order r_squared should drive the MVP preference"
        );
    }

    #[test]
    fn comparison_tie_behavior_prefers_first_order_deterministically() {
        let input = validated_input(&[0.0, 1.0], &[1.0, 0.5]);

        let comparison = KineticsModelComparison::from_input(&input)
            .expect("two-point comparison should succeed");

        assert_near(
            comparison.first_order.r_squared,
            comparison.second_order.r_squared,
        );
        assert_eq!(comparison.preferred_model, KineticsModelKind::FirstOrder);
    }

    #[test]
    fn selected_model_is_based_on_finite_r_squared_mvp_heuristic_only() {
        let input = validated_input(&[0.0, 1.0, 2.0, 3.0], &[2.0, 1.0 / 0.75, 1.0, 1.0 / 1.25]);

        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        assert_eq!(
            comparison.basis,
            KineticsComparisonBasis::FiniteRSquaredMvpHeuristic
        );
        assert!(comparison.first_order.r_squared.is_finite());
        assert!(comparison.second_order.r_squared.is_finite());
        assert_eq!(comparison.preferred_model, KineticsModelKind::SecondOrder);
    }

    #[test]
    fn comparison_preserves_access_to_both_fit_results() {
        let input = validated_input(
            &[0.0, 1.0, 2.0, 3.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp(), (-0.75_f64).exp()],
        );

        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        assert_near(comparison.first_order.rate_constant_k, 0.25);
        assert_eq!(
            comparison.second_order.model_kind,
            KineticsModelKind::SecondOrder
        );
    }

    #[test]
    fn comparison_public_names_remain_cautious() {
        let comparison_name = "KineticsModelComparison";
        let basis_name = format!("{:?}", KineticsComparisonBasis::FiniteRSquaredMvpHeuristic);

        assert!(basis_name.contains("Mvp"));
        assert!(basis_name.contains("Heuristic"));
        for name in [comparison_name, basis_name.as_str()] {
            assert!(!name.contains("True"));
            assert!(!name.contains("Best"));
            assert!(!name.contains("Final"));
            assert!(!name.contains("Prove"));
            assert!(!name.contains("Determine"));
        }
    }

    #[test]
    fn repeated_comparison_with_same_input_is_deterministic() {
        let input = validated_input(
            &[0.0, 1.0, 2.0, 3.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp(), (-0.75_f64).exp()],
        );

        let first =
            KineticsModelComparison::from_input(&input).expect("first comparison should succeed");
        let second =
            KineticsModelComparison::from_input(&input).expect("second comparison should succeed");

        assert_eq!(first, second);
    }

    #[test]
    fn comparison_uses_in_memory_table_without_csv_or_file_io() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );

        let comparison = KineticsModelComparison::from_input(&input)
            .expect("in-memory comparison should succeed");

        assert_eq!(comparison.first_order.valid_point_count, 3);
        assert_eq!(comparison.second_order.valid_point_count, 3);
    }

    #[test]
    fn reviewer_passes_for_clean_exact_first_order_like_comparison() {
        let input = validated_input(
            &[0.0, 1.0, 2.0, 3.0],
            &[1.0, (-0.5_f64).exp(), (-1.0_f64).exp(), (-1.5_f64).exp()],
        );
        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        let review = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(review.status, KineticsReviewStatus::Passed);
        assert!(review.findings.is_empty());
        assert!(review.checks_performed >= 5);
    }

    #[test]
    fn reviewer_passes_for_clean_exact_second_order_like_comparison() {
        let input = validated_input(&[0.0, 1.0, 2.0, 3.0], &[2.0, 1.0 / 0.75, 1.0, 1.0 / 1.25]);
        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        let review = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(comparison.preferred_model, KineticsModelKind::SecondOrder);
        assert_eq!(review.status, KineticsReviewStatus::Passed);
        assert!(review.findings.is_empty());
    }

    #[test]
    fn reviewer_reports_finite_metrics_check() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );
        let mut comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");
        comparison.first_order.r_squared = f64::NAN;

        let review = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(review.status, KineticsReviewStatus::Failed);
        assert!(review.findings.iter().any(|finding| {
            finding.severity == KineticsReviewSeverity::Error
                && finding.check_kind == KineticsReviewCheckKind::FiniteMetrics
                && finding.model_kind == Some(KineticsModelKind::FirstOrder)
        }));
    }

    #[test]
    fn reviewer_verifies_first_order_rate_constant_matches_negative_slope() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );
        let mut comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");
        comparison.first_order.rate_constant_k = comparison.first_order.slope;

        let review = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(review.status, KineticsReviewStatus::Failed);
        assert!(review.findings.iter().any(|finding| {
            finding.severity == KineticsReviewSeverity::Error
                && finding.check_kind == KineticsReviewCheckKind::RateConstantMatchesSlope
                && finding.model_kind == Some(KineticsModelKind::FirstOrder)
        }));
    }

    #[test]
    fn reviewer_verifies_second_order_rate_constant_matches_slope() {
        let input = validated_input(&[0.0, 1.0, 2.0], &[2.0, 1.0, 2.0 / 3.0]);
        let mut comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");
        comparison.second_order.rate_constant_k = -comparison.second_order.slope;

        let review = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(review.status, KineticsReviewStatus::Failed);
        assert!(review.findings.iter().any(|finding| {
            finding.severity == KineticsReviewSeverity::Error
                && finding.check_kind == KineticsReviewCheckKind::RateConstantMatchesSlope
                && finding.model_kind == Some(KineticsModelKind::SecondOrder)
        }));
    }

    #[test]
    fn reviewer_reports_warning_when_rejected_rows_exist() {
        let table = kinetics_table(
            &[0.0, 99.0, 1.0, 2.0],
            &[1.0, 0.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("two valid positive concentrations should remain");
        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        let review = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(review.status, KineticsReviewStatus::PassedWithWarnings);
        assert!(review.findings.iter().any(|finding| {
            finding.severity == KineticsReviewSeverity::Warning
                && finding.check_kind == KineticsReviewCheckKind::RejectedRowsVisible
        }));
    }

    #[test]
    fn reviewer_preserves_rejected_row_count_visibility() {
        let table = kinetics_table(
            &[0.0, 99.0, 1.0, 100.0, 2.0],
            &[1.0, 0.0, (-0.25_f64).exp(), -1.0, (-0.5_f64).exp()],
        );
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("three valid positive concentrations should remain");
        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        let review = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(review.rejected_row_count, 2);
        assert!(review
            .findings
            .iter()
            .any(|finding| finding.rejected_row_count == Some(2)));
    }

    #[test]
    fn reviewer_status_becomes_passed_with_warnings_when_warnings_exist() {
        let table = kinetics_table(&[0.0, 99.0, 1.0], &[1.0, 0.0, (-0.25_f64).exp()]);
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("two valid positive concentrations should remain");
        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        let review = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(review.status, KineticsReviewStatus::PassedWithWarnings);
    }

    #[test]
    fn reviewer_remains_deterministic_for_repeated_calls() {
        let input = validated_input(
            &[0.0, 1.0, 2.0, 3.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp(), (-0.75_f64).exp()],
        );
        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        let first = KineticsReview::from_input_and_comparison(&input, &comparison);
        let second = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(first, second);
    }

    #[test]
    fn reviewer_uses_in_memory_data_without_side_effects() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );
        let comparison =
            KineticsModelComparison::from_input(&input).expect("comparison should succeed");

        let review = KineticsReview::from_input_and_comparison(&input, &comparison);

        assert_eq!(review.status, KineticsReviewStatus::Passed);
        assert_eq!(review.rejected_row_count, 0);
    }

    #[test]
    fn reviewer_public_names_remain_non_definitive() {
        let names = [
            "KineticsReview",
            "KineticsReviewStatus",
            "KineticsReviewFinding",
            "KineticsReviewCheckKind",
        ];

        for name in names {
            assert!(!name.contains("True"));
            assert!(!name.contains("Best"));
            assert!(!name.contains("Final"));
            assert!(!name.contains("Prove"));
            assert!(!name.contains("Determine"));
        }
    }

    #[test]
    fn analysis_result_can_be_created_from_valid_input() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );

        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");

        assert_eq!(analysis.valid_point_count(), 3);
    }

    #[test]
    fn analysis_result_includes_comparison_result() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );

        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");

        assert_eq!(
            analysis.comparison.first_order.model_kind,
            KineticsModelKind::FirstOrder
        );
        assert_eq!(
            analysis.comparison.second_order.model_kind,
            KineticsModelKind::SecondOrder
        );
    }

    #[test]
    fn analysis_result_includes_review_result() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );

        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");

        assert_eq!(analysis.review.status, KineticsReviewStatus::Passed);
        assert!(analysis.review.findings.is_empty());
    }

    #[test]
    fn analysis_result_preserves_valid_and_rejected_counts() {
        let table = kinetics_table(
            &[0.0, 99.0, 1.0, 2.0],
            &[1.0, 0.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("three valid points should remain");

        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");

        assert_eq!(analysis.valid_point_count(), 3);
        assert_eq!(analysis.rejected_row_count(), 1);
    }

    #[test]
    fn analysis_result_exposes_preferred_model_and_basis() {
        let input = validated_input(&[0.0, 1.0, 2.0, 3.0], &[2.0, 1.0 / 0.75, 1.0, 1.0 / 1.25]);

        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");

        assert_eq!(analysis.preferred_model(), KineticsModelKind::SecondOrder);
        assert_eq!(
            analysis.comparison_basis,
            KineticsComparisonBasis::FiniteRSquaredMvpHeuristic
        );
    }

    #[test]
    fn analysis_result_exposes_review_status() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );

        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");

        assert_eq!(analysis.review_status(), KineticsReviewStatus::Passed);
        assert!(!analysis.has_warnings());
    }

    #[test]
    fn analysis_result_detects_warnings_when_rejected_rows_exist() {
        let table = kinetics_table(&[0.0, 99.0, 1.0], &[1.0, 0.0, (-0.25_f64).exp()]);
        let input = ValidatedKineticsInput::from_table(&table, &kinetics_columns())
            .expect("two valid positive concentrations should remain");

        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");

        assert_eq!(
            analysis.review_status(),
            KineticsReviewStatus::PassedWithWarnings
        );
        assert!(analysis.has_warnings());
    }

    #[test]
    fn repeated_analysis_with_same_input_is_deterministic() {
        let input = validated_input(
            &[0.0, 1.0, 2.0, 3.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp(), (-0.75_f64).exp()],
        );

        let first = KineticsAnalysisResult::analyze(&input).expect("first analysis should succeed");
        let second =
            KineticsAnalysisResult::analyze(&input).expect("second analysis should succeed");

        assert_eq!(first, second);
    }

    #[test]
    fn analysis_uses_in_memory_data_without_side_effects() {
        let input = validated_input(
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );

        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");

        assert_eq!(analysis.rejected_row_count(), 0);
        assert_eq!(analysis.review_status(), KineticsReviewStatus::Passed);
    }

    #[test]
    fn analysis_public_names_remain_cautious() {
        let names = [
            "KineticsAnalysisResult",
            "preferred_model",
            "comparison_basis",
            "review_status",
            "has_warnings",
        ];

        for name in names {
            assert!(!name.contains("True"));
            assert!(!name.contains("Best"));
            assert!(!name.contains("Final"));
            assert!(!name.contains("Prove"));
            assert!(!name.contains("Determine"));
        }
    }

    #[test]
    fn workflow_plan_has_chemistry_kinetics_csv_id() {
        let plan = kinetics_csv_workflow_plan().expect("workflow plan should construct");

        assert_eq!(plan.id().as_str(), CHEMISTRY_KINETICS_CSV_WORKFLOW_ID);
        assert_eq!(plan.name(), "Chemistry kinetics CSV");
    }

    #[test]
    fn workflow_plan_has_deterministic_ordered_steps() {
        let plan = kinetics_csv_workflow_plan().expect("workflow plan should construct");
        let keys: Vec<_> = plan.step_keys().map(|key| key.as_str()).collect();

        assert_eq!(
            keys,
            vec![
                "inspect_input",
                "validate_kinetics_input",
                "fit_first_order",
                "fit_second_order",
                "compare_models",
                "review_result",
                "produce_analysis_result",
                "complete",
            ]
        );
    }

    #[test]
    fn workflow_plan_uses_generic_step_kinds_without_chemistry_core_variants() {
        let plan = kinetics_csv_workflow_plan().expect("workflow plan should construct");
        let kinds: Vec<_> = plan.steps().iter().map(|step| step.kind()).collect();

        assert_eq!(
            kinds,
            vec![
                WorkflowStepKind::InspectInput,
                WorkflowStepKind::Custom,
                WorkflowStepKind::Custom,
                WorkflowStepKind::Custom,
                WorkflowStepKind::Review,
                WorkflowStepKind::Review,
                WorkflowStepKind::ProduceArtifact,
                WorkflowStepKind::Complete,
            ]
        );
    }

    #[test]
    fn repeated_workflow_plan_calls_are_equivalent() {
        let first = kinetics_csv_workflow_plan().expect("first plan should construct");
        let second = kinetics_csv_workflow_plan().expect("second plan should construct");

        assert_eq!(first, second);
    }

    #[test]
    fn workflow_plan_construction_does_not_execute_analysis() {
        let plan = kinetics_csv_workflow_plan().expect("workflow plan should construct");

        assert_eq!(plan.step_count(), 8);
        assert_eq!(plan.steps()[2].key().as_str(), "fit_first_order");
        assert_eq!(plan.steps()[3].key().as_str(), "fit_second_order");
    }

    #[test]
    fn workflow_plan_construction_does_not_require_data_table_or_csv_io() {
        let plan = kinetics_csv_workflow_plan().expect("workflow plan should construct");

        assert_eq!(plan.steps()[0].key().as_str(), "inspect_input");
        assert_eq!(plan.steps()[7].key().as_str(), "complete");
    }

    #[test]
    fn workflow_plan_public_names_remain_execution_free() {
        let names = [
            "kinetics_csv_workflow_plan",
            "inspect_input",
            "validate_kinetics_input",
            "produce_analysis_result",
        ];

        for name in names {
            assert!(!name.contains("execute"));
            assert!(!name.contains("read_file"));
            assert!(!name.contains("write_file"));
            assert!(!name.contains("call_model"));
            assert!(!name.contains("call_tool"));
        }
    }
}
