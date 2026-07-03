//! In-memory input validation for chemistry kinetics workflows.

use deepseek_science_common::{
    simple_linear_regression, ColumnName, CommonError, DataColumn, DataTable,
};

use crate::error::KineticsError;

/// Workflow identifier reserved for the future chemistry kinetics CSV workflow.
///
/// The first implementation accepts an in-memory [`DataTable`]. The `_csv`
/// suffix names the expected future user-facing adapter, not file IO here.
pub const CHEMISTRY_KINETICS_CSV_WORKFLOW_ID: &str = "chemistry.kinetics_csv";

const MINIMUM_VALID_POINTS: usize = 2;

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

#[cfg(test)]
mod tests {
    use deepseek_science_common::{DataColumn, DataTable};

    use crate::{
        KineticsColumns, KineticsError, KineticsFitResult, KineticsModelKind,
        RejectedKineticsRowReason, ValidatedKineticsInput, CHEMISTRY_KINETICS_CSV_WORKFLOW_ID,
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
}
