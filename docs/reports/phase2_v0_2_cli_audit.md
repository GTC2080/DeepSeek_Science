# Phase 2.25 v0.2 CLI Audit

## Summary

The v0.2 CLI scope is ready for release: text output remains the default,
`--json` emits one deterministic JSON object on success, errors remain
human-readable stderr messages, and `kinetics analyze --help` works.
Validation, dependency, crate-boundary, and disk-safety checks passed.

The previous version blocker is resolved. The CLI package, `version` command,
and `doctor` command now report `0.2.0`. Tagging `v0.2.0` is recommended after
this refreshed audit report is reviewed and committed.

## Commands Run

- `git status --short`
- `git branch --show-current`
- `git log --oneline --decorate -n 10`
- `git tag --list "v0.2.0"`
- `git diff --check`
- Read-only `cat`, `sed`, and `rg` inspections of the requested docs, source
  files, manifest, tests, and prior audit findings
- `cargo fmt --check`
- `cargo check --workspace`
- `cargo test --workspace --lib`
- `cargo test -p deepseek-science-cli --test kinetics_analyze_smoke`
- `cargo run -p deepseek-science-cli -- version`
- `cargo run -p deepseek-science-cli -- doctor`
- `cargo run -p deepseek-science-cli -- kinetics analyze --help`
- `cargo run -p deepseek-science-cli -- kinetics analyze --input crates/deepseek-science-cli/tests/fixtures/kinetics_success.csv --time-column time_s --concentration-column concentration_mol_l`
- `cargo run -p deepseek-science-cli -- kinetics analyze --input crates/deepseek-science-cli/tests/fixtures/kinetics_success.csv --time-column time_s --concentration-column concentration_mol_l --json`
- `cargo tree --workspace`

Each Cargo command was run once. No Cargo command was rerun after a failure.

## Git Status

- Current branch: `main`.
- Inspected `HEAD` and `origin/main` both pointed to `a7c1ef9`.
- `v0.2.0` was not present in the local tag list.
- The only initial worktree entry was the expected untracked
  `docs/reports/phase2_v0_2_cli_audit.md`, which was updated in place.
- No unexpected source, Cargo, generated-output, or documentation changes
  appeared during validation.

## README Accuracy

README accurately documents:

- text output as the default;
- `--json` success output;
- stdout/stderr behavior and the absence of a JSON error schema;
- schema version `kinetics.analysis.v1` and command `kinetics.analyze`;
- the `input`, `columns`, `counts`, `fits`, `comparison`, and `review` fields;
- narrow CSV support, explicit column selection, scientific caution, and disk
  safety;
- current limitations including no model calls, model-generated explanations,
  tool execution, UI, TypeScript, persistence, plotting, output-file flag,
  project storage, full CSV dialect support, or notebook/Jupyter/R/PubMed/HPC
  integrations.

No Claude Science parity claim was found.

## CLI Behavior

- `version` exited successfully and printed `deepseek-science 0.2.0`.
- `doctor` exited successfully with `status: ok`, version `0.2.0`, and prompt
  kernel version `0.2.0`.
- `kinetics analyze --help` exited successfully, documented all required
  arguments and `--json`, and stated that text is the default and errors remain
  on stderr.
- The text-mode fixture run succeeded and printed both fits, the finite
  `r_squared` MVP comparison basis, cautious preference wording, and review
  status.
- No definitive scientific wording was observed in the text success output.

## JSON Output Status

The JSON fixture run succeeded. The process-level smoke test parsed stdout with
`serde_json` and confirmed empty stderr on success.

Observed JSON properties:

- one JSON object with `schema_version`, `command`, `input`, `columns`,
  `counts`, `fits`, `comparison`, and `review`;
- `schema_version` is `kinetics.analysis.v1`;
- `command` is `kinetics.analyze`;
- comparison basis is `finite_r_squared_mvp_heuristic`;
- input path is echoed as provided rather than canonicalized;
- no timestamp, random identifier, storage path, temp path, or model-generated
  prose field;
- no `NaN` or infinity;
- no definitive scientific wording.

