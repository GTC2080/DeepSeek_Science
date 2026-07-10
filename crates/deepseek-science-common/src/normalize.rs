//! Pure normalization of eligible inspected text into the simple CSV boundary.
//!
//! This module trusts only existing encoding and delimited-table findings. It
//! preserves safe cell text exactly, performs no file IO or scientific
//! interpretation, and emits one bounded UTF-8 comma/LF representation.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::{
    ByteOrderMark, DelimitedTextInspection, DelimiterFinding, EncodingInspection,
    GenericTableShape, TableRegionInspection, TextEncoding,
};

/// Fixed maximum byte length of v0.4 normalized output.
pub const MAX_NORMALIZED_OUTPUT_BYTES: usize = 24 * 1024 * 1024;

/// Safe structured reason that exact cell text cannot enter simple CSV output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsafeCellReason {
    /// The cell has leading or trailing Unicode whitespace.
    SurroundingWhitespace,
    /// A tab-delimited cell contains a comma and would require quoting.
    CommaRequiresQuoting,
    /// The cell contains a double quote.
    DoubleQuote,
    /// The cell contains NUL.
    Nul,
    /// The cell contains an ASCII control character.
    ControlCharacter,
    /// The cell contains DEL.
    DeleteCharacter,
    /// A CR was not part of a CRLF record terminator.
    LoneCarriageReturn,
    /// A header cell is empty.
    EmptyHeader,
    /// A header label is duplicated.
    DuplicateHeader,
    /// A numeric body cell is empty.
    EmptyNumericCell,
    /// A body cell is not valid numeric text.
    InvalidNumericCell,
    /// A body cell parses to a non-finite number.
    NonFiniteNumericCell,
}

impl fmt::Display for UnsafeCellReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::SurroundingWhitespace => "leading or trailing whitespace is not supported",
            Self::CommaRequiresQuoting => "a comma would require CSV quoting",
            Self::DoubleQuote => "double quotes are not supported",
            Self::Nul => "NUL is not supported",
            Self::ControlCharacter => "ASCII control characters are not supported",
            Self::DeleteCharacter => "DEL is not supported",
            Self::LoneCarriageReturn => "lone carriage returns are not supported",
            Self::EmptyHeader => "header labels must not be empty",
            Self::DuplicateHeader => "header labels must be unique",
            Self::EmptyNumericCell => "numeric cells must not be empty",
            Self::InvalidNumericCell => "body cells must be numeric",
            Self::NonFiniteNumericCell => "numeric cells must be finite",
        };
        formatter.write_str(message)
    }
}

/// Errors raised by deterministic simple-CSV normalization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizationError {
    /// The source already matches the current simple numeric CSV boundary.
    AlreadyCompatible,
    /// Existing inspection findings do not establish one eligible narrow table.
    StructuralConversionIneligible,
    /// Exact cell text cannot be emitted without an unsupported transformation.
    UnsafeCellContent {
        /// One-based physical line number.
        line: usize,
        /// One-based column number.
        column: usize,
        /// Safe reason that does not retain raw cell contents.
        reason: UnsafeCellReason,
    },
    /// Normalized output exceeded the fixed byte limit.
    OutputLimitExceeded {
        /// First projected byte length beyond the accepted boundary.
        actual: usize,
        /// Maximum accepted normalized byte length.
        maximum: usize,
    },
    /// Projected output length could not be represented by `usize`.
    ArithmeticOverflow,
    /// Existing inspection evidence contradicted the exact decoded rows.
    InspectionInvariant,
}

impl fmt::Display for NormalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyCompatible => formatter.write_str(
                "input already matches the current simple numeric CSV format and can be used directly",
            ),
            Self::StructuralConversionIneligible => formatter.write_str(
                "input structure is not eligible for explicit conversion",
            ),
            Self::UnsafeCellContent {
                line,
                column,
                reason,
            } => write!(
                formatter,
                "unsafe cell content at line {line}, column {column}: {reason}"
            ),
            Self::OutputLimitExceeded { actual, maximum } => write!(
                formatter,
                "normalized output has at least {actual} bytes, exceeding limit of {maximum} bytes"
            ),
            Self::ArithmeticOverflow => {
                formatter.write_str("normalized output length overflowed")
            }
            Self::InspectionInvariant => formatter.write_str(
                "decoded rows contradict the completed table inspection",
            ),
        }
    }
}

