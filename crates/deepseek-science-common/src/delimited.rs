//! Pure bounded inspection for simple comma- or tab-delimited decoded text.
//!
//! This module reports structural evidence without reading files, retaining a
//! full parsed table, interpreting scientific labels, or implementing quoted
//! CSV behavior.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::{ByteOrderMark, EncodingInspection, TextEncoding, MAX_INSPECTION_BYTES};

const DIAGNOSTIC_EXAMPLE_LIMIT: usize = 8;

/// Supported delimiter finding for decoded text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelimiterFinding {
    /// A comma-delimited region was selected uniquely.
    Comma,
    /// A tab-delimited region was selected uniquely.
    Tab,
    /// More than one delimiter or table region remained equally plausible.
    Ambiguous,
    /// Neither comma nor tab produced a plausible table region.
    Unsupported,
}

/// Generic table-shape classification without domain semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenericTableShape {
    /// Named finite numeric columns without conservative matrix evidence.
    NumericNarrowTable,
    /// Named finite numeric rectangle with monotonic and sibling-header evidence.
    NumericMatrix,
    /// Mixed, ambiguous, quoted, or otherwise unsupported structure.
    MixedOrUnsupported,
    /// Empty or whitespace-only decoded text.
    Empty,
}

/// Compatibility with the existing simple numeric CSV input boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimpleCsvCompatibility {
    /// Input already matches UTF-8, no-BOM, comma, named numeric-table rules.
    CompatibleAsIs,
    /// Structure is narrow and deterministic but needs an explicit future normalization.
    RequiresExplicitNormalization,
    /// Structure cannot safely enter the existing simple numeric CSV parser.
    Incompatible,
}

/// Structured reasons supporting a generic shape classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableShapeReason {
    /// No usable nonblank input was present.
    NoUsableTable,
    /// At least one nonblank line contained a quote.
    QuotedInput,
    /// Neither supported delimiter produced a plausible table region.
    UnsupportedDelimiter,
    /// Comma and tab evidence was equally strong.
    AmbiguousDelimiter,
    /// Multiple equally strong regions existed for one delimiter.
    AmbiguousTableRegion,
    /// The selected numeric region had no named header row.
    MissingNamedHeader,
    /// More than one leading nonnumeric row could be a header or unit row.
    MultipleHeaderRows,
    /// At least one selected header label was empty after trimming.
    EmptyHeaderLabel,
    /// A selected header label occurred more than once.
    DuplicateHeaderLabel,
    /// A nonnumeric row appeared after the numeric body began.
    NonnumericBody,
    /// A trailing row disagreed with the selected stable field width.
    InconsistentFieldCount,
    /// Additional nonblank content followed the selected region.
    AdditionalTableContent,
    /// A numeric-body row contained an empty cell.
    EmptyNumericCell,
    /// A numeric-body row contained a non-finite numeric value.
    NonFiniteNumericCell,
    /// Nonblank metadata appeared before the selected region.
    MetadataBeforeTable,
    /// One named finite rectangular numeric body was established.
    NamedFiniteRectangle,
    /// Monotonic first-column and sibling-header syntax established a matrix.
    MonotonicSiblingMatrix,
}

/// Bounded one-based line examples for one diagnostic kind.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BoundedLineEvidence {
    /// Total number of matching physical lines.
    pub total_count: usize,
    /// First bounded set of one-based physical line numbers.
    pub example_lines: Vec<usize>,
    /// Whether matching lines were omitted from `example_lines`.
    pub additional_examples_omitted: bool,
}

impl BoundedLineEvidence {
    fn record(&mut self, line_number: usize) {
        self.total_count += 1;
        if self.example_lines.len() < DIAGNOSTIC_EXAMPLE_LIMIT {
            self.example_lines.push(line_number);
        } else {
            self.additional_examples_omitted = true;
        }
    }
}

/// Evidence for one uniquely selected stable table region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableRegionInspection {
    /// First one-based physical line in the selected region.
    pub first_line: usize,
    /// Last one-based physical line in the selected region.
    pub last_line: usize,
    /// Stable field count shared inside the selected region.
    pub stable_field_count: usize,
    /// Leading nonnumeric rows that may be headers or unit rows.
    pub header_candidate_lines: BoundedLineEvidence,
    /// Trimmed labels from the unique selected header, or empty when ambiguous.
    pub header_labels: Vec<String>,
    /// Fully finite numeric rows inside the selected region.
    pub fully_numeric_row_count: usize,
    /// Nonnumeric rows inside the selected region, including header candidates.
    pub nonnumeric_row_count: usize,
    /// Bounded one-based evidence for nonnumeric rows inside the region.
    pub nonnumeric_lines: BoundedLineEvidence,
    /// Bounded one-based nonblank metadata lines before the region.
    pub metadata_lines: BoundedLineEvidence,
    /// Bounded one-based trailing lines with a different field count.
    pub inconsistent_width_lines: BoundedLineEvidence,
    /// Bounded one-based nonblank lines following the selected region.
    pub additional_content_lines: BoundedLineEvidence,
    /// Bounded one-based selected-region rows containing an empty cell.
    pub empty_numeric_cell_lines: BoundedLineEvidence,
    /// Bounded one-based selected-region rows containing non-finite numeric text.
    pub non_finite_numeric_lines: BoundedLineEvidence,
}

