# Phase 2.15 CLI Kinetics Analyze Design

## Summary

Design the first user-facing v0.1 CLI command for deterministic kinetics
analysis:

```sh
deepseek-science kinetics analyze \
  --input <path> \
  --time-column <column> \
  --concentration-column <column>
```

The command should read one user-provided CSV file, parse it through the generic
`DataTable` adapter, run the existing chemistry kinetics pipeline, and print a
concise deterministic plain-text summary. This phase is design-only.

## User Goal

A CLI user wants to analyze a small kinetics CSV without writing Rust code:

1. provide a CSV path,
2. provide exact time and concentration column names,
3. receive fitted first-order and second-order results,
4. see the MVP heuristic preference and deterministic review warnings.

## Minimal Deliverable

- Add one command path: `kinetics analyze`.
- Manually parse the three required flags with `std::env::args`.
- Read exactly one UTF-8 CSV file in the CLI layer.
- Convert CSV text to `DataTable` with `parse_simple_numeric_csv`.
- Run:
  - `KineticsColumns`,
  - `ValidatedKineticsInput`,
  - `KineticsAnalysisResult`,
  - `KineticsArtifactProposal` only if useful for future metadata visibility.
- Print deterministic plain text.
- Return simple exit codes.

## Non-goals

- No source implementation in this phase.
- No `clap` or other argument parsing dependency.
- No config files or workspace loading.
- No storage writes or artifact persistence.
- No model calls, tool execution, plotting, report generation, UI, or
  TypeScript.
- No automatic column detection, unit parsing, delimiter detection, or full CSV
  dialect support.
- No JSON output in v0.1.

## Existing Building Blocks

| Area | Existing item | CLI use |
| --- | --- | --- |
| CLI entry | `run_cli(std::env::args())` | Preserve manual parser style |
| CSV adapter | `parse_simple_numeric_csv(&str)` | Generic `&str -> DataTable` boundary |
| Table | `DataTable`, `DataColumn` | Chemistry-neutral in-memory input |
| Kinetics columns | `KineticsColumns` | Exact caller-provided column names |
| Input validation | `ValidatedKineticsInput` | Reject invalid kinetics rows |
| Analysis | `KineticsAnalysisResult` | Deterministic comparison plus review |
| Artifact proposal | `KineticsArtifactProposal` | Optional in-memory metadata only |

## Command Shape

Only this command is in scope:

```sh
deepseek-science kinetics analyze \
  --input kinetics.csv \
  --time-column time_s \
  --concentration-column concentration_mol_l
```

Do not add sibling kinetics subcommands yet. Future commands can be added when
there is a real second user workflow.

## Argument Parsing Rules

The v0.1 implementation should keep the current no-`clap` style and parse
arguments manually.

Rules:

- `deepseek-science kinetics analyze` selects the command.
- `--input`, `--time-column`, and `--concentration-column` are required.
- Each required flag must be followed by a non-empty value.
- Unknown flags or positional extras are user errors.
- Duplicate flags should be rejected with a concise error and usage text. This
  avoids hidden last-one-wins behavior in scientific runs.
- Option names are exact; no fuzzy matching or aliases.
- Missing subcommand prints usage and exits non-zero once `kinetics` becomes a
  command namespace.

Recommended usage text:

```text
Usage:
  deepseek-science kinetics analyze --input <path> --time-column <column> --concentration-column <column>
```

## File Reading Boundary

File IO belongs only in `deepseek-science-cli`.

The command may:

- read the user-provided `--input` path into a UTF-8 string,
- include the user-provided path in output or error text after avoiding extra
  normalization,
- fail with a user/input error when the path cannot be read or is not valid
  UTF-8 text.

The command must not:

- write output files,
- create temp files,
- write logs or caches,
- pass file paths into chemistry analysis,
- move file reading into `deepseek-science-common` or
  `deepseek-science-chemistry`.

## CSV Adapter Boundary

The CLI should call:

```text
parse_simple_numeric_csv(&csv_text) -> Result<DataTable, CommonError>
```

Responsibilities:

| Layer | Responsibility |
| --- | --- |
| CLI | Read file and format user-facing errors |
| Common CSV adapter | Parse narrow numeric CSV text into `DataTable` |
| Chemistry | Validate kinetics columns and analyze |

The CSV adapter must remain generic. It should not know about time,
concentration, kinetics, units, or workflow IDs.

## Kinetics Pipeline Boundary

After CSV parsing, the CLI should construct and run:

```text
DataTable
  -> KineticsColumns
  -> ValidatedKineticsInput
  -> KineticsAnalysisResult
  -> optional KineticsArtifactProposal
```