The CLI defensively rejects non-finite fit values before JSON serialization.
The schema is assembled from fixed fields and deterministic analysis values.

## Help / Error UX Status

- `kinetics analyze --help` returns success and writes usage to stdout.
- Help includes `--input`, `--time-column`, `--concentration-column`, `--json`,
  `--help`, and `-h`.
- Missing and unknown arguments remain concise user errors with usage.
- Duplicate `--json` is rejected deterministically.
- JSON mode does not introduce a JSON error schema or change failure output.

## Version Status

- `deepseek-science-cli` package version is `0.2.0`.
- `deepseek-science version` reports `deepseek-science 0.2.0`.
- `deepseek-science doctor` reports version `0.2.0`.
- `doctor` also reports prompt kernel version `0.2.0` because it uses the same
  package version source.
- `cargo tree --workspace` identifies `deepseek-science-cli v0.2.0`.
- No stale user-visible `0.1.0` CLI version was observed.

## Test Status

- `cargo fmt --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace --lib`: passed.
- `cargo test -p deepseek-science-cli --test kinetics_analyze_smoke`: passed
  with 7 process-level tests.

The process tests cover text success, JSON success, help, missing file, invalid
CSV, missing time column, and missing concentration column. They use four tiny
committed fixtures totaling 152 bytes, use no temp directories, and contain no
file-write path.

Non-blocking coverage gap: the JSON smoke test verifies parsed fields and
cautious wording but does not compare two separate process runs byte-for-byte
or explicitly assert every forbidden metadata key. The fixed schema and manual
output inspection provide current confidence, but a later focused assertion
could make the determinism contract more direct.

## Dependency Status

`cargo tree --workspace` showed only the existing Rust workspace crates and the
existing small dependency set. The CLI is reported as version `0.2.0` and uses
the existing workspace `serde_json` dependency edge.

No `reqwest`, `tokio`, `sqlx`, `rusqlite`, `clap`, UI framework, or unexpected
heavy dependency was found. No TypeScript, JavaScript, Node, Bun, npm, pnpm, or
yarn project path was found.

Generic crates do not depend on `deepseek-science-chemistry`; the dependency
direction remains from chemistry and CLI toward generic crates.

## Crate Boundary Status

- `deepseek-science-cli` owns the sole analyze file read and output formatting.
- `deepseek-science-common` parses `&str` into `DataTable` and remains
  chemistry-neutral and file-IO-free.
- `deepseek-science-chemistry` owns validation, fitting, comparison, review,
  and analysis contracts.
- `deepseek-science-core` and `deepseek-science-artifacts` do not depend on
  chemistry.
- The kinetics analyze path performs no model call, tool execution, artifact
  persistence, workflow execution, or storage write.

## Disk Safety Status

- Analyze reads one explicitly supplied input file; help reads none.
- Text and JSON results are written only to stdout; errors use stderr.
- No output file, artifact file, storage record, log, cache, temp directory, or
  generated report is created by the CLI path.
- Cargo output remained in the configured external target directory:
  `../.cache/deepseek-science-target`.
- No cleanup script, `cargo clean`, doc build, release build, watcher,
  coverage, profiling, benchmark, or deletion command was run.
- The only repository file modified by this task is this existing untracked
  audit report.

## v0.2 Readiness

Functional CLI readiness: **yes**. The scoped v0.2 text, JSON, help, error,
dependency, boundary, and disk-safety behavior is ready.

Release/tag readiness: **yes**. The CLI version is aligned to `0.2.0`, the tag
does not already exist, and the bounded release validation passed. Creating the
`v0.2.0` tag is recommended after this report is reviewed and committed.

## Blocking Issues

None.

## Non-blocking Follow-ups

- Add a focused repeated-run or forbidden-key JSON assertion if stronger
  executable determinism evidence is desired.
- Revisit real laboratory UTF-16, TSV, or instrument-export adapters in a later
  phase without expanding the v0.2 release scope.

## Recommended Next Step

Review and commit this refreshed audit report. Then, in a separate release task,
reconfirm that `main` is aligned with `origin/main`, the worktree is clean, and
`v0.2.0` is still absent before creating and pushing the annotated tag.