/// Completed pure inspection of one bounded decoded text input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelimitedTextInspection {
    /// Number of decoded physical lines.
    pub physical_line_count: usize,
    /// Number of blank or whitespace-only physical lines.
    pub blank_line_count: usize,
    /// Number of nonblank physical lines.
    pub nonblank_line_count: usize,
    /// Whether the bounded input was inspected completely.
    pub complete: bool,
    /// Deterministic comma-versus-tab finding.
    pub delimiter: DelimiterFinding,
    /// Selected table-region evidence, when unique.
    pub region: Option<TableRegionInspection>,
    /// Generic table shape.
    pub shape: GenericTableShape,
    /// Bounded structured reasons supporting `shape`.
    pub reasons: Vec<TableShapeReason>,
    /// Bounded one-based evidence for lines containing quotes.
    pub quoted_lines: BoundedLineEvidence,
}

/// Contract-level errors for delimited-text inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DelimitedInspectionError {
    /// Decoded text exceeded the shared inspection byte limit.
    InspectionLimitExceeded {
        /// Decoded UTF-8 byte length.
        actual: usize,
        /// Maximum accepted byte length.
        maximum: usize,
    },
}

impl fmt::Display for DelimitedInspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InspectionLimitExceeded { actual, maximum } => write!(
                formatter,
                "decoded text has {actual} bytes, exceeding inspection limit of {maximum} bytes"
            ),
        }
    }
}

impl Error for DelimitedInspectionError {}

/// Inspects bounded decoded text for deterministic comma/tab table structure.
pub fn inspect_delimited_text(
    text: &str,
) -> Result<DelimitedTextInspection, DelimitedInspectionError> {
    inspect_delimited_text_with_limit(text, MAX_INSPECTION_BYTES)
}

/// Assesses generic compatibility with the current simple numeric CSV parser.
///
/// `RequiresExplicitNormalization` is a format finding only. This function
/// does not perform conversion or claim that a conversion command exists.
pub fn assess_simple_csv_compatibility(
    encoding: &EncodingInspection,
    table: &DelimitedTextInspection,
) -> SimpleCsvCompatibility {
    if table.shape != GenericTableShape::NumericNarrowTable {
        return SimpleCsvCompatibility::Incompatible;
    }

    let Some(region) = table.region.as_ref() else {
        return SimpleCsvCompatibility::Incompatible;
    };

    if encoding.encoding == TextEncoding::Utf8
        && encoding.bom == ByteOrderMark::None
        && table.delimiter == DelimiterFinding::Comma
        && region.metadata_lines.total_count == 0
    {
        SimpleCsvCompatibility::CompatibleAsIs
    } else {
        SimpleCsvCompatibility::RequiresExplicitNormalization
    }
}

fn inspect_delimited_text_with_limit(
    text: &str,
    maximum: usize,
) -> Result<DelimitedTextInspection, DelimitedInspectionError> {
    if text.len() > maximum {
        return Err(DelimitedInspectionError::InspectionLimitExceeded {
            actual: text.len(),
            maximum,
        });
    }

    let counts = inspect_line_counts(text);
    if counts.nonblank == 0 {
        return Ok(DelimitedTextInspection {
            physical_line_count: counts.physical,
            blank_line_count: counts.blank,
            nonblank_line_count: counts.nonblank,
            complete: true,
            delimiter: DelimiterFinding::Unsupported,
            region: None,
            shape: GenericTableShape::Empty,
            reasons: vec![TableShapeReason::NoUsableTable],
            quoted_lines: counts.quoted,
        });
    }

    if counts.quoted.total_count > 0 {
        return Ok(DelimitedTextInspection {
            physical_line_count: counts.physical,
            blank_line_count: counts.blank,
            nonblank_line_count: counts.nonblank,
            complete: true,
            delimiter: DelimiterFinding::Unsupported,
            region: None,
            shape: GenericTableShape::MixedOrUnsupported,
            reasons: vec![TableShapeReason::QuotedInput],
            quoted_lines: counts.quoted,
        });
    }

    let comma = inspect_delimiter_candidate(text, ',');
    let tab = inspect_delimiter_candidate(text, '\t');
    let selection = select_delimiter(comma, tab);

    let Some(candidate) = selection.region else {
        return Ok(DelimitedTextInspection {
            physical_line_count: counts.physical,
            blank_line_count: counts.blank,
            nonblank_line_count: counts.nonblank,
            complete: true,
            delimiter: selection.finding,
            region: None,
            shape: GenericTableShape::MixedOrUnsupported,
            reasons: vec![selection.reason],
            quoted_lines: counts.quoted,
        });
    };

    let delimiter = selection.delimiter;
    let built = inspect_selected_region(text, delimiter, candidate);
    let (shape, reasons) = classify_region(text, delimiter, &built);

    Ok(DelimitedTextInspection {
        physical_line_count: counts.physical,
        blank_line_count: counts.blank,
        nonblank_line_count: counts.nonblank,
        complete: true,
        delimiter: selection.finding,
        region: Some(built.public),
        shape,
        reasons,
        quoted_lines: counts.quoted,
    })
}

