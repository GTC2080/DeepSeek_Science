//! Minimal in-memory CSV adapter for small numeric tables.
//!
//! This module intentionally supports only a tiny CSV subset for future
//! adapters: comma-separated UTF-8 text with one header row and numeric data
//! rows. It does not read or write files, detect dialects, support quoted
//! fields, evaluate formulas, or include domain-specific workflow behavior.

use crate::{CommonError, DataColumn, DataTable};

/// Parses a tiny in-memory numeric CSV subset into a [`DataTable`].
///
/// The supported subset is UTF-8 text already available as `&str`, comma
/// separators, one header row, numeric data rows, trimmed cells, and no quoted
/// fields. Row indices in CSV errors are zero-based data row indices and do not
/// include the header row.
pub fn parse_simple_numeric_csv(input: &str) -> Result<DataTable, CommonError> {
    let mut lines = input.lines();
    let header_line = lines.next().ok_or(CommonError::MissingCsvHeader)?;
    if header_line.trim().is_empty() {
        return Err(CommonError::MissingCsvHeader);
    }

    let header_names = parse_header(header_line)?;
    let mut column_values = vec![Vec::new(); header_names.len()];
    let mut data_row_count = 0;

    for (row_index, line) in lines.enumerate() {
        let values = parse_data_row(line, row_index, &header_names)?;
        for (column_index, value) in values.into_iter().enumerate() {
            column_values[column_index].push(value);
        }
        data_row_count += 1;
    }

    if data_row_count == 0 {
        return Err(CommonError::NoCsvDataRows);
    }

    let columns = header_names
        .into_iter()
        .zip(column_values)
        .map(|(name, values)| DataColumn::numeric(name, values))
        .collect::<Result<Vec<_>, _>>()?;

    DataTable::new(columns)
}

fn parse_header(line: &str) -> Result<Vec<String>, CommonError> {
    let fields = split_fields(line);
    reject_quoted_fields(&fields, None)?;

    let mut header_names = Vec::with_capacity(fields.len());
    for (column_index, field) in fields.iter().enumerate() {
        let name = field.trim();
        if name.is_empty() {
            return Err(CommonError::EmptyCsvHeaderName { column_index });
        }
        if header_names
            .iter()
            .any(|existing_name: &String| existing_name == name)
        {
            return Err(CommonError::DuplicateColumnName {
                name: name.to_string(),
            });
        }
        header_names.push(name.to_string());
    }

    Ok(header_names)
}

fn parse_data_row(
    line: &str,
    row_index: usize,
    header_names: &[String],
) -> Result<Vec<f64>, CommonError> {
    let fields = split_fields(line);
    reject_quoted_fields(&fields, Some(row_index))?;

    if fields.len() != header_names.len() {
        return Err(CommonError::InconsistentCsvFieldCount {
            row_index,
            expected: header_names.len(),
            actual: fields.len(),
        });
    }

    let mut values = Vec::with_capacity(header_names.len());
    for (column_index, field) in fields.iter().enumerate() {
        let value = parse_numeric_cell(field.trim(), row_index, column_index, header_names)?;
        values.push(value);
    }

    Ok(values)
}

fn split_fields(line: &str) -> Vec<&str> {
    line.split(',').collect()
}

fn reject_quoted_fields(fields: &[&str], row_index: Option<usize>) -> Result<(), CommonError> {
    for (column_index, field) in fields.iter().enumerate() {
        if field.contains('"') {
            return Err(CommonError::UnsupportedCsvQuotedField {
                row_index,
                column_index,
            });
        }
    }

    Ok(())
}

