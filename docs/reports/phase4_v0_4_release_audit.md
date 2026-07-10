# Phase 4.6b v0.4 Release Audit

## Summary

The committed Phase 4 release candidate at
`fc65362420613d6b18b2cea827dbedb1ff1f0ef6` is functionally ready and ready
for an annotated `v0.4.0` tag after this audit report is reviewed and committed.

The audit found no functional or release-blocking issue. `HEAD`, `main`, and
`origin/main` were aligned at the expected commit before the report was
created. The CLI package and all user-visible version paths report `0.4.0`.
The local and remote `v0.4.0` tags were both absent.

The inspected implementation remains deliberately narrow: inspection is
bounded and read-only; conversion is explicit, syntax-only, bounded, and
no-overwrite; normalized output is deterministic; and kinetics analysis still
requires exact user-selected columns. This conclusion does not claim general
instrument-format support or scientific suitability.

## Commands Run

Repository and tag checks:

- `git status --short`
- `git branch --show-current`
- `git remote -v`
- `git log --oneline --decorate -n 15`
- `git rev-parse HEAD`
- `git rev-parse main`
- `git rev-parse origin/main`
- `git ls-files` for each required Phase 4 RFC and audit report
- `git tag --list "v0.4.0"`
- `git ls-remote --tags origin v0.4.0`

Validation and dependency checks, each run once:

- `cargo fmt --check`
- `cargo check -p deepseek-science-common`
- `cargo test -p deepseek-science-common --lib`
- `cargo check -p deepseek-science-cli`
- `cargo test -p deepseek-science-cli --lib`
- `cargo test -p deepseek-science-cli --test data_inspect_smoke`
- `cargo test -p deepseek-science-cli --test data_convert_smoke`
- `cargo test -p deepseek-science-cli --test kinetics_analyze_smoke`
- `cargo test -p deepseek-science-storage --test atomic_create_new`
- `cargo check --workspace`
- `cargo test --workspace --lib`
- `cargo tree --workspace`
- `cargo tree -p deepseek-science-cli`
- `cargo run -p deepseek-science-cli -- version`
- `cargo run -p deepseek-science-cli -- doctor`
- `cargo run -p deepseek-science-cli -- data inspect --help`
- `cargo run -p deepseek-science-cli -- data convert --help`

Scoped source searches were also run for filesystem writes outside storage,
system temporary-directory use, recursive cleanup, overwrite or replacement
fallbacks, duplicate encoding or delimiter logic, chemistry-keyword branches
in common normalization, and model, network, UI, JavaScript, or TypeScript
behavior. The manual release smoke used `cargo run` only for the specific
inspect, convert, and kinetics commands described below.

No script, release build, documentation build, dependency installation,
coverage, profiling, benchmark, watch process, or broad cleanup command was
run.

## Git and Tag Status

- Current branch before report creation: `main`.
- `HEAD`: `fc65362420613d6b18b2cea827dbedb1ff1f0ef6`.
- `main`: `fc65362420613d6b18b2cea827dbedb1ff1f0ef6`.
- `origin/main`: `fc65362420613d6b18b2cea827dbedb1ff1f0ef6`.
- The worktree was clean before this report was created.
- The history contains PR #36 and the CLI `0.4.0` version commit.
- `git tag --list "v0.4.0"` returned no local tag.
- `git ls-remote --tags origin v0.4.0` returned no remote tag.
- No tag was created, changed, or deleted during this audit.

## Documentation History Status

All required Phase 4 design and audit documents are tracked and committed:

- `docs/reports/phase4_real_lab_data_import_rfc.md`
- `docs/reports/phase4_data_inspect_user_path_audit.md`
- `docs/reports/phase4_explicit_data_conversion_rfc.md`
- `docs/reports/phase4_end_to_end_data_path_audit.md`

The Phase 3 persistence RFC and v0.3 persistence audit are also present and
remain the applicable atomic-output safety foundation. No existing RFC or
audit report was modified.

## Version Status

- `deepseek-science-cli` package version: `0.4.0`.
- `Cargo.lock` records `deepseek-science-cli` as `0.4.0`.
- `deepseek-science version` printed `deepseek-science 0.4.0`.
- `deepseek-science doctor` printed `version: 0.4.0`.
- `deepseek-science doctor` printed `prompt_kernel_version: 0.4.0`.
- `cargo tree -p deepseek-science-cli` identified the CLI package as `0.4.0`.
- The commands use the CLI package version as the controlling source; no stale
  user-visible `0.3.0` appeared in the live version or doctor output.
- Independently versioned internal crates remain at their existing versions;
  no workspace-wide version bump occurred.