#[derive(Default)]
struct LineCounts {
    physical: usize,
    blank: usize,
    nonblank: usize,
    quoted: BoundedLineEvidence,
}

fn inspect_line_counts(text: &str) -> LineCounts {
    let mut counts = LineCounts::default();

    for (line_index, line) in text.lines().enumerate() {
        counts.physical += 1;
        if line.trim().is_empty() {
            counts.blank += 1;
        } else {
            counts.nonblank += 1;
            if line.contains('"') {
                counts.quoted.record(line_index + 1);
            }
        }
    }

    counts
}

#[derive(Clone, Copy, Debug)]
struct RegionCandidate {
    first_line: usize,
    last_line: usize,
    field_count: usize,
    numeric_rows: usize,
    nonnumeric_rows: usize,
}

impl RegionCandidate {
    fn new(line_number: usize, field_count: usize, fully_numeric: bool) -> Self {
        Self {
            first_line: line_number,
            last_line: line_number,
            field_count,
            numeric_rows: usize::from(fully_numeric),
            nonnumeric_rows: usize::from(!fully_numeric),
        }
    }

    fn extend(&mut self, line_number: usize, fully_numeric: bool) {
        self.last_line = line_number;
        if fully_numeric {
            self.numeric_rows += 1;
        } else {
            self.nonnumeric_rows += 1;
        }
    }

    fn line_count(self) -> usize {
        self.numeric_rows + self.nonnumeric_rows
    }

    fn is_plausible(self) -> bool {
        self.field_count >= 2 && self.line_count() >= 2 && self.numeric_rows > 0
    }
}

#[derive(Default)]
struct DelimiterAnalysis {
    best: Option<RegionCandidate>,
    equally_strong_regions: bool,
}

impl DelimiterAnalysis {
    fn consider(&mut self, candidate: RegionCandidate) {
        if !candidate.is_plausible() {
            return;
        }

        let Some(best) = self.best else {
            self.best = Some(candidate);
            return;
        };

        match compare_region_strength(candidate, best) {
            Ordering::Greater => {
                self.best = Some(candidate);
                self.equally_strong_regions = false;
            }
            Ordering::Equal => {
                self.equally_strong_regions = true;
                if candidate.first_line < best.first_line {
                    self.best = Some(candidate);
                }
            }
            Ordering::Less => {}
        }
    }
}

fn compare_region_strength(left: RegionCandidate, right: RegionCandidate) -> Ordering {
    left.numeric_rows
        .cmp(&right.numeric_rows)
        .then_with(|| left.line_count().cmp(&right.line_count()))
}

fn inspect_delimiter_candidate(text: &str, delimiter: char) -> DelimiterAnalysis {
    let mut analysis = DelimiterAnalysis::default();
    let mut current: Option<RegionCandidate> = None;

    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        if line.trim().is_empty() {
            if let Some(candidate) = current.take() {
                analysis.consider(candidate);
            }
            continue;
        }

        let row = inspect_row(line, delimiter);
        if row.field_count < 2 {
            if let Some(candidate) = current.take() {
                analysis.consider(candidate);
            }
            continue;
        }

        match current.as_mut() {
            Some(candidate)
                if candidate.field_count == row.field_count
                    && candidate.last_line + 1 == line_number =>
            {
                candidate.extend(line_number, row.fully_numeric);
            }
            _ => {
                if let Some(candidate) = current.take() {
                    analysis.consider(candidate);
                }
                current = Some(RegionCandidate::new(
                    line_number,
                    row.field_count,
                    row.fully_numeric,
                ));
            }
        }
    }

    if let Some(candidate) = current {
        analysis.consider(candidate);
    }

    analysis
}

struct DelimiterSelection {
    finding: DelimiterFinding,
    delimiter: char,
    region: Option<RegionCandidate>,
    reason: TableShapeReason,
}

fn select_delimiter(comma: DelimiterAnalysis, tab: DelimiterAnalysis) -> DelimiterSelection {
    match (comma.best, tab.best) {
        (None, None) => unsupported_selection(),
        (Some(candidate), None) => select_one(',', DelimiterFinding::Comma, candidate, comma),
        (None, Some(candidate)) => select_one('\t', DelimiterFinding::Tab, candidate, tab),
        (Some(comma_candidate), Some(tab_candidate)) => {
            match compare_region_strength(comma_candidate, tab_candidate) {
                Ordering::Greater => {
                    select_one(',', DelimiterFinding::Comma, comma_candidate, comma)
                }
                Ordering::Less => select_one('\t', DelimiterFinding::Tab, tab_candidate, tab),
                Ordering::Equal => DelimiterSelection {
                    finding: DelimiterFinding::Ambiguous,
                    delimiter: ',',
                    region: None,
                    reason: TableShapeReason::AmbiguousDelimiter,
                },
            }
        }
    }
}

