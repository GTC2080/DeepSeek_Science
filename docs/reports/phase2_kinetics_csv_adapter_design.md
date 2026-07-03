# Phase 2.13 Kinetics CSV Adapter Design

## Summary

The first CSV adapter should be a tiny deterministic parser that turns a narrow
CSV text subset into the existing in-memory `DataTable` boundary. It must not
perform chemistry validation, read files, infer columns, persist artifacts, or
become a general-purpose CSV engine.

Recommended first boundary:

```text
&str CSV content -> DataTable -> KineticsColumns -> ValidatedKineticsInput -> existing kinetics pipeline
```

The `kinetics_csv` workflow name remains user-facing. Chemistry-specific logic
continues to live in `deepseek-science-chemistry`; CSV table parsing is generic.

## User Goal

A future CLI user should be able to provide a small CSV file plus explicit time
and concentration column names, then run the existing deterministic kinetics
pipeline without hand-constructing a `DataTable`.

## Minimal Deliverable

- Parse small UTF-8 CSV content already loaded into memory.
- Require one header row and at least one numeric data row.
- Produce a `DataTable` containing numeric `DataColumn` values in header order.
- Preserve exact header names for later explicit column selection.
- Surface structured parse errors with enough row/column context for CLI output.
- Use inline string tests only.

## Non-goals

- Full CSV standards compliance.
- File reading inside the parser.
- CSV parsing inside chemistry analysis.
- Automatic time/concentration detection.
- Units parsing.
- Streaming large files.
- Quoted multiline fields.
- Excel import behavior or formula evaluation.
- CLI implementation.
- Artifact persistence, plotting, model calls, tool calls, UI, or TypeScript.

## Existing Building Blocks

| Area | Existing building block | Planned use |
| --- | --- | --- |
| Table boundary | `DataTable`, `DataColumn` | Parser output |
| Table validation | `CommonError` | Reuse empty/duplicate/non-finite table semantics where clean |
| Kinetics validation | `KineticsColumns`, `ValidatedKineticsInput` | Explicit domain validation after parsing |
| Analysis | `KineticsAnalysisResult` | Existing deterministic pipeline |
| Artifacts | `KineticsArtifactProposal`, `ArtifactManifest` | Existing in-memory metadata mapping |
| CLI | `deepseek-science-cli` entry point | Future file-read and argument boundary |

## Input File Scope

The parser should accept CSV content as `&str`. The future CLI command should
read a file path into a string and pass that content to the parser.

This separation keeps parser tests filesystem-free and prevents file IO from
entering chemistry analysis.

## CSV Dialect Scope

MVP dialect:

- UTF-8 text.
- Comma-separated fields.
- One header row.
- Numeric data rows only.
- `\n` and `\r\n` line endings.
- Optional trailing newline.

Out of scope for the MVP:

- multiline fields,
- quoted multiline values,
- automatic delimiter detection,
- semicolon/tab dialects,
- comments,
- Excel-specific behavior,
- formula evaluation.

Simple quoted cells should be deferred for the first implementation. Supporting
quotes correctly brings escaping rules and comma-in-quote behavior; that should
wait until real input requires it or a CSV crate is justified.

## Header Rules

- The first non-empty line is not special-cased; the first line is the header.
- Missing header: structured error.
- Empty header name after trimming whitespace: structured error.
- Duplicate header name: structured error, matching exact names.
- Header order is preserved in the resulting `DataTable`.
- Header names are not lowercased, normalized, or semantically interpreted.

Whitespace policy should be conservative: trim leading/trailing whitespace
around header names and numeric cells, but preserve the resulting exact header
string for later lookup.

## Numeric Parsing Rules

- Every data field must parse as finite `f64`.
- Use standard library parsing first.
- Reject `NaN`, `inf`, `-inf`, and other non-finite values even if `f64` parses
  them.
- Do not infer schemas beyond "all columns are numeric".
- Do not evaluate formulas or expressions.

Non-positive concentration values are not parser errors. They remain kinetics
validation warnings/rejections after the caller supplies `KineticsColumns`.

## Empty Cell Rules

- Empty numeric cell after trimming: structured error.
- Do not coerce empty cells to zero.
- Do not silently drop rows.
- Empty lines in the middle of data should be rejected as malformed data.
- A trailing newline after the last data row should not create an extra row.

## Column Selection Rules

The CSV adapter does not choose chemistry columns.

Caller or CLI must pass exact names:

- `--time-column <name>`
- `--concentration-column <name>`

No fuzzy matching, case-insensitive matching, unit inference, or semantic
detection should happen in the parser.

## Row Index Mapping

