# Phase 2.20 v0.1 CLI Audit

## Summary

The current repository is ready to consider a `v0.1.0` CLI MVP tag for:

```sh
deepseek-science kinetics analyze \
  --input <path> \
  --time-column <column> \
  --concentration-column <column>
```

The implemented path is deterministic, plain-text, file-output-free, and scoped
to one user-provided CSV file. No blocking issues were found.

## Commands Run

- `git status --short`
- `git branch --show-current`
- `git log --oneline --decorate -n 8`
- `cargo fmt --check`
- `cargo check --workspace`
- `cargo test --workspace --lib`
- `cargo test -p deepseek-science-cli --test kinetics_analyze_smoke`
- `cargo run -p deepseek-science-cli -- version`
- `cargo run -p deepseek-science-cli -- doctor`
- `cargo run -p deepseek-science-cli -- kinetics analyze --input crates/deepseek-science-cli/tests/fixtures/kinetics_success.csv --time-column time_s --concentration-column concentration_mol_l`
- `cargo tree --workspace`
- `git diff --check`

## Git Status

- Current branch: `main`.
- Local `HEAD` and `origin/main` both point at `c0edfd7` in the inspected log.
- Initial `git status --short` was clean.
- No pre-existing untracked `docs/reports` files were observed before creating
  this audit report.
- Cargo output is configured outside the source tree at
  `../.cache/deepseek-science-target`.

## README Accuracy

README accurately documents the implemented v0.1 CLI MVP:

- command shape for `deepseek-science kinetics analyze`,
- tiny CSV shape with one header row and numeric rows,
- explicit `--time-column` and `--concentration-column` selection,
- narrow CSV limitations,
- deterministic first-order and second-order linearized fits,
- MVP finite `r_squared` heuristic,
- plain text output,
- no output files or storage records.

README does not claim implemented support for DeepSeek/model calls, tool
execution, UI, artifact persistence, project workspace storage, plotting, JSON
output, full CSV support, Jupyter/R/PubMed/HPC integrations, or Claude Science
parity.

## CLI Behavior

- `version` exits successfully and prints `deepseek-science 0.1.0`.
- `doctor` exits successfully and reports `status: ok`.
- `kinetics analyze` succeeds with the tiny project fixture.
- Success output includes:
  - `first_order.k`,
  - `first_order.r_squared`,
  - `second_order.k`,
  - `second_order.r_squared`,
  - `preferred_model`,
  - `comparison_basis: finite_r_squared_mvp_heuristic`,
  - `Preferred by MVP r_squared heuristic`,
  - `review_status`.
- Success output uses cautious wording:
  `not final scientific model selection`.
- Success output does not contain definitive scientific wording such as
  `definitive`, `true model`, `proved`, or `proof`.

## Test Status

- `cargo fmt --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace --lib`: passed.
- `cargo test -p deepseek-science-cli --test kinetics_analyze_smoke`: passed
  with 5 process-level tests.

The CLI smoke tests cover:

- success path,
- missing file,
- invalid CSV,
- missing time column,
- missing concentration column.

The smoke tests use tiny committed fixtures, do not use temp directories, and do
not write output files.

## Dependency Status

`cargo tree --workspace` shows the current dependency set is limited to existing
Rust workspace crates plus small existing libraries such as `serde`,
`serde_json`, `thiserror`, `uuid`, and `blake3`.

No forbidden v0.1 dependency was observed:

- no TypeScript/Node/Bun/npm dependency,
- no UI framework,
- no `reqwest`,
- no `tokio`,
- no `sqlx`,
- no `rusqlite`,
- no `clap`.

No unexpected generic-crate dependency on chemistry was observed in the
workspace tree. Chemistry depends on generic crates; generic crates do not
depend on chemistry.

## Crate Boundary Status

- `deepseek-science-cli` owns the file read boundary with
  `std::fs::read_to_string`.
- `deepseek-science-common` owns the narrow generic `&str -> DataTable` CSV
  parser and contains no chemistry-specific CSV behavior.
- `deepseek-science-chemistry` owns kinetics validation, linearized fitting,
  comparison, deterministic review, and in-memory artifact proposal logic.
- `deepseek-science-core` remains domain-neutral.
- `deepseek-science-artifacts` remains generic and does not depend on
  chemistry.
- Storage persistence remains absent from the CLI MVP path.

## Disk Safety Status

- The CLI MVP reads exactly one user-provided input file.
- The CLI MVP writes no output files.
- No artifact files, storage records, logs, caches, temp directories, or project
  workspace files are created by the analyzed command path.
- Cargo build/test output goes to the configured external target directory.
- No cleanup script, `cargo clean`, doc build, release build, watcher, coverage,
  profile, or benchmark command was run.
- `git diff --check` passed before this report was created.

## v0.1 Readiness

Recommended: yes, the current repository is ready to consider a `v0.1.0` CLI MVP
tag after this audit report is reviewed and committed if desired.

The recommendation is scoped only to the current CLI MVP. It does not imply
model integration, tool execution, storage persistence, plotting, UI, or full
CSV support.

## Blocking Issues

None.

## Non-blocking Follow-ups

- Consider adding a tag checklist or release note before tagging.
- Consider one future CLI smoke for rejected-row warning output.
- Consider future JSON output only after a stable schema is designed.

## Recommended Next Step

Review and commit this audit report, then tag `v0.1.0` if maintainers accept the
scope and current validation results.