fn select_one(
    delimiter: char,
    finding: DelimiterFinding,
    candidate: RegionCandidate,
    analysis: DelimiterAnalysis,
) -> DelimiterSelection {
    if analysis.equally_strong_regions {
        DelimiterSelection {
            finding: DelimiterFinding::Ambiguous,
            delimiter,
            region: None,
            reason: TableShapeReason::AmbiguousTableRegion,
        }
    } else {
        DelimiterSelection {
            finding,
            delimiter,
            region: Some(candidate),
            reason: TableShapeReason::NamedFiniteRectangle,
        }
    }
}

fn unsupported_selection() -> DelimiterSelection {
    DelimiterSelection {
        finding: DelimiterFinding::Unsupported,
        delimiter: ',',
        region: None,
        reason: TableShapeReason::UnsupportedDelimiter,
    }
}

struct RowInspection {
    field_count: usize,
    fully_numeric: bool,
    has_empty: bool,
    has_non_finite: bool,
}

fn inspect_row(line: &str, delimiter: char) -> RowInspection {
    let mut field_count = 0;
    let mut fully_numeric = true;
    let mut has_empty = false;
    let mut has_non_finite = false;

    for field in line.split(delimiter) {
        field_count += 1;
        let value = field.trim();
        if value.is_empty() {
            fully_numeric = false;
            has_empty = true;
            continue;
        }

        match value.parse::<f64>() {
            Ok(number) if number.is_finite() => {}
            Ok(_) => {
                fully_numeric = false;
                has_non_finite = true;
            }
            Err(_) => fully_numeric = false,
        }
    }

    RowInspection {
        field_count,
        fully_numeric,
        has_empty,
        has_non_finite,
    }
}

struct BuiltRegion {
    public: TableRegionInspection,
    body_nonnumeric_count: usize,
    unique_named_header: bool,
    duplicate_header: bool,
}

fn inspect_selected_region(text: &str, delimiter: char, candidate: RegionCandidate) -> BuiltRegion {
    let mut header_candidate_lines = BoundedLineEvidence::default();
    let mut header_labels = Vec::new();
    let mut fully_numeric_row_count = 0;
    let mut nonnumeric_row_count = 0;
    let mut nonnumeric_lines = BoundedLineEvidence::default();
    let mut metadata_lines = BoundedLineEvidence::default();
    let mut inconsistent_width_lines = BoundedLineEvidence::default();
    let mut additional_content_lines = BoundedLineEvidence::default();
    let mut empty_numeric_cell_lines = BoundedLineEvidence::default();
    let mut non_finite_numeric_lines = BoundedLineEvidence::default();
    let mut body_nonnumeric_count = 0;
    let mut saw_numeric = false;

    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        if line.trim().is_empty() {
            continue;
        }

        if line_number < candidate.first_line {
            metadata_lines.record(line_number);
            continue;
        }

        if line_number > candidate.last_line {
            additional_content_lines.record(line_number);
            if line.split(delimiter).count() != candidate.field_count {
                inconsistent_width_lines.record(line_number);
            }
            continue;
        }

        let row = inspect_row(line, delimiter);
        if row.fully_numeric {
            saw_numeric = true;
            fully_numeric_row_count += 1;
            continue;
        }

        nonnumeric_row_count += 1;
        nonnumeric_lines.record(line_number);
        if row.has_empty {
            empty_numeric_cell_lines.record(line_number);
        }
        if row.has_non_finite {
            non_finite_numeric_lines.record(line_number);
        }
        if !saw_numeric {
            header_candidate_lines.record(line_number);
            if header_candidate_lines.total_count == 1 {
                header_labels = line
                    .split(delimiter)
                    .map(|field| field.trim().to_string())
                    .collect();
            } else {
                header_labels.clear();
            }
        } else {
            body_nonnumeric_count += 1;
        }
    }

    let unique_header = header_candidate_lines.total_count == 1;
    let nonempty_header = unique_header
        && !header_labels.is_empty()
        && header_labels.iter().all(|label| !label.is_empty());
    let duplicate_header = nonempty_header && has_duplicate_labels(&header_labels);

    BuiltRegion {
        public: TableRegionInspection {
            first_line: candidate.first_line,
            last_line: candidate.last_line,
            stable_field_count: candidate.field_count,
            header_candidate_lines,
            header_labels,
            fully_numeric_row_count,
            nonnumeric_row_count,
            nonnumeric_lines,
            metadata_lines,
            inconsistent_width_lines,
            additional_content_lines,
            empty_numeric_cell_lines,
            non_finite_numeric_lines,
        },
        body_nonnumeric_count,
        unique_named_header: nonempty_header && !duplicate_header,
        duplicate_header,
    }
}

fn has_duplicate_labels(labels: &[String]) -> bool {
    let mut seen = HashSet::with_capacity(labels.len());
    labels.iter().any(|label| !seen.insert(label.as_str()))
}

