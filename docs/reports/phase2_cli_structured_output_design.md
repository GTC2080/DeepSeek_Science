# Phase 2.21 CLI Structured Output Design

## Summary

Design deterministic structured JSON output for the existing command:

```sh
deepseek-science kinetics analyze \
  --input <path> \
  --time-column <column> \
  --concentration-column <column> \
  --json
```

The v0.2 target is a stable success-output contract only. Text output remains
the default, error behavior remains human-readable stderr, and the command keeps
the current no-storage, no-model, no-tool disk profile.

## User Goal

A CLI user wants the same deterministic kinetics analysis currently available
as text, but in a machine-readable form suitable for scripts, notebooks, and
future artifact boundaries.

The user should be able to run the existing command with `--json` and receive a
single JSON object on stdout when analysis succeeds.

## Current Behavior

Current `kinetics analyze` behavior:

- reads one user-provided input file in the CLI layer;
- parses the file with the generic `parse_simple_numeric_csv(&str)` adapter;
- constructs `KineticsColumns`, `ValidatedKineticsInput`, and
  `KineticsAnalysisResult`;
- prints deterministic plain text to stdout on success;
- writes concise errors to stderr on failure;
- exits non-zero for user/input errors;
- writes no output files and creates no storage records.

The current CLI does not accept `--json`; unknown flags are rejected.

## Proposed CLI Interface

Preferred v0.2 interface:

```sh
deepseek-science kinetics analyze \
  --input <path> \
  --time-column <column> \
  --concentration-column <column> \
  --json
```

Rules:

- `--json` is an optional flag for `kinetics analyze`.
- Text output remains the default when `--json` is absent.
- `--json` changes only successful output formatting.
- No output files, config files, implicit storage, model calls, or tool calls are
  introduced.
- Do not add `--format text|json` in v0.2 unless implementation proves it is
  simpler than a single boolean `--json` flag. The preferred design is only
  `--json`.

## Output Mode Rules

| Mode | Trigger | Success stdout | Failure stderr |
| --- | --- | --- | --- |
| Text | default | Existing concise human summary | Existing concise error message |
| JSON | `--json` | One deterministic JSON object | Existing concise error message |

JSON mode must not change the analysis pipeline. It should format the same
`KineticsAnalysisResult` data that text mode already uses.

## stdout / stderr Contract

- Success text mode writes the human summary to stdout.
- Success JSON mode writes JSON only to stdout.
- Failure in any mode writes a concise human-readable error to stderr.
- Success JSON stdout must not mix warnings, banners, debug text, or prose
  outside the JSON object.
- stderr must not contain Rust `Debug` dumps, panic messages, debug backtraces,
  or internal stack traces.
- `review.findings` belongs inside the JSON object; it must not be printed as
  extra stderr text on successful JSON runs.

## JSON Schema

Schema version:

```json
"schema_version": "kinetics.analysis.v1"
```

Suggested top-level shape:

```json
{
  "schema_version": "kinetics.analysis.v1",
  "command": "kinetics.analyze",
  "input": {
    "path": "<as provided by user>"
  },
  "columns": {
    "time": "time_s",
    "concentration": "concentration_mol_l"
  },
  "counts": {
    "valid_points": 4,
    "rejected_rows": 0
  },
  "fits": {
    "first_order": {
      "k": 0.6931471805599453,
      "slope": -0.6931471805599453,
      "intercept": 0.0,
      "r_squared": 1.0,
      "valid_point_count": 4
    },
    "second_order": {
      "k": 2.333333333333333,
      "slope": 2.333333333333333,
      "intercept": -0.5,
      "r_squared": 0.9142857142857143,
      "valid_point_count": 4
    }
  },
  "comparison": {
    "basis": "finite_r_squared_mvp_heuristic",
    "preferred_model": "first_order",
    "caution": "preferred_by_mvp_r_squared_heuristic_not_final_scientific_model_selection"
  },
  "review": {
    "status": "passed",
    "findings": []
  }
}
```

The numeric values above are illustrative. The contract defines field names and
semantics, not exact values for every input.