## CLI Surface Status

The active command surface includes:

- `version`;
- `doctor`;
- `data inspect --input <path>`;
- `data convert --input <path> --output <path>`;
- `kinetics analyze` with exact time and concentration column arguments;
- the existing kinetics `--json` mode;
- the existing kinetics explicit `--output` path.

Conversion exposes no force, overwrite, in-place, metadata-removal,
unit-row-removal, header-selection, delimiter-selection, encoding-selection,
column-selection, JSON, batch, recursive, watch, or background option.
Conversion does not invoke kinetics. Inspection and conversion do not invoke a
model, network path, tool execution path, or scientific column inference.

## Data Inspect Status

`data inspect`:

- opens one explicit path and requires the opened object to be a regular file;
- checks metadata size and then reads at most
  `MAX_INSPECTION_BYTES + 1` bytes;
- supports strict UTF-8, UTF-8 BOM, UTF-16LE BOM, and UTF-16BE BOM input;
- does not guess BOM-free UTF-16 and does not decode lossily;
- considers only comma and tab delimiters;
- reports `NumericNarrowTable`, `NumericMatrix`, `MixedOrUnsupported`, or
  `Empty` through stable user-facing values;
- returns a successful deterministic report for completed structural
  inspection, including unsupported or incompatible structure;
- returns nonzero with empty success stdout for fatal argument, file, limit,
  binary, or encoding failures;
- writes no file and creates no hidden state;
- never runs chemistry or chooses scientific columns.

The process tests cover supported encodings, narrow tables, matrices, empty and
quoted input, delimiter ambiguity, encoding failures, missing and directory
inputs, deterministic repeated output, and no-side-effect behavior.

## Data Convert Status

`data convert` accepts only an unambiguous named finite numeric narrow table
that requires a real normalization step:

- removal of one UTF-8 BOM;
- strict transcoding from BOM-marked UTF-16LE or UTF-16BE;
- tab-to-comma delimiter normalization;
- or a supported combination of those transformations.

It refuses already-compatible UTF-8/no-BOM/comma input, matrices, metadata,
unit rows, blank rows, headerless data, duplicate or empty headers, quotes or
multiline requirements, leading or trailing Unicode cell whitespace, unsafe
controls, inconsistent widths, empty body cells, non-finite body values, and
ambiguous delimiters or table regions.

The normalizer preserves safe decoded cell text and column order. Refusal does
not remove rows, trim cells, rewrite numeric values, select columns, or repair
input. `AlreadyCompatible` is decided before output planning, so the output
path is not touched for a no-op request.

## Normalized Byte Contract

Every successful conversion produces one deterministic representation:

- UTF-8 encoding;
- no BOM;
- ASCII comma delimiter;
- LF record endings;
- exactly one trailing LF;
- no blank record;
- no quoting or escaping;
- one named header followed by a finite numeric body.

Safe header text, column order, and numeric lexical text are copied from the
decoded source rather than serialized from parsed `f64` values. The common
tests cover forms including explicit signs, leading zeros, exponent notation,
and negative zero.

After normalization succeeds and before output planning, the CLI passes the
normalized text to `parse_simple_numeric_csv` as a syntax postcondition. The
returned `DataTable` is discarded. It is not used to select columns or drive
chemistry, and a parser-postcondition failure prevents publication.

## Resource Limit Status

- `MAX_INSPECTION_BYTES` is exactly `16 * 1024 * 1024` bytes.
- `MAX_NORMALIZED_OUTPUT_BYTES` is exactly `24 * 1024 * 1024` bytes.
- Neither limit is configurable.
- Input reading uses the fixed limit plus one observed byte to detect growth
  after the metadata check.
- Normalized output projection uses checked arithmetic.
- The actual normalized byte length is checked again before publication.
- Limit branches use private small-limit test seams rather than large test
  allocations.
- No streaming converter, memory mapping, spool file, cache, async runtime, or
  background worker was introduced.

## Atomic Publication Status

Conversion publication uses the existing storage boundary only:

- `AtomicWriteRequest`;
- `WriteMode::CreateNew`;
- `AtomicWritePlan::execute`.

The conversion path contains no direct `std::fs::write`, second writer,
check-then-write `Path::exists()` authority, replacement fallback, pre-delete,
or overwrite mode. The output parent must already exist and is never created.
Existing targets retain their bytes, including the sentinel cases covered by
the CLI and storage tests.