fn classify_region(
    text: &str,
    delimiter: char,
    built: &BuiltRegion,
) -> (GenericTableShape, Vec<TableShapeReason>) {
    let region = &built.public;
    let valid_rectangle = built.unique_named_header
        && region.fully_numeric_row_count > 0
        && built.body_nonnumeric_count == 0
        && region.inconsistent_width_lines.total_count == 0
        && region.additional_content_lines.total_count == 0;

    if valid_rectangle {
        let mut reasons = vec![TableShapeReason::NamedFiniteRectangle];
        if region.metadata_lines.total_count > 0 {
            reasons.push(TableShapeReason::MetadataBeforeTable);
        }

        if is_numeric_matrix(text, delimiter, region) {
            reasons.push(TableShapeReason::MonotonicSiblingMatrix);
            return (GenericTableShape::NumericMatrix, reasons);
        }

        return (GenericTableShape::NumericNarrowTable, reasons);
    }

    let mut reasons = Vec::new();
    if region.header_candidate_lines.total_count == 0 {
        push_reason(&mut reasons, TableShapeReason::MissingNamedHeader);
    } else if region.header_candidate_lines.total_count > 1 {
        push_reason(&mut reasons, TableShapeReason::MultipleHeaderRows);
    } else if region.header_labels.iter().any(String::is_empty) {
        push_reason(&mut reasons, TableShapeReason::EmptyHeaderLabel);
    }
    if built.duplicate_header {
        push_reason(&mut reasons, TableShapeReason::DuplicateHeaderLabel);
    }
    if built.body_nonnumeric_count > 0 {
        push_reason(&mut reasons, TableShapeReason::NonnumericBody);
    }
    if region.inconsistent_width_lines.total_count > 0 {
        push_reason(&mut reasons, TableShapeReason::InconsistentFieldCount);
    }
    if region.additional_content_lines.total_count > 0 {
        push_reason(&mut reasons, TableShapeReason::AdditionalTableContent);
    }
    if region.empty_numeric_cell_lines.total_count > 0 {
        push_reason(&mut reasons, TableShapeReason::EmptyNumericCell);
    }
    if region.non_finite_numeric_lines.total_count > 0 {
        push_reason(&mut reasons, TableShapeReason::NonFiniteNumericCell);
    }
    if region.metadata_lines.total_count > 0 {
        push_reason(&mut reasons, TableShapeReason::MetadataBeforeTable);
    }

    (GenericTableShape::MixedOrUnsupported, reasons)
}

fn push_reason(reasons: &mut Vec<TableShapeReason>, reason: TableShapeReason) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn is_numeric_matrix(text: &str, delimiter: char, region: &TableRegionInspection) -> bool {
    region.stable_field_count >= 3
        && region.fully_numeric_row_count >= 2
        && sibling_headers_match(&region.header_labels)
        && first_column_is_strictly_monotonic(text, delimiter, region)
}

fn sibling_headers_match(labels: &[String]) -> bool {
    if labels.len() < 3 {
        return false;
    }

    for left_index in 1..labels.len() {
        let Some((left_template, left_digits)) = sibling_template(&labels[left_index]) else {
            continue;
        };

        for right_label in &labels[left_index + 1..] {
            let Some((right_template, right_digits)) = sibling_template(right_label) else {
                continue;
            };
            if left_template == right_template && left_digits != right_digits {
                if sibling_template(&labels[0])
                    .is_some_and(|(first_template, _)| first_template == left_template)
                {
                    continue;
                }
                return true;
            }
        }
    }

    false
}

fn sibling_template(label: &str) -> Option<(String, String)> {
    let bytes = label.as_bytes();
    let start = bytes.iter().position(u8::is_ascii_digit)?;
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if bytes[end..].iter().any(u8::is_ascii_digit) || (start == 0 && end == bytes.len()) {
        return None;
    }

    let mut template = String::with_capacity(label.len() - (end - start) + 1);
    template.push_str(&label[..start]);
    template.push('#');
    template.push_str(&label[end..]);

    Some((template, label[start..end].to_string()))
}

fn first_column_is_strictly_monotonic(
    text: &str,
    delimiter: char,
    region: &TableRegionInspection,
) -> bool {
    let Some(header_line) = region.header_candidate_lines.example_lines.first().copied() else {
        return false;
    };
    let mut previous = None;
    let mut direction = 0_i8;
    let mut value_count = 0;

    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        if line_number < region.first_line
            || line_number > region.last_line
            || line_number == header_line
        {
            continue;
        }

        let Some(value) = line
            .split(delimiter)
            .next()
            .and_then(|field| field.trim().parse::<f64>().ok())
            .filter(|value| value.is_finite())
        else {
            return false;
        };

        if let Some(previous_value) = previous {
            let next_direction = if value > previous_value {
                1
            } else if value < previous_value {
                -1
            } else {
                return false;
            };
            if direction != 0 && direction != next_direction {
                return false;
            }
            direction = next_direction;
        }

        previous = Some(value);
        value_count += 1;
    }

    value_count >= 2 && direction != 0
}

#[cfg(test)]
mod tests {
    use crate::{ByteOrderMark, EncodingInspection, TextEncoding};

    use super::{
        assess_simple_csv_compatibility, inspect_delimited_text, inspect_delimited_text_with_limit,
        DelimitedInspectionError, DelimiterFinding, GenericTableShape, SimpleCsvCompatibility,
        TableShapeReason,
    };