impl Error for NormalizationError {}

/// Normalizes one eligible inspected text input into deterministic simple CSV.
///
/// The output is UTF-8 without a BOM, comma-delimited, LF-terminated, and has
/// exactly one trailing LF. Existing-compatible input and any structure or cell
/// requiring repair, trimming, quoting, or interpretation are rejected.
pub fn normalize_delimited_text(
    encoding: &EncodingInspection,
    inspection: &DelimitedTextInspection,
) -> Result<String, NormalizationError> {
    normalize_delimited_text_with_limit(encoding, inspection, MAX_NORMALIZED_OUTPUT_BYTES)
}

fn normalize_delimited_text_with_limit(
    encoding: &EncodingInspection,
    inspection: &DelimitedTextInspection,
    maximum: usize,
) -> Result<String, NormalizationError> {
    let (delimiter, region) = validate_eligibility(inspection)?;
    let normalization_required = normalization_required(encoding, inspection.delimiter)?;
    let projected_length = if normalization_required {
        project_output_length(&encoding.text, maximum)?
    } else {
        0
    };
    let mut output = String::with_capacity(projected_length);
    let mut scanner = RowScanner::new(&encoding.text, delimiter);
    let mut header_names = HashSet::with_capacity(region.stable_field_count);
    let expected_rows = region
        .fully_numeric_row_count
        .checked_add(1)
        .ok_or(NormalizationError::ArithmeticOverflow)?;
    let mut row_count = 0;

    while let Some((line_number, row)) = scanner.next_row()? {
        row_count += 1;
        let mut field_count = 0;

        for cell in row.split(delimiter) {
            field_count += 1;
            validate_cell(cell, line_number, field_count, delimiter)?;

            if line_number == 1 {
                if cell.is_empty() {
                    return unsafe_cell(line_number, field_count, UnsafeCellReason::EmptyHeader);
                }
                if !header_names.insert(cell) {
                    return unsafe_cell(
                        line_number,
                        field_count,
                        UnsafeCellReason::DuplicateHeader,
                    );
                }
            } else {
                validate_numeric_cell(cell, line_number, field_count)?;
            }

            if normalization_required {
                if field_count > 1 {
                    push_checked(&mut output, ",", maximum)?;
                }
                push_checked(&mut output, cell, maximum)?;
            }
        }

        if field_count != region.stable_field_count {
            return Err(NormalizationError::InspectionInvariant);
        }
        if normalization_required {
            push_checked(&mut output, "\n", maximum)?;
        }
    }

    if row_count != expected_rows || row_count != inspection.physical_line_count {
        return Err(NormalizationError::InspectionInvariant);
    }
    if !normalization_required {
        return Err(NormalizationError::AlreadyCompatible);
    }
    if output.len() != projected_length || output.len() > maximum {
        return Err(NormalizationError::InspectionInvariant);
    }

    Ok(output)
}

fn project_output_length(text: &str, maximum: usize) -> Result<usize, NormalizationError> {
    let bytes = text.as_bytes();
    let mut row_start = 0;
    let mut projected = 0;

    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        let row_end = if index > row_start && bytes[index - 1] == b'\r' {
            index - 1
        } else {
            index
        };
        projected = checked_projected_add(projected, row_end - row_start + 1, maximum)?;
        row_start = index + 1;
    }

    if row_start < bytes.len() {
        projected = checked_projected_add(projected, bytes.len() - row_start + 1, maximum)?;
    }

    Ok(projected)
}

fn checked_projected_add(
    current: usize,
    additional: usize,
    maximum: usize,
) -> Result<usize, NormalizationError> {
    let next = current
        .checked_add(additional)
        .ok_or(NormalizationError::ArithmeticOverflow)?;
    if next > maximum {
        return Err(NormalizationError::OutputLimitExceeded {
            actual: next,
            maximum,
        });
    }
    Ok(next)
}