All input, eligibility, normalization, parser-postcondition, and size checks
finish before output planning. Success stdout is emitted only after
`AtomicWritePlan::execute` succeeds. Publication error mapping hides the
operation-owned temporary sibling and conservatively warns that the requested
target may exist and should be inspected before retrying. The CLI does not
delete a possibly published final target as rollback.

## End-to-End Release Smoke

The supported smoke used the exact audit-owned directory:

`/Users/taomu/开发/.cache/deepseek-science-target/tmp/phase4-6b-v0-4-release-audit`

A 204-byte UTF-16LE BOM, tab-delimited narrow table used the explicit headers
`time_s`, `concentration_mol_l`, and `temperature_c`, with four finite numeric
rows. The sequence produced these results:

1. Original inspection reported `utf-16le`, `utf-16le` BOM, `tab`,
   `numeric-narrow-table`, and `requires-explicit-normalization`.
2. Conversion succeeded and reported a 101-byte output.
3. A byte-for-byte comparison confirmed the exact expected UTF-8/no-BOM,
   comma-delimited, LF-terminated output with one trailing LF.
4. The output preserved lexical values including `-0`, `+01.00e+0`, `5E-1`,
   `0.2500`, and `+1.25e-1`.
5. Reinspection reported UTF-8, no BOM, comma, `numeric-narrow-table`, and
   `compatible-as-is`.
6. `kinetics analyze` succeeded only after the exact `time_s` and
   `concentration_mol_l` columns were supplied explicitly.
7. SHA-256 checks confirmed that conversion did not modify the source and
   kinetics analysis did not modify the normalized file.

No kinetics `--output` path was used.

## Refusal-path Status

A 47-byte UTF-8 tab-delimited matrix with the neutral headers `axis`,
`series 1`, and `series 2` was inspected and reported as `numeric-matrix` with
current kinetics incompatibility. Conversion returned exit status 1 with
empty stdout and a concise structural-ineligibility error. No requested matrix
output or atomic temporary sibling appeared, and the matrix input hash was
unchanged.

The process and common tests additionally cover refusal of already-compatible
input, metadata, unit rows, blank rows, headerless data, duplicate and empty
headers, quotes, whitespace-bearing cells, controls, inconsistent widths,
non-finite values, and ambiguous structure. These cases are refused without
repair or output publication.

## Existing CLI Compatibility

The existing kinetics process suite passed for text output, JSON output,
explicit JSON `--output`, existing-target refusal, missing-parent refusal, and
analysis-failure no-output behavior. The JSON schema remains
`kinetics.analysis.v1`.

The existing inspect suite passed unchanged. The storage CreateNew suite passed
its no-overwrite and concurrency checks. `version` and `doctor` both continued
to work with the aligned `0.4.0` package version.

## Determinism Status

- Repeated inspection is covered as byte-identical by process tests.
- Repeated conversion to distinct fresh targets is covered as byte-identical
  by process and common tests.
- The manual output matched one exact byte sequence.
- Inspect and convert reports use fixed field order and exactly one trailing
  newline.
- Successful output includes no timestamp, random identifier, temporary path,
  derived absolute output path, model prose, terminal-width formatting, ANSI
  color, or progress output.

## Untrusted Input Safety

- Decoding is strict and non-lossy.
- UTF-8 and UTF-16 decoding errors retain original-input byte offsets where
  applicable.
- Binary, NUL, invalid encoding, unsupported BOM, and ambiguous BOM-free bytes
  are rejected without dumping raw bytes or decoded body content.
- Inspect escapes and bounds untrusted displayed header controls so they cannot
  inject report fields.
- Conversion success output does not display source headers or body cells.
- Unsafe controls, quoted structure, embedded record controls, and implicit
  whitespace repair are rejected.
- Paths are not canonicalized into a claimed containment guarantee.
- Production inspect and convert paths execute no shell, subprocess, model, or
  network operation.

## Disk Safety Status

Production `data inspect` reads one explicit file and writes nothing. It creates
no directory, temporary file, cache, log, project, run, artifact, workspace, or
database state.

Production `data convert` reads one explicit file and may create one bounded,
same-directory, operation-owned temporary sibling before publishing one
explicit final target. It does not modify the input, overwrite a target, create
a parent, scan siblings, perform broad cleanup, or create project, artifact,
run, cache, log, or database state. It starts no background task.

Tests use tiny exact paths beneath `CARGO_TARGET_TMPDIR`, inspect their owned
directories, and use exact cleanup. Scoped searches found no
`std::env::temp_dir` or `remove_dir_all` in the audited tests.

For the manual smoke, the audit directory contained exactly the UTF-16LE input,
the normalized output, and the matrix input before cleanup. No unexpected file
or temporary sibling existed. Each audit-created file and then the exact empty
directory were removed individually; the audit directory no longer exists.
The repository itself received no generated file other than this report.