`ArtifactManifest` creation can be deferred for the first CLI command because
v0.1 does not persist artifacts or need a caller-provided artifact id. If added
later, it must remain in-memory and deterministic.

## Output Format

Use concise deterministic plain text. Suggested success output:

```text
DeepSeek_Science kinetics analyze
input: kinetics.csv
time_column: time_s
concentration_column: concentration_mol_l
valid_points: 3
rejected_rows: 0
first_order.k: 0.250000
first_order.r_squared: 1.000000
second_order.k: 0.324361
second_order.r_squared: 0.982000
preferred_model: first_order
comparison_basis: finite_r_squared_mvp_heuristic
preferred_note: Preferred by MVP r_squared heuristic; not definitive model selection.
review_status: passed
review_findings: none
```

Formatting rules:

- Stable field order.
- No long natural-language scientific prose.
- Do not claim a definitive reaction order.
- Print review findings as stable short lines when present.
- For rejected rows, report the count and warning summary. Future CLI versions
  may add row-number details.

## Exit Codes

Keep exit codes simple:

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | User/input error |
| `2` | Internal error or unexpected invariant failure |

Examples of exit code `1`:

- missing required argument,
- unknown argument,
- file read failure,
- invalid CSV,
- missing selected column,
- not enough valid kinetics rows.

## Error Handling

Errors should be short and actionable, without stack traces.

| Source | Example CLI message |
| --- | --- |
| Argument parser | `error: missing required argument --input` |
| File read | `error: could not read input file: <path>` |
| CSV parse | `error: invalid CSV: row 0 column concentration_mol_l has invalid float` |
| Missing time column | `error: missing kinetics time column: time_s` |
| Missing concentration column | `error: missing kinetics concentration column: concentration_mol_l` |
| Invalid kinetics input | `error: not enough valid kinetics points after validation` |
| Fitting failure | `error: kinetics fitting failed: ...` |
| Review failure | `error: kinetics review failed deterministic consistency checks` |

Prefer stderr for errors. If the current `CliOutput` shape is kept initially,
the implementation may return error text through the existing output field with
a non-zero exit code, then add stderr support in a later CLI cleanup.

## Disk Safety Considerations

- The command reads exactly one user-specified input file.
- The common CSV parser remains `&str` only.
- The chemistry pipeline remains `DataTable` only.
- The command prints to stdout/stderr only.
- No output files.
- No storage writes.
- No artifact files.
- No generated reports.
- No logs, caches, temp dirs, or fixture creation.
- No watcher loops or repeated command execution.

## Dependency Policy

Use the standard library:

- `std::env::args` for argument parsing,
- `std::fs::read_to_string` for the one CLI file read,
- existing workspace crates for parsing and analysis.

Do not add `clap` or a CSV crate for v0.1. Add a dependency only if the command
surface or CSV dialect grows beyond the current tiny MVP.

## Testing Plan

Future implementation tests should stay small:

- unit tests for the manual argument parser,
- success path with tiny input content,
- missing required arguments,
- duplicate argument rejection,
- unknown argument rejection,
- missing file error,
- invalid CSV error,
- missing time column,
- missing concentration column,
- rejected-row warning path.

Prefer keeping parser and pipeline tests in-memory. A CLI smoke fixture should
be added only if the process boundary cannot be tested otherwise, and it must be
tiny. Do not add large fixtures, snapshot bloat, network calls, API keys, model
calls, tools, storage, or repeated CLI loops.

## Implementation Milestones

1. Add a minimal `kinetics analyze` branch to the existing CLI parser.
2. Add a small internal parsed-args type for the three required values.
3. Read `--input` with the narrow CLI file boundary.
4. Call `parse_simple_numeric_csv`.
5. Construct `KineticsColumns` and `ValidatedKineticsInput`.
6. Run `KineticsAnalysisResult::analyze`.
7. Print deterministic plain text.
8. Add focused CLI tests and one bounded validation pass.

## Deferred Work

- `--json` output and a stable JSON schema.
- Artifact persistence and storage integration.
- `ArtifactManifest` emission from the CLI.
- Project workspace loading.
- DeepSeek/model explanations.
- Plotting.
- Long-form report generation.
- Automatic column detection.
- Units system.
- Full CSV dialect support.
- Streaming large files.
- `clap` or richer CLI framework.

## Open Questions

- Should the first implementation add `stderr` to `CliOutput`, or keep current
  output plumbing and defer stderr cleanup?
- Should success output include artifact proposal hash once no persistence is
  involved, or keep the first CLI focused on scientific summary fields only?
- Should future warning output include rejected row indices immediately, or only
  rejected counts until row-number formatting is designed?
