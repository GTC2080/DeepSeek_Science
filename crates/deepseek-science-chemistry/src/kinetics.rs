//! In-memory input validation for chemistry kinetics workflows.

use deepseek_science_common::{ColumnName, CommonError, DataColumn, DataTable};

use crate::KineticsError;

/// Workflow identifier reserved for the future chemistry kinetics CSV workflow.
///
/// The first implementation accepts an in-memory [`DataTable`]. The `_csv`
/// suffix names the expected future user-facing adapter, not file IO here.
pub const CHEMISTRY_KINETICS_CSV_WORKFLOW_ID: &str = "chemistry.kinetics_csv";

const MINIMUM_VALID_POINTS: usize = 2;

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

#[cfg(test)]
mod tests {
    use deepseek_science_common::{DataColumn, DataTable};

    use crate::{
        KineticsColumns, KineticsError, RejectedKineticsRowReason, ValidatedKineticsInput,
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
}