## Crate Boundary Status

- Encoding, delimiter and table-shape inspection, simple CSV parsing, and
  normalization remain pure in `deepseek-science-common`.
- Common performs no filesystem, environment, storage, model, tool, or
  chemistry operation.
- The CLI owns argument parsing, bounded file reading, orchestration, report
  formatting, and calls into storage publication.
- Storage owns opaque-byte atomic publication only.
- Chemistry is invoked only by the explicit kinetics command; inspect and
  convert do not call it.
- No common-to-chemistry dependency exists.
- The CLI does not duplicate the common encoding or delimiter algorithms.
- No new Phase 4 crate was introduced.

## Dependency Status

`cargo tree --workspace` and `cargo tree -p deepseek-science-cli` confirmed the
existing small dependency graph. Phase 4 introduced no `clap`, broad CSV
framework, encoding detector, async runtime, temporary-file crate, database,
logger, model or network client, UI framework, or JavaScript/TypeScript package
path.

The Phase 4 Cargo history contains no dependency addition or unrelated lockfile
churn. The only Cargo metadata change after the feature work is the intended
CLI package version alignment from `0.3.0` to `0.4.0` in the CLI manifest and
matching lockfile package entry.

## README and Help Accuracy

README and the live command help accurately describe:

- `data inspect` and `data convert`;
- the 16 MiB input limit and 24 MiB normalized-output limit;
- supported UTF-8 and BOM-marked UTF-16 encodings;
- comma and tab inspection scope;
- deterministic UTF-8/no-BOM/comma/LF output with one trailing LF;
- no overwrite and the existing-parent requirement;
- `AlreadyCompatible` refusal;
- matrix, metadata, unit-row, quote, blank-row, and whitespace-refusal scope;
- explicit kinetics column selection;
- no conversion JSON, automatic analysis, or model call.

They do not claim full CSV support, matrix conversion, proprietary instrument
support, metadata repair, unit conversion, scientific suitability, automatic
chemistry interpretation, UI support, or DeepSeek API execution.

## Test Status

All requested validation passed:

- formatting check: passed;
- common check: passed;
- common library tests: 129 passed;
- CLI check: passed;
- CLI library tests: 39 passed;
- data inspect process tests: 17 passed;
- data convert process tests: 18 passed;
- kinetics process tests: 12 passed;
- atomic CreateNew integration tests: 6 passed;
- workspace check: passed;
- workspace library tests: 370 passed in total;
- both dependency-tree inspections: passed;
- live version, doctor, inspect help, and convert help checks: passed;
- supported end-to-end release smoke: passed;
- matrix refusal smoke: passed.

No requested validation was skipped, and no validation command was rerun after
failure. No failure occurred.

## Known Risks

The following risks are real but non-blocking and match the accepted Phase 4
contracts:

- Inputs are not locked; concurrent modification can cause a read failure or a
  report for the bounded bytes actually observed.
- Symlinks follow ordinary platform file-opening behavior; no cross-platform
  symlink containment policy is claimed.
- Atomic publication depends on filesystem hard-link support and has no unsafe
  fallback.
- A cleanup failure after successful publication can leave the requested final
  target present; the conservative error message tells the user to inspect the
  target before retrying.
- Matrix classification is deliberately conservative and syntax-driven.
- Inspection and conversion prove only supported syntax compatibility, not
  scientific correctness or suitability.
- No private laboratory data should be committed as a fixture.

## Functional Readiness

**Yes.** The bounded inspect, explicit conversion, deterministic normalized
output, atomic no-overwrite publication, and explicit-column kinetics path all
pass their unit, process, workspace, and manual release checks.

## Release/Tag Readiness

**Yes.** The audited committed candidate is ready for an annotated `v0.4.0`
tag after this audit report is reviewed and committed. The version is aligned,
the required documentation history is present, validation passed, and no local
or remote `v0.4.0` tag currently exists.

This conclusion authorizes no tag operation in this audit task and does not
claim GitHub Release readiness or creation.

## Blocking Issues

None.

## Non-blocking Follow-ups

- Preserve the narrow Phase 4 format boundary in any later format proposal.
- Keep the documented concurrency, symlink, hard-link, and syntax-versus-
  science limitations visible in future release notes.
- Do not add real or private laboratory exports as repository fixtures.

## Recommended Next Step

Review and commit this audit report. Then, in a separate task, verify the final
clean aligned `main` state and create and push the annotated `v0.4.0` tag.
Do not create a GitHub Release unless it is separately requested.
