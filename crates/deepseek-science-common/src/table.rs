//! Small in-memory table contracts for future parsers and workflows.

use crate::CommonError;
use serde::{Deserialize, Serialize};

/// Validated table column name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnName(String);

impl ColumnName {
    /// Creates a non-empty column name.
    pub fn new(name: impl Into<String>) -> Result<Self, CommonError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(CommonError::EmptyColumnName);
        }

        Ok(Self(name))
    }

    /// Returns the original column name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Numeric column stored entirely in memory.
#[derive(Clone, Debug, PartialEq)]
pub struct DataColumn {
    name: ColumnName,
    values: Vec<f64>,
}

impl DataColumn {
    /// Creates a numeric column with finite values.
    pub fn numeric(name: impl Into<String>, values: Vec<f64>) -> Result<Self, CommonError> {
        let name = ColumnName::new(name)?;
        if values.iter().any(|value| !value.is_finite()) {
            return Err(CommonError::NonFiniteValue);
        }

        Ok(Self { name, values })
    }

    /// Returns the validated column name.
    pub fn name(&self) -> &ColumnName {
        &self.name
    }

    /// Returns the numeric values.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Returns the number of values in the column.
    pub fn len(&self) -> usize {
        self.values.len()
    }
}

/// Small deterministic numeric table.
#[derive(Clone, Debug, PartialEq)]
pub struct DataTable {
    columns: Vec<DataColumn>,
    rows: usize,
}

impl DataTable {
    /// Creates a table from validated numeric columns.
    pub fn new(columns: Vec<DataColumn>) -> Result<Self, CommonError> {
        let Some(first_column) = columns.first() else {
            return Err(CommonError::EmptyTable);
        };

        let rows = first_column.len();
        let mut names = Vec::with_capacity(columns.len());

        for column in &columns {
            let name = column.name().as_str();
            if names.iter().any(|existing_name| *existing_name == name) {
                return Err(CommonError::DuplicateColumnName {
                    name: name.to_string(),
                });
            }
            if column.len() != rows {
                return Err(CommonError::ColumnLengthMismatch {
                    name: name.to_string(),
                    expected: rows,
                    actual: column.len(),
                });
            }
            names.push(name);
        }

        Ok(Self { columns, rows })
    }

    /// Returns the number of rows.
    pub fn row_count(&self) -> usize {
        self.rows
    }

    /// Returns the number of columns.
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Returns shape metadata for the table.
    pub fn shape(&self) -> TableShape {
        TableShape::new(self.row_count(), self.column_count())
    }

    /// Returns column names in caller-provided order.
    pub fn column_names(&self) -> Vec<&str> {
        self.columns
            .iter()
            .map(|column| column.name().as_str())
            .collect()
    }

    /// Looks up a numeric column by exact name.
    pub fn numeric_column(&self, name: &str) -> Result<&DataColumn, CommonError> {
        self.columns
            .iter()
            .find(|column| column.name().as_str() == name)
            .ok_or_else(|| CommonError::MissingColumn {
                name: name.to_string(),
            })
    }
}

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

#[cfg(test)]
mod tests {
    use crate::CommonError;

    use super::{DataColumn, DataTable, TableShape};

    fn numeric_column(name: &str, values: &[f64]) -> DataColumn {
        DataColumn::numeric(name, values.to_vec()).expect("test data should be valid")
    }

    #[test]
    fn numeric_column_can_be_constructed() {
        let column = numeric_column("time_s", &[0.0, 1.0, 2.0]);

        assert_eq!(column.name().as_str(), "time_s");
        assert_eq!(column.values(), &[0.0, 1.0, 2.0]);
        assert_eq!(column.len(), 3);
    }

    #[test]
    fn numeric_column_rejects_empty_name() {
        let result = DataColumn::numeric("", vec![1.0]);

        assert_eq!(result, Err(CommonError::EmptyColumnName));
    }

    #[test]
    fn numeric_column_rejects_non_finite_values() {
        let nan_result = DataColumn::numeric("value", vec![f64::NAN]);
        let infinity_result = DataColumn::numeric("value", vec![f64::INFINITY]);

        assert_eq!(nan_result, Err(CommonError::NonFiniteValue));
        assert_eq!(infinity_result, Err(CommonError::NonFiniteValue));
    }

    #[test]
    fn data_table_can_be_constructed_from_equal_length_numeric_columns() {
        let time = numeric_column("time_s", &[0.0, 1.0]);
        let signal = numeric_column("signal", &[2.0, 3.0]);

        let result = DataTable::new(vec![time, signal]);

        assert!(result.is_ok());
    }

    #[test]
    fn data_table_reports_row_count_and_column_count() {
        let table = DataTable::new(vec![
            numeric_column("time_s", &[0.0, 1.0]),
            numeric_column("signal", &[2.0, 3.0]),
        ])
        .expect("table should be valid");

        assert_eq!(table.row_count(), 2);
        assert_eq!(table.column_count(), 2);
        assert_eq!(table.shape(), TableShape::new(2, 2));
    }

    #[test]
    fn data_table_rejects_zero_columns() {
        let result = DataTable::new(Vec::new());

        assert_eq!(result, Err(CommonError::EmptyTable));
    }

    #[test]
    fn data_table_rejects_duplicate_column_names() {
        let result = DataTable::new(vec![
            numeric_column("time_s", &[0.0, 1.0]),
            numeric_column("time_s", &[2.0, 3.0]),
        ]);

        assert_eq!(
            result,
            Err(CommonError::DuplicateColumnName {
                name: "time_s".to_string()
            })
        );
    }

    #[test]
    fn data_table_rejects_mismatched_column_lengths() {
        let result = DataTable::new(vec![
            numeric_column("time_s", &[0.0, 1.0]),
            numeric_column("signal", &[2.0]),
        ]);

        assert_eq!(
            result,
            Err(CommonError::ColumnLengthMismatch {
                name: "signal".to_string(),
                expected: 2,
                actual: 1
            })
        );
    }

    #[test]
    fn data_table_lookup_returns_requested_numeric_column() {
        let table = DataTable::new(vec![
            numeric_column("time_s", &[0.0, 1.0]),
            numeric_column("signal", &[2.0, 3.0]),
        ])
        .expect("table should be valid");

        let column = table
            .numeric_column("signal")
            .expect("signal column should exist");

        assert_eq!(column.name().as_str(), "signal");
        assert_eq!(column.values(), &[2.0, 3.0]);
    }

    #[test]
    fn data_table_lookup_returns_structured_error_for_missing_column() {
        let table = DataTable::new(vec![numeric_column("time_s", &[0.0, 1.0])])
            .expect("table should be valid");

        let result = table.numeric_column("signal");

        assert_eq!(
            result,
            Err(CommonError::MissingColumn {
                name: "signal".to_string()
            })
        );
    }

    #[test]
    fn data_table_column_names_keep_caller_order() {
        let table = DataTable::new(vec![
            numeric_column("time_s", &[0.0, 1.0]),
            numeric_column("signal", &[2.0, 3.0]),
            numeric_column("baseline", &[4.0, 5.0]),
        ])
        .expect("table should be valid");

        assert_eq!(table.column_names(), vec!["time_s", "signal", "baseline"]);
    }
}