fn validate_eligibility(
    inspection: &DelimitedTextInspection,
) -> Result<(char, &TableRegionInspection), NormalizationError> {
    let delimiter = match inspection.delimiter {
        DelimiterFinding::Comma => ',',
        DelimiterFinding::Tab => '\t',
        DelimiterFinding::Ambiguous | DelimiterFinding::Unsupported => {
            return Err(NormalizationError::StructuralConversionIneligible);
        }
    };
    let Some(region) = inspection.region.as_ref() else {
        return Err(NormalizationError::StructuralConversionIneligible);
    };

    let mut unique_headers = HashSet::with_capacity(region.header_labels.len());
    let headers_are_unique = region
        .header_labels
        .iter()
        .all(|header| !header.is_empty() && unique_headers.insert(header.as_str()));

    if !inspection.complete
        || inspection.shape != GenericTableShape::NumericNarrowTable
        || inspection.blank_line_count != 0
        || inspection.quoted_lines.total_count != 0
        || region.first_line != 1
        || region.last_line != inspection.physical_line_count
        || region.stable_field_count < 2
        || region.header_candidate_lines.total_count != 1
        || region.header_candidate_lines.example_lines.first() != Some(&1)
        || region.header_labels.len() != region.stable_field_count
        || !headers_are_unique
        || region.fully_numeric_row_count == 0
        || region.nonnumeric_row_count != 1
        || region.nonnumeric_lines.total_count != 1
        || region.metadata_lines.total_count != 0
        || region.inconsistent_width_lines.total_count != 0
        || region.additional_content_lines.total_count != 0
        || region.empty_numeric_cell_lines.total_count != 0
        || region.non_finite_numeric_lines.total_count != 0
    {
        return Err(NormalizationError::StructuralConversionIneligible);
    }

    Ok((delimiter, region))
}

fn normalization_required(
    encoding: &EncodingInspection,
    delimiter: DelimiterFinding,
) -> Result<bool, NormalizationError> {
    let supported_pair = matches!(
        (encoding.encoding, encoding.bom),
        (
            TextEncoding::Utf8,
            ByteOrderMark::None | ByteOrderMark::Utf8
        ) | (TextEncoding::Utf16Le, ByteOrderMark::Utf16Le)
            | (TextEncoding::Utf16Be, ByteOrderMark::Utf16Be)
    );
    if !supported_pair {
        return Err(NormalizationError::InspectionInvariant);
    }

    Ok(encoding.encoding != TextEncoding::Utf8
        || encoding.bom != ByteOrderMark::None
        || delimiter == DelimiterFinding::Tab)
}

fn validate_cell(
    cell: &str,
    line: usize,
    column: usize,
    delimiter: char,
) -> Result<(), NormalizationError> {
    if cell != cell.trim() {
        return unsafe_cell(line, column, UnsafeCellReason::SurroundingWhitespace);
    }

    for character in cell.chars() {
        let reason = match character {
            ',' if delimiter == '\t' => Some(UnsafeCellReason::CommaRequiresQuoting),
            '"' => Some(UnsafeCellReason::DoubleQuote),
            '\0' => Some(UnsafeCellReason::Nul),
            '\r' => Some(UnsafeCellReason::LoneCarriageReturn),
            '\n' => Some(UnsafeCellReason::ControlCharacter),
            '\u{0001}'..='\u{001f}' => Some(UnsafeCellReason::ControlCharacter),
            '\u{007f}' => Some(UnsafeCellReason::DeleteCharacter),
            _ => None,
        };
        if let Some(reason) = reason {
            return unsafe_cell(line, column, reason);
        }
    }

    Ok(())
}

fn validate_numeric_cell(cell: &str, line: usize, column: usize) -> Result<(), NormalizationError> {
    if cell.is_empty() {
        return unsafe_cell(line, column, UnsafeCellReason::EmptyNumericCell);
    }
    let value = cell
        .parse::<f64>()
        .map_err(|_| NormalizationError::UnsafeCellContent {
            line,
            column,
            reason: UnsafeCellReason::InvalidNumericCell,
        })?;
    if !value.is_finite() {
        return unsafe_cell(line, column, UnsafeCellReason::NonFiniteNumericCell);
    }
    Ok(())
}