- Parser data row indices are 0-based relative to the first data row after the
  header.
- For the simple single-line MVP dialect, human file line number is normally
  `data_row_index + 2`.
- Parser errors should include line number and column name/index when available.
- `ValidatedKineticsInput` should continue using 0-based `DataTable` row
  indices; the future CLI can convert them to human-readable file row numbers.

## Error Model

Prefer a small structured CSV-table error enum, conceptually:

| Error | Context to carry |
| --- | --- |
| `MissingHeader` | none |
| `NoDataRows` | none |
| `EmptyHeaderName` | column index |
| `DuplicateHeaderName` | header name |
| `InconsistentFieldCount` | line number, expected, actual |
| `EmptyNumericCell` | line number, column index, header name |
| `InvalidFloat` | line number, column index, header name, value |
| `NonFiniteFloat` | line number, column index, header name, value |
| `UnsupportedQuotedField` | line number, column index |
| `Common` | wrapped `CommonError` when building `DataColumn`/`DataTable` |

The parser should not collapse these into strings. CLI can format them later.

## Crate Boundary Plan

Preferred implementation location:

- A generic parser function from `&str` to `DataTable` may live in
  `deepseek-science-common` if it remains tiny, dependency-free, and
  domain-neutral.

Acceptable later alternative:

- A dedicated data/adapter crate if CSV support grows beyond the tiny MVP.

Avoid:

- CSV parsing inside `deepseek-science-chemistry`.
- Chemistry-specific behavior in `deepseek-science-common`.
- File reading inside the parser.
- Artifact or storage concerns inside CSV parsing.

The chemistry crate should keep accepting `DataTable` plus explicit
`KineticsColumns`.

## Disk Safety Considerations

- Parser tests use inline `&str` fixtures only.
- No test CSV files.
- No temp directories.
- No file reads or writes in parser tests.
- No generated files.
- No storage writes.
- No cargo commands for this design-only phase.

Future CLI file reading must remain a narrow, explicit boundary and should not
write derived files by default.

## Testing Plan

Future tests should be tiny and deterministic:

- simple valid CSV creates a `DataTable`,
- column order is preserved,
- duplicate header is rejected,
- empty header is rejected,
- inconsistent row width is rejected,
- empty numeric cell is rejected,
- invalid float is rejected,
- non-finite float is rejected,
- quoted cells are rejected or explicitly unsupported in MVP,
- no file IO, temp dirs, network, API keys, models, tools, or storage.

No chemistry-specific behavior should be asserted in generic parser tests.
Chemistry tests should start from the resulting `DataTable` and explicit
`KineticsColumns`.

## Future CLI Integration

Possible future command shape:

```sh
deepseek-science kinetics analyze \
  --input kinetics.csv \
  --time-column time_s \
  --concentration-column concentration_mol_l
```

CLI responsibilities:

- parse arguments,
- read the input file as UTF-8 text,
- call the CSV adapter,
- construct `KineticsColumns`,
- run existing kinetics validation and analysis,
- print a deterministic concise summary,
- return non-zero exit code on structured errors.

Parser responsibilities:

- parse CSV text to `DataTable` only.

Kinetics responsibilities:

- validate kinetics-specific input,
- reject non-positive concentrations,
- fit first-order and second-order models,
- compare by finite `r_squared` MVP heuristic,
- review results,
- produce in-memory proposal/manifest metadata.

## Deferred Work

- Full CSV standards compliance.
- CSV crate dependency.
- Streaming large files.
- Quoted and escaped fields.
- Quoted multiline cells.
- Delimiter detection.
- Excel import behavior.
- Units parsing.
- Automatic column detection.
- CLI command implementation.
- Artifact persistence.
- Plotting.
- Model-generated explanations.

A CSV crate becomes justified only when robust quoting, dialect support, large
files, streaming, or standards compliance becomes a real requirement.

## Implementation Milestones

1. Add a generic `&str -> DataTable` parser with a small structured error enum.
2. Add inline string unit tests for valid and invalid tiny CSV inputs.
3. Keep chemistry tests using `DataTable` and explicit `KineticsColumns`.
4. Add a future CLI command that reads a file and calls the parser.
5. Add CLI smoke tests only after file reading behavior is explicitly in scope.

## Open Questions

- Should the tiny parser live in `deepseek-science-common`, or should Phase 2
  start a small data-adapter crate before CSV scope grows?
- Should whitespace around header names be trimmed or preserved exactly?
- Should unsupported quotes produce a dedicated error or be rejected as invalid
  field syntax?
- Should future CLI summaries include 1-based file line numbers for both parser
  errors and kinetics rejected rows?