fn parse_numeric_cell(
    value: &str,
    row_index: usize,
    column_index: usize,
    header_names: &[String],
) -> Result<f64, CommonError> {
    let column_name = &header_names[column_index];
    if value.is_empty() {
        return Err(CommonError::EmptyCsvNumericCell {
            row_index,
            column_index,
            column_name: column_name.clone(),
        });
    }

    let parsed = value
        .parse::<f64>()
        .map_err(|_| CommonError::InvalidCsvFloat {
            row_index,
            column_index,
            column_name: column_name.clone(),
            value: value.to_string(),
        })?;

    if !parsed.is_finite() {
        return Err(CommonError::NonFiniteCsvFloat {
            row_index,
            column_index,
            column_name: column_name.clone(),
            value: value.to_string(),
        });
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use crate::{parse_simple_numeric_csv, CommonError};

    #[test]
    fn simple_valid_csv_parses_to_data_table() {
        let table = parse_simple_numeric_csv("time,concentration\n0,1\n1,0.5\n")
            .expect("inline CSV should parse");

        assert_eq!(table.row_count(), 2);
        assert_eq!(table.column_count(), 2);
        assert_eq!(
            table.numeric_column("time").expect("time column").values(),
            &[0.0, 1.0]
        );
        assert_eq!(
            table
                .numeric_column("concentration")
                .expect("concentration column")
                .values(),
            &[1.0, 0.5]
        );
    }

    #[test]
    fn column_order_is_preserved() {
        let table = parse_simple_numeric_csv("b,a\n2,1\n4,3\n").expect("CSV should parse");

        assert_eq!(table.column_names(), vec!["b", "a"]);
    }

    #[test]
    fn numeric_column_lookup_works_after_parsing() {
        let table =
            parse_simple_numeric_csv("time,value\n0,2.5\n1,3.5\n").expect("CSV should parse");

        let value = table.numeric_column("value").expect("value column");

        assert_eq!(value.values(), &[2.5, 3.5]);
    }

    #[test]
    fn duplicate_header_is_rejected() {
        let result = parse_simple_numeric_csv("time,time\n0,1\n");

        assert_eq!(
            result,
            Err(CommonError::DuplicateColumnName {
                name: "time".to_string()
            })
        );
    }

    #[test]
    fn empty_header_name_is_rejected() {
        let result = parse_simple_numeric_csv("time, \n0,1\n");

        assert_eq!(
            result,
            Err(CommonError::EmptyCsvHeaderName { column_index: 1 })
        );
    }

    #[test]
    fn missing_header_is_rejected() {
        let result = parse_simple_numeric_csv("");

        assert_eq!(result, Err(CommonError::MissingCsvHeader));
    }

    #[test]
    fn no_data_rows_is_rejected() {
        let result = parse_simple_numeric_csv("time,concentration\n");

        assert_eq!(result, Err(CommonError::NoCsvDataRows));
    }

    #[test]
    fn inconsistent_row_width_is_rejected() {
        let result = parse_simple_numeric_csv("time,concentration\n0,1\n1\n");

        assert_eq!(
            result,
            Err(CommonError::InconsistentCsvFieldCount {
                row_index: 1,
                expected: 2,
                actual: 1
            })
        );
    }

    #[test]
    fn empty_numeric_cell_is_rejected() {
        let result = parse_simple_numeric_csv("time,concentration\n0,\n");

        assert_eq!(
            result,
            Err(CommonError::EmptyCsvNumericCell {
                row_index: 0,
                column_index: 1,
                column_name: "concentration".to_string()
            })
        );
    }

    #[test]
    fn invalid_float_is_rejected() {
        let result = parse_simple_numeric_csv("time,concentration\n0,nope\n");

        assert_eq!(
            result,
            Err(CommonError::InvalidCsvFloat {
                row_index: 0,
                column_index: 1,
                column_name: "concentration".to_string(),
                value: "nope".to_string()
            })
        );
    }

    #[test]
    fn non_finite_float_is_rejected() {
        let result = parse_simple_numeric_csv("time,concentration\n0,NaN\n");

        assert_eq!(
            result,
            Err(CommonError::NonFiniteCsvFloat {
                row_index: 0,
                column_index: 1,
                column_name: "concentration".to_string(),
                value: "NaN".to_string()
            })
        );
    }

    #[test]
    fn quoted_field_is_rejected_as_unsupported() {
        let result = parse_simple_numeric_csv("time,concentration\n0,\"1\"\n");

        assert_eq!(
            result,
            Err(CommonError::UnsupportedCsvQuotedField {
                row_index: Some(0),
                column_index: 1
            })
        );
    }

    #[test]
    fn parser_uses_inline_strings_without_file_io() {
        let table = parse_simple_numeric_csv("x,y\n1,2\n").expect("inline CSV should parse");

        assert_eq!(table.column_names(), vec!["x", "y"]);
    }

    #[test]
    fn parser_remains_domain_neutral() {
        let table = parse_simple_numeric_csv("x,y\n1,2\n").expect("inline CSV should parse");

        assert_eq!(
            table.numeric_column("x").expect("x column").values(),
            &[1.0]
        );
        assert_eq!(
            table.numeric_column("y").expect("y column").values(),
            &[2.0]
        );
    }
}