## Field Semantics

| Field | Meaning |
| --- | --- |
| `schema_version` | Stable schema identifier for this JSON contract. |
| `command` | Stable command identifier: `kinetics.analyze`. |
| `input.path` | The input path string exactly as provided by the user; do not canonicalize or derive absolute paths. |
| `columns.time` | Exact caller-provided time column name. |
| `columns.concentration` | Exact caller-provided concentration column name. |
| `counts.valid_points` | Number of positive-concentration rows analyzed. |
| `counts.rejected_rows` | Number of rows rejected by kinetics validation. |
| `fits.first_order.k` | First-order rate constant, derived as `-slope`. |
| `fits.first_order.slope` | Regression slope for `ln(concentration)` vs time. |
| `fits.first_order.intercept` | Regression intercept for first-order linearization. |
| `fits.first_order.r_squared` | Finite `r_squared` from first-order linearized regression. |
| `fits.second_order.k` | Second-order rate constant, derived as `slope`. |
| `fits.second_order.slope` | Regression slope for `1 / concentration` vs time. |
| `fits.second_order.intercept` | Regression intercept for second-order linearization. |
| `fits.second_order.r_squared` | Finite `r_squared` from second-order linearized regression. |
| `fits.*.valid_point_count` | Number of valid points used by that fit. |
| `comparison.basis` | Always `finite_r_squared_mvp_heuristic` in v0.2. |
| `comparison.preferred_model` | `first_order` or `second_order`, using the existing MVP heuristic. |
| `comparison.caution` | Stable caution label, not natural-language model output. |
| `review.status` | `passed`, `passed_with_warnings`, or `failed`. |
| `review.findings` | Ordered deterministic reviewer findings. Empty when no findings exist. |

Suggested review finding shape:

```json
{
  "severity": "warning",
  "check": "rejected_rows_visible",
  "model": null,
  "rejected_row_count": 1,
  "message": "rejected_rows_reported"
}
```

Finding messages should be stable labels where possible. If implementation
reuses current static reviewer messages, they must remain deterministic and
must not be model-generated prose.

## Floating Point Rules

JSON output must never emit invalid JSON numbers:

- no `NaN`;
- no positive or negative infinity;
- no string encoding for non-finite numbers in v0.2.

Implementation should rely on existing finite checks in the CSV parser,
`DataTable`, kinetics fitting, comparison, review, and artifact mapping where
available. Before serialization, the CLI should defensively verify all emitted
numeric analysis values are finite.

If a non-finite value is discovered at the output boundary, return an internal
or analysis error before serialization and write the error to stderr.

## Scientific Wording Rules

The JSON contract must remain scientifically cautious.

Allowed stable labels:

- `finite_r_squared_mvp_heuristic`
- `preferred_by_mvp_r_squared_heuristic`
- `not_final_scientific_model_selection`
- `preferred_by_mvp_r_squared_heuristic_not_final_scientific_model_selection`

Do not emit fields or values that imply final scientific proof, including:

- `definitive`
- `true_model`
- `proved`
- `proof`
- `final_reaction_order`

The preferred model is only the result of the Phase 2 MVP finite `r_squared`
heuristic. It is not a final reaction-order determination.

## Error Handling

JSON mode does not change errors in v0.2.

Preferred behavior:

- argument errors remain concise stderr messages with usage when appropriate;
- file read errors remain concise stderr messages;
- CSV parse errors remain concise stderr messages;
- kinetics validation, fitting, comparison, or review errors remain concise
  stderr messages;
- stdout remains empty on failure;
- exit code behavior remains unchanged.

Do not design or implement a JSON error schema in v0.2. A future JSON error
contract can be designed only after success output stabilizes.

## Exit Codes

Keep current exit-code categories:

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | User/input error |
| `2` | Internal error or unexpected invariant failure |

Adding `--json` should not introduce new exit codes.

## Backward Compatibility

The v0.2 JSON mode must preserve:

- `version`;
- `doctor`;
- `kinetics analyze` text mode;
- `kinetics analyze` missing-argument behavior;
- existing failure-path behavior;
- current plain text success output unless a very small wording adjustment is
  needed for consistency.

Existing tests that assert text-mode fields should continue to pass.

## Disk Safety

The structured-output mode keeps the v0.1 disk profile:

- reads one explicit user-provided input file;
- writes no output files;
- creates no storage records;
- creates no artifact files;
- writes no logs;
- creates no caches;
- creates no temp directories;
- generates no reports;
- sends JSON only to stdout on success.

The implementation must not canonicalize the input path for output. If the user
provides an absolute path, it may be echoed as provided; the CLI must not derive
or expand local paths itself.

## Dependency Policy

Default: no new external dependencies.

Current workspace dependency state:

- `serde` is already a workspace dependency used by several crates.
- `serde_json` is already a workspace dependency used by existing crates.
- `deepseek-science-cli` does not currently depend directly on `serde` or
  `serde_json`.

Future implementation may add `serde` / `serde_json` to the CLI crate only as
existing workspace dependencies if that keeps serialization safer and smaller
than manual JSON string construction. This is not a new external dependency, but
it is still a Cargo change and should be justified in the implementation phase.

Do not add:

- `clap`;
- async runtime;
- `reqwest`;
- database or storage dependencies;
- broad serialization frameworks beyond existing `serde` / `serde_json`;
- UI or TypeScript dependencies.

## Testing Plan

Future implementation should add focused tests:

- CLI unit test: parser accepts `--json`.
- CLI unit test: parser still rejects duplicate and unknown arguments.
- JSON formatter/unit test if formatting is separated.
- Process-level success smoke test for `--json`.
- Parse JSON stdout in tests if `serde_json` is available in CLI dev/test
  context.
- Assert `schema_version == "kinetics.analysis.v1"`.
- Assert `command == "kinetics.analyze"`.
- Assert selected columns match the CLI arguments.
- Assert valid/rejected counts.
- Assert `fits.first_order` and `fits.second_order` objects exist.
- Assert comparison basis is `finite_r_squared_mvp_heuristic`.
- Assert preferred model is deterministic.
- Assert review status is present.
- Assert stdout has no forbidden scientific wording.
- Assert JSON has no timestamp, random ID, storage path, temp path, or
  model-prose fields.
- Assert stderr is empty on successful JSON runs.
- Assert failure paths still write stderr, keep stdout empty, and exit non-zero.

Avoid full snapshot blobs. Prefer parsing JSON and asserting stable fields.

## Implementation Milestones

1. Extend the `kinetics analyze` argument parser with a boolean `--json` flag.
2. Preserve the existing text mode as the default.
3. Add a small output-mode enum or equivalent local representation if needed.
4. Add a deterministic JSON response struct or local formatter for the success
   path.
5. Map existing `KineticsAnalysisResult` fields into the schema.
6. Add finite-value checks at the serialization boundary.
7. Add unit tests for argument parsing and output formatting.
8. Add one process-level `--json` success smoke test.
9. Re-run existing text-mode and failure-path CLI tests.

Keep all implementation inside the CLI crate unless a small reusable label
function is already public in chemistry. Do not move CLI formatting into generic
crates.

## Deferred Work

- JSON error schema.
- `--output <path>` or any output-file flag.
- Artifact persistence.
- Storage/project workspace records.
- DeepSeek/model explanations.
- Tool execution.
- Plotting.
- Full CSV dialect support.
- Automatic column detection.
- Units system.
- UI.
- Notebook, Jupyter, R, PubMed, or HPC integrations.

## Open Questions

- Should `input.path` always be included, or should a later privacy mode omit it?
- Should review finding `message` use current static messages or new stable
  machine labels?
- Should `valid_point_count` stay inside each fit, or is top-level
  `counts.valid_points` enough for v0.2?
- Should the implementation use existing workspace `serde_json` in CLI, or use a
  tiny hand-written formatter with explicit escaping? The safer default is
  `serde_json` if added as an existing workspace dependency.