fn unsafe_cell<T>(
    line: usize,
    column: usize,
    reason: UnsafeCellReason,
) -> Result<T, NormalizationError> {
    Err(NormalizationError::UnsafeCellContent {
        line,
        column,
        reason,
    })
}

fn push_checked(
    output: &mut String,
    value: &str,
    maximum: usize,
) -> Result<(), NormalizationError> {
    let next_length = output
        .len()
        .checked_add(value.len())
        .ok_or(NormalizationError::ArithmeticOverflow)?;
    if next_length > maximum {
        return Err(NormalizationError::OutputLimitExceeded {
            actual: next_length,
            maximum,
        });
    }
    output.push_str(value);
    Ok(())
}

struct RowScanner<'a> {
    text: &'a str,
    position: usize,
    line_number: usize,
    delimiter: char,
}

impl<'a> RowScanner<'a> {
    fn new(text: &'a str, delimiter: char) -> Self {
        Self {
            text,
            position: 0,
            line_number: 1,
            delimiter,
        }
    }

    fn next_row(&mut self) -> Result<Option<(usize, &'a str)>, NormalizationError> {
        if self.position == self.text.len() {
            return Ok(None);
        }

        let remainder = &self.text[self.position..];
        let (row_end, next_position) =
            match remainder.as_bytes().iter().position(|byte| *byte == b'\n') {
                Some(relative_newline) => {
                    let newline = self.position + relative_newline;
                    let row_end =
                        if newline > self.position && self.text.as_bytes()[newline - 1] == b'\r' {
                            newline - 1
                        } else {
                            newline
                        };
                    (row_end, newline + 1)
                }
                None => (self.text.len(), self.text.len()),
            };
        let row = &self.text[self.position..row_end];
        if let Some(relative_cr) = row.as_bytes().iter().position(|byte| *byte == b'\r') {
            let column = row.as_bytes()[..relative_cr]
                .iter()
                .filter(|byte| **byte == self.delimiter as u8)
                .count()
                + 1;
            return unsafe_cell(
                self.line_number,
                column,
                UnsafeCellReason::LoneCarriageReturn,
            );
        }

        let line_number = self.line_number;
        self.position = next_position;
        self.line_number += 1;
        Ok(Some((line_number, row)))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        inspect_delimited_text, inspect_text_encoding, ByteOrderMark, EncodingInspectionError,
    };

    use super::{
        normalize_delimited_text, normalize_delimited_text_with_limit, NormalizationError,
        UnsafeCellReason,
    };