    fn inspect(text: &str) -> super::DelimitedTextInspection {
        inspect_delimited_text(text).expect("inline decoded text should inspect")
    }

    fn encoding(text: &str, text_encoding: TextEncoding, bom: ByteOrderMark) -> EncodingInspection {
        let bom_len = match bom {
            ByteOrderMark::None => 0,
            ByteOrderMark::Utf8 => 3,
            ByteOrderMark::Utf16Le | ByteOrderMark::Utf16Be => 2,
        };
        let payload_len = match text_encoding {
            TextEncoding::Utf8 => text.len(),
            TextEncoding::Utf16Le | TextEncoding::Utf16Be => text.encode_utf16().count() * 2,
        };

        EncodingInspection {
            text: text.to_string(),
            encoding: text_encoding,
            bom,
            original_byte_len: bom_len + payload_len,
        }
    }

    fn compatibility(
        text: &str,
        text_encoding: TextEncoding,
        bom: ByteOrderMark,
    ) -> SimpleCsvCompatibility {
        let table = inspect(text);
        assess_simple_csv_compatibility(&encoding(text, text_encoding, bom), &table)
    }

    #[test]
    fn comma_named_numeric_table_selects_comma() {
        let result = inspect("A,B\n1,2\n3,4\n");

        assert_eq!(result.delimiter, DelimiterFinding::Comma);
        assert_eq!(result.shape, GenericTableShape::NumericNarrowTable);
    }

    #[test]
    fn tab_named_numeric_table_selects_tab() {
        let result = inspect("A\tB\n1\t2\n3\t4\n");

        assert_eq!(result.delimiter, DelimiterFinding::Tab);
        assert_eq!(result.shape, GenericTableShape::NumericNarrowTable);
    }

    #[test]
    fn isolated_comma_line_is_insufficient() {
        assert_eq!(inspect("A,B\n").delimiter, DelimiterFinding::Unsupported);
    }

    #[test]
    fn isolated_tab_line_is_insufficient() {
        assert_eq!(inspect("A\tB\n").delimiter, DelimiterFinding::Unsupported);
    }

    #[test]
    fn equally_plausible_comma_and_tab_regions_are_ambiguous() {
        let result = inspect("A,B\n1,2\n\nC\tD\n3\t4\n");

        assert_eq!(result.delimiter, DelimiterFinding::Ambiguous);
        assert!(result
            .reasons
            .contains(&TableShapeReason::AmbiguousDelimiter));
    }

    #[test]
    fn text_without_plausible_delimiter_is_unsupported() {
        let result = inspect("metadata\nvalue\n");

        assert_eq!(result.delimiter, DelimiterFinding::Unsupported);
        assert!(result
            .reasons
            .contains(&TableShapeReason::UnsupportedDelimiter));
    }

    #[test]
    fn repeated_inspection_is_equal() {
        let text = "A,B\n1,2\n3,4\n";

        assert_eq!(inspect(text), inspect(text));
    }

    #[test]
    fn line_counts_region_numbers_metadata_and_labels_are_reported() {
        let result = inspect("Instrument\n\n Axis , Value \n1,2\n3,4\n");
        let region = result.region.expect("region should be selected");

        assert_eq!(result.physical_line_count, 5);
        assert_eq!(result.blank_line_count, 1);
        assert_eq!(result.nonblank_line_count, 4);
        assert!(result.complete);
        assert_eq!((region.first_line, region.last_line), (3, 5));
        assert_eq!(region.stable_field_count, 2);
        assert_eq!(region.header_candidate_lines.example_lines, vec![3]);
        assert_eq!(region.header_labels, vec!["Axis", "Value"]);
        assert_eq!(region.metadata_lines.example_lines, vec![1]);
    }

    #[test]
    fn inconsistent_body_width_is_reported() {
        let result = inspect("A,B\n1,2\n3\n4,5\n");
        let region = result.region.expect("strongest region should be selected");

        assert_eq!(region.inconsistent_width_lines.example_lines, vec![3]);
        assert_eq!(result.shape, GenericTableShape::MixedOrUnsupported);
        assert!(result
            .reasons
            .contains(&TableShapeReason::InconsistentFieldCount));
    }

    #[test]
    fn empty_body_cell_is_nonnumeric() {
        let result = inspect("A,B\n1,\n2,3\n");
        let region = result.region.expect("region should be selected");

        assert_eq!(region.empty_numeric_cell_lines.example_lines, vec![2]);
        assert_eq!(result.shape, GenericTableShape::MixedOrUnsupported);
    }

    #[test]
    fn non_finite_body_value_is_nonnumeric() {
        let result = inspect("A,B\n1,NaN\n2,3\n");
        let region = result.region.expect("region should be selected");

        assert_eq!(region.non_finite_numeric_lines.example_lines, vec![2]);
        assert!(result
            .reasons
            .contains(&TableShapeReason::NonFiniteNumericCell));
    }

    #[test]
    fn one_header_followed_by_numeric_rows_is_structurally_accepted() {
        let result = inspect("name,value\n1,2\n");
        let region = result.region.expect("region should be selected");

        assert_eq!(region.header_candidate_lines.total_count, 1);
        assert_eq!(region.fully_numeric_row_count, 1);
        assert_eq!(result.shape, GenericTableShape::NumericNarrowTable);
    }

    #[test]
    fn header_and_unit_row_are_not_silently_removed() {
        let result = inspect("Label,Reading,Condition\nu,v,w\n0,1,2\n1,2,3\n");
        let region = result.region.expect("region should be selected");

        assert_eq!(region.header_candidate_lines.example_lines, vec![1, 2]);
        assert!(region.header_labels.is_empty());
        assert_eq!(result.shape, GenericTableShape::MixedOrUnsupported);
        assert!(result
            .reasons
            .contains(&TableShapeReason::MultipleHeaderRows));
    }

    #[test]
    fn headerless_numeric_rows_are_not_current_parser_compatible() {
        let text = "1,2\n3,4\n";
        let result = inspect(text);

        assert_eq!(result.shape, GenericTableShape::MixedOrUnsupported);
        assert!(result
            .reasons
            .contains(&TableShapeReason::MissingNamedHeader));
        assert_eq!(
            compatibility(text, TextEncoding::Utf8, ByteOrderMark::None),
            SimpleCsvCompatibility::Incompatible
        );
    }

    #[test]
    fn equally_strong_named_regions_are_not_guessed() {
        let result = inspect("A,B\n1,2\n3,4\n\nC,D\n5,6\n7,8\n");

        assert_eq!(result.delimiter, DelimiterFinding::Ambiguous);
        assert!(result.region.is_none());
        assert!(result
            .reasons
            .contains(&TableShapeReason::AmbiguousTableRegion));
    }

    #[test]
    fn nonnumeric_row_inside_numeric_body_is_unsupported() {
        let result = inspect("A,B\n1,2\ninvalid,3\n4,5\n");
        let region = result.region.expect("region should be selected");

        assert_eq!(region.nonnumeric_lines.example_lines, vec![1, 3]);
        assert_eq!(result.shape, GenericTableShape::MixedOrUnsupported);
        assert!(result.reasons.contains(&TableShapeReason::NonnumericBody));
    }

    #[test]
    fn empty_input_is_empty_shape() {
        assert_eq!(inspect("").shape, GenericTableShape::Empty);
    }

    #[test]
    fn whitespace_only_input_is_empty_shape() {
        let result = inspect(" \n\t\n");

        assert_eq!(result.shape, GenericTableShape::Empty);
        assert_eq!(result.blank_line_count, 2);
    }

    #[test]
    fn normal_two_column_table_is_narrow() {
        assert_eq!(
            inspect("A,B\n1,2\n3,4\n").shape,
            GenericTableShape::NumericNarrowTable
        );
    }

    #[test]
    fn normal_three_column_table_without_sibling_pattern_is_narrow() {
        assert_eq!(
            inspect("A,B,C\n1,2,3\n2,3,4\n").shape,
            GenericTableShape::NumericNarrowTable
        );
    }

    #[test]
    fn monotonic_axis_and_sibling_headers_form_matrix() {
        let result = inspect("axis,series 1,series 2\n1,10,11\n2,12,13\n3,14,15\n");

        assert_eq!(result.shape, GenericTableShape::NumericMatrix);
        assert!(result
            .reasons
            .contains(&TableShapeReason::MonotonicSiblingMatrix));
    }

    #[test]
    fn arbitrary_neutral_sibling_labels_form_matrix() {
        let result = inspect("q,block 7(v),block 9(v)\n3,1,2\n2,3,4\n1,5,6\n");

        assert_eq!(result.shape, GenericTableShape::NumericMatrix);
    }

    #[test]
    fn matrix_classification_is_independent_of_domain_vocabulary() {
        let neutral = inspect("axis,series 4(unit),series 5(unit)\n1,10,11\n2,12,13\n");
        let domain_labels =
            inspect("Wavelength(nm),kinetics 4(Abs),kinetics 5(Abs)\n1,10,11\n2,12,13\n");

        assert_eq!(neutral.shape, GenericTableShape::NumericMatrix);
        assert_eq!(domain_labels.shape, neutral.shape);
    }

    #[test]
    fn unrelated_peer_pattern_can_still_establish_matrix() {
        let result =
            inspect("series 0,series 1,series 2,block 3,block 4\n1,10,11,12,13\n2,14,15,16,17\n");

        assert_eq!(result.shape, GenericTableShape::NumericMatrix);
    }

    #[test]
    fn nonmonotonic_first_column_prevents_matrix() {
        let result = inspect("axis,series 1,series 2\n1,10,11\n1,12,13\n");

        assert_eq!(result.shape, GenericTableShape::NumericNarrowTable);
    }

    #[test]
    fn monotonic_first_column_without_sibling_syntax_is_narrow() {
        let result = inspect("axis,left,right\n1,10,11\n2,12,13\n");

        assert_eq!(result.shape, GenericTableShape::NumericNarrowTable);
    }