    fn utf16_bytes(text: &str, little_endian: bool) -> Vec<u8> {
        let mut bytes = if little_endian {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for unit in text.encode_utf16() {
            let encoded = if little_endian {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            };
            bytes.extend_from_slice(&encoded);
        }
        bytes
    }

    fn normalize(bytes: &[u8]) -> Result<String, NormalizationError> {
        let encoding = inspect_text_encoding(bytes).expect("test bytes should decode");
        let inspection =
            inspect_delimited_text(&encoding.text).expect("decoded test text should inspect");
        normalize_delimited_text(&encoding, &inspection)
    }

    fn structural_error(bytes: &[u8]) {
        assert_eq!(
            normalize(bytes),
            Err(NormalizationError::StructuralConversionIneligible)
        );
    }

    #[test]
    fn utf8_bom_comma_table_normalizes() {
        assert_eq!(
            normalize(b"\xef\xbb\xbfA,B\n1,2\n").expect("table should normalize"),
            "A,B\n1,2\n"
        );
    }

    #[test]
    fn utf16le_bom_comma_table_normalizes() {
        assert_eq!(
            normalize(&utf16_bytes("A,B\n1,2\n", true)).expect("table should normalize"),
            "A,B\n1,2\n"
        );
    }

    #[test]
    fn utf16be_bom_comma_table_normalizes() {
        assert_eq!(
            normalize(&utf16_bytes("A,B\n1,2\n", false)).expect("table should normalize"),
            "A,B\n1,2\n"
        );
    }

    #[test]
    fn utf8_no_bom_tab_table_normalizes() {
        assert_eq!(
            normalize(b"A\tB\n1\t2\n").expect("table should normalize"),
            "A,B\n1,2\n"
        );
    }

    #[test]
    fn utf8_bom_tab_table_normalizes() {
        assert_eq!(
            normalize(b"\xef\xbb\xbfA\tB\n1\t2\n").expect("table should normalize"),
            "A,B\n1,2\n"
        );
    }

    #[test]
    fn utf16le_tab_table_normalizes() {
        assert_eq!(
            normalize(&utf16_bytes("A\tB\n1\t2\n", true)).expect("table should normalize"),
            "A,B\n1,2\n"
        );
    }

    #[test]
    fn utf16be_tab_table_normalizes() {
        assert_eq!(
            normalize(&utf16_bytes("A\tB\n1\t2\n", false)).expect("table should normalize"),
            "A,B\n1,2\n"
        );
    }

    #[test]
    fn lf_and_crlf_sources_normalize_to_lf() {
        assert_eq!(
            normalize(b"\xef\xbb\xbfA,B\r\n1,2\r\n").expect("CRLF should normalize"),
            "A,B\n1,2\n"
        );
        assert_eq!(
            normalize(b"\xef\xbb\xbfA,B\n1,2\n").expect("LF should normalize"),
            "A,B\n1,2\n"
        );
    }

    #[test]
    fn missing_or_existing_final_terminator_becomes_one_lf() {
        let missing = normalize(b"\xef\xbb\xbfA,B\n1,2").expect("table should normalize");
        let existing = normalize(b"\xef\xbb\xbfA,B\n1,2\n").expect("table should normalize");

        assert_eq!(missing, "A,B\n1,2\n");
        assert_eq!(existing, missing);
        assert!(!existing.ends_with("\n\n"));
    }

    #[test]
    fn numeric_lexical_text_and_column_order_are_preserved() {
        let output =
            normalize(b"left\tright\n+01.00e-03\t-0\n1E+2\t3.0\n").expect("table should normalize");

        assert_eq!(output, "left,right\n+01.00e-03,-0\n1E+2,3.0\n");
    }

    #[test]
    fn printable_unicode_headers_are_preserved() {
        assert_eq!(
            normalize("波长\tΔ值\n1\t2\n".as_bytes()).expect("table should normalize"),
            "波长,Δ值\n1,2\n"
        );
    }

    #[test]
    fn repeated_normalization_is_identical() {
        let bytes = b"A\tB\n1\t2\n";

        assert_eq!(normalize(bytes), normalize(bytes));
    }

    #[test]
    fn already_compatible_input_is_rejected() {
        assert_eq!(
            normalize(b"A,B\n1,2\n"),
            Err(NormalizationError::AlreadyCompatible)
        );
    }

    #[test]
    fn matrix_empty_metadata_unit_and_headerless_inputs_are_rejected() {
        structural_error(b"\xef\xbb\xbfaxis,series 1,series 2\n1,2,3\n2,3,4\n");
        structural_error(b"");
        structural_error(b"\xef\xbb\xbfmetadata\nA,B\n1,2\n");
        structural_error(b"\xef\xbb\xbfA,B\nunit,unit\n1,2\n");
        structural_error(b"\xef\xbb\xbf1,2\n3,4\n");
    }

    #[test]
    fn ambiguous_headers_delimiters_and_regions_are_rejected() {
        structural_error(b"\xef\xbb\xbfA,B\nunit,unit\n1,2\n");
        structural_error(b"\xef\xbb\xbfA,B\n1,2\n\nC\tD\n3\t4\n");
        structural_error(b"\xef\xbb\xbfA,B\n1,2\n3,4\n\nC,D\n5,6\n7,8\n");
    }

    #[test]
    fn blank_rows_and_inconsistent_width_are_rejected() {
        structural_error(b"\xef\xbb\xbfA,B\n1,2\n\n3,4\n");
        structural_error(b"\xef\xbb\xbfA,B\n1,2\n3\n");
    }

    #[test]
    fn empty_and_non_finite_body_cells_are_rejected() {
        structural_error(b"\xef\xbb\xbfA,B\n1,\n2,3\n");
        structural_error(b"\xef\xbb\xbfA,B\n1,NaN\n2,3\n");
    }

    #[test]
    fn quoted_and_multiline_input_is_rejected() {
        structural_error(b"\xef\xbb\xbfA,B\n1,\"2\"\n");
        structural_error(b"\xef\xbb\xbfA,B\n1,\"two\nlines\"\n2,3\n");
    }

    #[test]
    fn comma_inside_tsv_cell_is_rejected_with_location() {
        assert_eq!(
            normalize(b"A,unit\tB\n1\t2\n"),
            Err(NormalizationError::UnsafeCellContent {
                line: 1,
                column: 1,
                reason: UnsafeCellReason::CommaRequiresQuoting,
            })
        );
    }

    #[test]
    fn surrounding_ascii_and_unicode_whitespace_is_rejected() {
        for bytes in [
            b" A\tB\n1\t2\n".as_slice(),
            b"A \tB\n1\t2\n".as_slice(),
            "\u{2003}A\tB\n1\t2\n".as_bytes(),
            "A\tB\n1\t2\u{2003}\n".as_bytes(),
        ] {
            assert!(matches!(
                normalize(bytes),
                Err(NormalizationError::UnsafeCellContent {
                    reason: UnsafeCellReason::SurroundingWhitespace,
                    ..
                })
            ));
        }
    }

    #[test]
    fn double_quote_control_and_del_are_rejected() {
        structural_error(b"A\t\"B\"\n1\t2\n");
        assert!(matches!(
            normalize(b"A\x01\tB\n1\t2\n"),
            Err(NormalizationError::UnsafeCellContent {
                reason: UnsafeCellReason::ControlCharacter,
                ..
            })
        ));
        assert!(matches!(
            normalize(b"A\x7f\tB\n1\t2\n"),
            Err(NormalizationError::UnsafeCellContent {
                reason: UnsafeCellReason::DeleteCharacter,
                ..
            })
        ));
    }

    #[test]
    fn nul_is_rejected_by_the_real_encoding_boundary() {
        assert_eq!(
            inspect_text_encoding(b"A\tB\n1\t2\0\n"),
            Err(EncodingInspectionError::UnsupportedBinaryInput { byte_offset: 7 })
        );
    }

    #[test]
    fn lone_cr_is_rejected_with_deterministic_location() {
        assert_eq!(
            normalize(b"\xef\xbb\xbfA,B\r\n1,2\r"),
            Err(NormalizationError::UnsafeCellContent {
                line: 2,
                column: 2,
                reason: UnsafeCellReason::LoneCarriageReturn,
            })
        );
    }

    #[test]
    fn duplicate_and_empty_headers_are_rejected() {
        structural_error(b"\xef\xbb\xbfA,A\n1,2\n");
        structural_error(b"\xef\xbb\xbfA,\n1,2\n");
    }

    #[test]
    fn output_limit_is_tested_without_a_large_allocation() {
        let encoding =
            inspect_text_encoding(b"\xef\xbb\xbfA,B\n1,2\n").expect("test bytes should decode");
        assert_eq!(encoding.bom, ByteOrderMark::Utf8);
        let inspection = inspect_delimited_text(&encoding.text).expect("text should inspect");

        assert_eq!(
            normalize_delimited_text_with_limit(&encoding, &inspection, 7),
            Err(NormalizationError::OutputLimitExceeded {
                actual: 8,
                maximum: 7,
            })
        );
    }

    #[test]
    fn unsafe_error_location_is_one_based_and_deterministic() {
        let expected = Err(NormalizationError::UnsafeCellContent {
            line: 2,
            column: 2,
            reason: UnsafeCellReason::SurroundingWhitespace,
        });

        assert_eq!(normalize(b"A\tB\n1\t 2\n"), expected);
        assert_eq!(normalize(b"A\tB\n1\t 2\n"), expected);
    }
}