    #[test]
    fn first_header_sharing_peer_pattern_prevents_matrix() {
        let result = inspect("series 0,series 1,series 2\n1,10,11\n2,12,13\n");

        assert_eq!(result.shape, GenericTableShape::NumericNarrowTable);
    }

    #[test]
    fn quoted_comma_field_is_unsupported() {
        let result = inspect("A,B\n1,\"2\"\n");

        assert_eq!(result.delimiter, DelimiterFinding::Unsupported);
        assert_eq!(result.shape, GenericTableShape::MixedOrUnsupported);
        assert_eq!(result.quoted_lines.example_lines, vec![2]);
        assert!(result.region.is_none());
    }

    #[test]
    fn quoted_tab_field_is_unsupported() {
        let result = inspect("A\tB\n1\t\"2\"\n");

        assert_eq!(result.delimiter, DelimiterFinding::Unsupported);
        assert!(result.reasons.contains(&TableShapeReason::QuotedInput));
    }

    #[test]
    fn quote_requiring_multiline_interpretation_is_unsupported() {
        let result = inspect("A,B\n1,\"two\nlines\"\n2,3\n");

        assert_eq!(result.shape, GenericTableShape::MixedOrUnsupported);
        assert_eq!(result.quoted_lines.example_lines, vec![2, 3]);
        assert!(result.region.is_none());
    }

    #[test]
    fn utf8_no_bom_comma_narrow_table_is_compatible_as_is() {
        assert_eq!(
            compatibility("A,B\n1,2\n", TextEncoding::Utf8, ByteOrderMark::None),
            SimpleCsvCompatibility::CompatibleAsIs
        );
    }

    #[test]
    fn utf8_bom_narrow_comma_table_requires_normalization() {
        assert_eq!(
            compatibility("A,B\n1,2\n", TextEncoding::Utf8, ByteOrderMark::Utf8),
            SimpleCsvCompatibility::RequiresExplicitNormalization
        );
    }

    #[test]
    fn utf16le_narrow_comma_table_requires_normalization() {
        assert_eq!(
            compatibility("A,B\n1,2\n", TextEncoding::Utf16Le, ByteOrderMark::Utf16Le),
            SimpleCsvCompatibility::RequiresExplicitNormalization
        );
    }

    #[test]
    fn utf16be_narrow_comma_table_requires_normalization() {
        assert_eq!(
            compatibility("A,B\n1,2\n", TextEncoding::Utf16Be, ByteOrderMark::Utf16Be),
            SimpleCsvCompatibility::RequiresExplicitNormalization
        );
    }

    #[test]
    fn utf8_tab_narrow_table_requires_normalization() {
        assert_eq!(
            compatibility("A\tB\n1\t2\n", TextEncoding::Utf8, ByteOrderMark::None),
            SimpleCsvCompatibility::RequiresExplicitNormalization
        );
    }

    #[test]
    fn metadata_before_narrow_table_requires_normalization() {
        assert_eq!(
            compatibility("note\nA,B\n1,2\n", TextEncoding::Utf8, ByteOrderMark::None),
            SimpleCsvCompatibility::RequiresExplicitNormalization
        );
    }

    #[test]
    fn matrix_is_incompatible() {
        assert_eq!(
            compatibility(
                "axis,series 1,series 2\n1,10,11\n2,12,13\n",
                TextEncoding::Utf8,
                ByteOrderMark::None
            ),
            SimpleCsvCompatibility::Incompatible
        );
    }

    #[test]
    fn mixed_structure_is_incompatible() {
        assert_eq!(
            compatibility(
                "A,B\n1,2\ninvalid,3\n",
                TextEncoding::Utf8,
                ByteOrderMark::None
            ),
            SimpleCsvCompatibility::Incompatible
        );
    }

    #[test]
    fn empty_structure_is_incompatible() {
        assert_eq!(
            compatibility("", TextEncoding::Utf8, ByteOrderMark::None),
            SimpleCsvCompatibility::Incompatible
        );
    }

    #[test]
    fn more_than_two_columns_remain_generic_and_compatible() {
        assert_eq!(
            compatibility(
                "A,B,C\n1,2,3\n2,3,4\n",
                TextEncoding::Utf8,
                ByteOrderMark::None
            ),
            SimpleCsvCompatibility::CompatibleAsIs
        );
    }

    #[test]
    fn decoded_text_limit_is_shared_and_checked_before_inspection() {
        assert_eq!(
            inspect_delimited_text_with_limit("ab", 1),
            Err(DelimitedInspectionError::InspectionLimitExceeded {
                actual: 2,
                maximum: 1,
            })
        );
    }

    #[test]
    fn diagnostic_line_examples_are_bounded() {
        let result =
            inspect("A,B\n1,2\nbad,3\nbad,4\nbad,5\nbad,6\nbad,7\nbad,8\nbad,9\nbad,10\nbad,11\n");
        let region = result.region.expect("region should be selected");

        assert_eq!(region.nonnumeric_lines.total_count, 10);
        assert_eq!(region.nonnumeric_lines.example_lines.len(), 8);
        assert!(region.nonnumeric_lines.additional_examples_omitted);
    }
}
