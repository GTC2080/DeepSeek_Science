# Phase 4.6a End-to-End Laboratory Data Path Audit

## Summary

Phase 4 now provides one coherent deliberately narrow user path: inspect one
supported laboratory text export, explicitly normalize an eligible source to
the current simple CSV boundary, inspect the normalized bytes, and run the
existing kinetics command with exact user-selected columns. The primary
UTF-16LE BOM/tab-to-UTF-8 CSV-to-kinetics path completed successfully.

Unsupported structures remain visible and are not repaired. Neutral-label
numeric matrices, metadata preambles, separate unit rows, already-compatible
inputs, and unsafe output paths were refused without modifying inputs,
overwriting targets, or leaving operation-owned temporary files.

Functional readiness is **yes**, and the CLI is ready for a separate version
alignment to `0.4.0`. Release and tag readiness are **no**: the CLI remains
`0.3.0`, this report is not yet committed, and a separate final v0.4 release
audit has not been performed.

## Commands Run

Initial Git and documentation checks:

- `git status --short`
- `git branch --show-current`
- `git remote -v`
- `git log --oneline --decorate -n 12`
- `git ls-files docs/reports/phase4_data_inspect_user_path_audit.md`
- `git ls-files docs/reports/phase4_explicit_data_conversion_rfc.md`

Required validation, with each Cargo command run once:

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

Manual CLI checks:

- `cargo run -p deepseek-science-cli -- data inspect --help`
- `cargo run -p deepseek-science-cli -- data convert --help`
- `cargo run -p deepseek-science-cli -- version`
- `cargo run -p deepseek-science-cli -- doctor`
- inspect, convert, re-inspect, and explicit-column kinetics analysis for one
  tiny UTF-16LE BOM tab table;
- inspect and conversion refusal for one neutral-label tab matrix;
- inspect and conversion refusal for metadata and unit-row inputs;
- conversion refusal for an already-compatible input and an existing target.

Read-only evidence commands included scoped `rg` searches, `shasum -a 256`,
`xxd`, `cmp -s`, and an exact one-directory listing. One tiny UTF-16LE source
was generated under the external Cargo target area with the system `iconv`.
The first attempted `iconv -o` form was unsupported by the platform and
created no file; the same conversion was then written explicitly through
standard output redirection and its `FF FE` BOM was verified.

After this report was written, only its requested no-index diff and final
short Git status were run. Cargo validation was not repeated.

## Git and Documentation Status

Status: pass.

- Initial branch: `main`.
- Initial `HEAD`, `main`, and `origin/main`:
  `1eaaf3f843f96fb72c4afd2bb444224bd89367e5`.
- Initial worktree: clean.
- PR #35 is present through merge commit `1eaaf3f`.
- `docs/reports/phase4_data_inspect_user_path_audit.md` is tracked and
  committed at `97a59c0`.
- `docs/reports/phase4_explicit_data_conversion_rfc.md` is tracked and
  committed at `4ce809c`.
- No branch was created and no file was staged, committed, or pushed.
- The only repository change produced by this task is this audit report.

## CLI Surface Status

Status: pass.

- `data inspect --input <path>` and
  `data convert --input <path> --output <path>` are both routed and documented.
- `data convert` accepts exactly one input, one output, and help.
- Its manual parser rejects missing values, duplicate options, unknown flags,
  and unexpected positional arguments.
- There is no conversion `--json`, `--force`, `--overwrite`, `--in-place`,
  metadata, header, delimiter, encoding, row, or column-selection flag.
- Help states the 16 MiB input limit, 24 MiB output limit, deterministic
  UTF-8/no-BOM/comma/LF format, no-overwrite behavior, existing-parent
  requirement, and deliberate refusal cases.
- `kinetics analyze` remains a separate explicit command with required exact
  time and concentration column arguments.
- Inspection and conversion never automatically start kinetics analysis.

## Encoding and Inspection Status

Status: pass.

- Strict UTF-8 without a BOM, UTF-8 with one BOM, BOM-marked UTF-16LE, and
  BOM-marked UTF-16BE are covered and passed.
- UTF-32, binary/NUL evidence, invalid UTF-8, invalid UTF-16, conflicting or
  repeated BOMs, and BOM-free invalid UTF-8 are rejected deterministically.
- BOM-free invalid UTF-8 is not guessed as UTF-16, and decoding is never
  lossy.
- Invalid UTF-8/UTF-16 errors preserve original-input byte offsets and fatal
  process tests confirm empty success stdout.
- Only comma and tab are inspected; no semicolon, locale, or arbitrary
  delimiter inference exists.
- Structural unsupported findings remain completed successful inspections,
  while fatal byte/encoding failures remain nonzero command errors.
- Narrow-table classification requires named finite rectangular evidence.
  Matrix classification is based on generic sibling-header syntax plus a
  strictly monotonic first numeric column, not chemistry vocabulary.

## Conversion Eligibility Status

Status: pass.

Successful coverage includes:

- UTF-8 BOM plus comma;
- UTF-16LE BOM plus comma;
- UTF-16BE BOM plus comma;
- UTF-8 without BOM plus tab;
- UTF-8 BOM plus tab;
- UTF-16LE and UTF-16BE BOM plus tab.

Conversion requires one complete unambiguous `NumericNarrowTable` beginning
on physical line 1, one unique nonempty named header, at least one finite
rectangular numeric row, no metadata or additional content, and no blank,
quoted, inconsistent, empty, non-finite, or ambiguous evidence.

Structured common tests and CLI process tests refuse already-compatible
UTF-8 comma input, matrices, empty input, metadata, unit rows, headerless
numeric input, duplicate or empty headers, blank rows, inconsistent widths,
empty/non-finite body cells, quotes/multiline requirements, surrounding
whitespace, commas requiring TSV quoting, lone CR, NUL/control/DEL characters,
and ambiguous delimiters or regions. No refusal path trims, removes, repairs,
renames, reorders, or reformats source content.

## Normalized Byte Contract

Status: pass.

Every successful conversion produces one owned string with:

- UTF-8 encoding and no BOM;
- ASCII comma delimiters;
- LF-only record endings;
- exactly one trailing LF;
- no blank rows, quotes, escapes, metadata, or unit row;
- one header followed by finite numeric rows.

The manual normalized output was exactly 101 bytes and byte-identical to the
audit's expected file. Its hexadecimal prefix contained the UTF-8 header
directly, not a BOM, and its final byte was `0A`. The output preserved column
order and exact lexical forms including `-0`, `+01.00e+0`, `1E+2`, `5E-1`,
`0.2500`, and `+1.25e-1`.

Production code parses numeric body cells only to confirm finite eligibility;
it appends the original decoded cell slices to output and never serializes the
parsed `f64` values.

## Parser Postcondition Status

Status: pass.

- The CLI calls `parse_simple_numeric_csv(&normalized)` after pure
  normalization and before any output plan is built or executed.
- The returned `DataTable` is immediately discarded; it is not used for
  column selection, scientific inference, or chemistry.
- A parser failure maps to an internal normalized-output validation error and
  prevents output path planning and publication.
- The pure normalizer does not call the CSV parser or construct `DataTable`.
- The existing simple CSV parser is unchanged.

## Atomic Publication Status

Status: pass.

- Conversion output is published only through `AtomicWritePlan::execute` with
  `WriteMode::CreateNew`.
- No direct production `std::fs::write`, `File::create`, second atomic writer,
  check-then-write target existence authority, replacement mode, or unsafe
  fallback exists in inspect/convert/common/chemistry production code.
- The output parent must already be a directory; no parent is created.
- A deterministic same-directory create-new temporary sibling is fully
  written and synced, then hard-linked to a create-new target.
- Existing targets and stale temporary files are refused without modification.
- Storage integration tests confirm exact bytes, missing-parent refusal,
  sentinel preservation, stale-temp preservation, temporary cleanup, rejected
  replacement mode, and exactly one winner under bounded concurrent writers.
- Success stdout is formatted only after `execute` succeeds.
- Conversion-specific publication errors hide temporary sibling paths. An
  ambiguous failure tells the user that the requested target may exist and
  must be inspected before retrying; the command does not delete a possibly
  published final target.

## Supported End-to-End Path

Status: pass.

One tiny synthetic UTF-16LE BOM, tab-delimited table used the explicit headers
`time_s`, `concentration_mol_l`, and `temperature_c` with four finite numeric
rows.

1. Original inspection reported:
   - encoding `utf-16le`;
   - BOM `utf-16le`;
   - delimiter `tab`;
   - shape `numeric-narrow-table`;
   - `requires-explicit-normalization`;
   - current kinetics workflow `requires-normalization-before-analysis`.
2. Conversion succeeded with three fields and four data rows, producing one
   101-byte target. The input SHA-256 remained
   `d36dfb520fa7221801ff15159a8b9b8502401053f6bf1834098ac57b50d42a0f`.
3. Exact comparison with the expected normalized bytes passed. The normalized
   SHA-256 was
   `ea561b971cc3ade21ec9f396348dd96c6d8d8ed4b6fba173aeea84105932a3c2`.
4. Re-inspection reported UTF-8, no BOM, comma, numeric narrow table,
   `compatible-as-is`, and only
   `potentially-compatible-after-explicit-column-selection`.
5. `kinetics analyze` was invoked with exact `time_s` and
   `concentration_mol_l` arguments. It accepted four points, rejected none,
   completed deterministic first- and second-order fits, and passed review.
6. No analysis `--output` was used. The normalized SHA-256 remained unchanged
   after analysis, and no automatic column inference occurred.

This path establishes format and command compatibility only; it does not
establish scientific suitability.

## Matrix Refusal Path

Status: pass.

A tiny tab table with neutral labels `axis`, `series 1`, and `series 2` was
inspected as `numeric-matrix`, with simple CSV and current kinetics workflow
both reported as incompatible. Conversion returned nonzero with empty success
stdout and `input structure is not eligible for explicit conversion`.

No matrix output or atomic temporary sibling appeared. The matrix input hash
remained
`9d8ab0d1f0b777b72fab91286968e9feaf42cd60eb90b7c6ce341ccee99c4abb`.
No chemistry term was required for matrix classification.

## Metadata and Unit-row Refusal

Status: pass.

- The metadata case was inspected successfully with table region `2-5`,
  `metadata_lines: 1`, and a finding that metadata precedes the table.
  Conversion refused it; no row was removed and no output appeared.
- The unit-row case was inspected successfully with header candidates `1,2`,
  shape `mixed-or-unsupported`, and a finding that multiple header or unit
  rows require an explicit decision. Conversion refused it; no unit row was
  removed and no output appeared.
- Both input hashes matched their pre-command values, and no temporary sibling
  remained.

## Path Safety Status

Status: pass.

- Lexically identical input/output paths are rejected before output access.
- Already-compatible input is rejected after complete structural and raw-cell
  validation and before output planning; the manual refusal created no target.
- Missing parents are not created.
- Directory and missing inputs are rejected as input errors.
- Existing-target process and manual checks preserved sentinel bytes exactly.
- Target existence is authoritative only at atomic `CreateNew` publication;
  production conversion does not call `Path::exists()`.
- Production input and output paths are not canonicalized. Storage layout
  explicitly computes paths without canonicalization or claimed filesystem
  containment.
- Ordinary platform symlink opening behavior remains unchanged.

## Resource Limit Status

Status: pass.

- `MAX_INSPECTION_BYTES` is exactly `16 * 1024 * 1024`.
- `MAX_NORMALIZED_OUTPUT_BYTES` is exactly `24 * 1024 * 1024`.
- Neither limit is configurable through CLI, environment, feature, or public
  policy objects.
- Opened-file metadata is checked before allocation, then the shared bounded
  reader observes at most the input limit plus one byte through `Read::take`.
- The input file is read only once.
- Output projection and appends use checked arithmetic, reject before
  over-limit allocation/publication, and verify the completed byte count.
- Limit branches use private small-limit unit seams. No 16 MiB or 24 MiB
  process fixture was created.
- No streaming converter/publication layer, mmap, spool file, cache, async
  runtime, watcher, or background task exists.

## Determinism Status

Status: pass.

- Repeated inspection process coverage compares stdout bytes directly.
- Repeated conversion to two fresh targets produces byte-identical normalized
  content.
- Pure repeated normalization returns equal structured results.
- Conversion summaries use fixed fields and do not contain output paths,
  random values, timestamps, temporary paths, terminal-width formatting,
  ANSI output, progress text, or model prose.
- Successful inspect, conversion, text analysis, and JSON format coverage
  verifies exactly one trailing newline where applicable.

## Disk Safety Status

Status: pass.

Production `data inspect`:

- opens and reads one explicit regular file;
- writes no file or hidden state;
- creates no temporary directory, cache, log, project, run, or artifact.

Production `data convert`:

- opens and bounded-reads one explicit regular input;
- may create one same-directory operation-owned temporary sibling;
- may publish one explicit final target;
- modifies no input, creates no parent, overwrites nothing, and performs no
  recursive, project, artifact, run, cache, log, database, or background work.

Tests use `CARGO_TARGET_TMPDIR`, tiny PID/counter-owned directories, exact file
cleanup, and exact empty-directory removal. Scoped searches found no
`std::env::temp_dir` or `remove_dir_all` in the audited common, CLI, or storage
tests.

Manual files lived only under:

`/Users/taomu/开发/.cache/deepseek-science-target/tmp/phase4-6a-end-to-end-audit`

Before cleanup, that directory contained exactly nine explicitly created or
expected files: seven source/control files, the generated UTF-16LE source, and
the one successful normalized target. No refused output, unexpected file, or
`.atomic-write.tmp` sibling existed. Those exact nine files and then the exact
empty audit-owned directory were removed. No system temporary directory,
recursive cleanup, wildcard cleanup, or broad deletion was used.

## Existing CLI Compatibility

Status: pass.

- `data inspect` process coverage passed unchanged.
- Existing `kinetics analyze` text, `--json`, and explicit `--output` process
  behavior passed, including deterministic schema, byte-identical JSON
  stdout/output, no-overwrite, missing-parent, and analysis-failure paths.
- JSON schema remains `kinetics.analysis.v1`.
- Storage semantics remain unchanged and passed their dedicated integration
  suite.
- `version` reported `deepseek-science 0.3.0`.
- `doctor` reported version `0.3.0` and `status: ok`.
- No version metadata was modified during this audit.

## Crate Boundary Status

Status: pass.

- Strict encoding, deterministic delimited inspection, simple CSV parsing,
  and pure normalization remain in `deepseek-science-common`.
- The common modules perform no file, environment, model, tool, storage,
  subprocess, or chemistry operation.
- File opening, bounded reading, manual CLI parsing, orchestration, error
  mapping, and success report formatting remain in `deepseek-science-cli`.
- `deepseek-science-storage` remains an opaque-byte create-new publication
  boundary.
- Chemistry is called only by the separately routed kinetics command; inspect
  and convert do not call it.
- Normalization production code contains no chemistry-keyword conditional.
- The CLI reuses `inspect_text_encoding` and `inspect_delimited_text`; scoped
  searches found no second encoding or delimiter implementation.
- There is no common-to-chemistry dependency and no Phase 4 crate addition.

## Dependency Status

Status: pass.

- `cargo tree --workspace` completed successfully.
- A Phase 4 history diff from the accepted Phase 4.0 RFC commit through
  `HEAD` showed no `Cargo.toml` or `Cargo.lock` change.
- No `clap`, broad CSV framework, encoding detector, async runtime,
  temporary-file crate, database, logger, UI framework, JavaScript/TypeScript
  toolchain, or model/network dependency was introduced for this path.
- Phase 4 reuses the Rust standard library and existing workspace crates.

## Test Status

Status: pass.

- Formatting and every required check completed successfully.
- `deepseek-science-common` library tests: 129 passed.
- `deepseek-science-cli` library tests: 39 passed.
- `data_inspect_smoke`: 17 passed.
- `data_convert_smoke`: 18 passed.
- `kinetics_analyze_smoke`: 12 passed.
- `atomic_create_new`: 6 passed.
- Workspace library tests: 370 passed across all crates.
- Manual help, supported end-to-end, matrix refusal, metadata refusal,
  unit-row refusal, AlreadyCompatible, existing-target, version, and doctor
  checks matched the documented contracts.
- No Cargo command was rerun after writing this report.

## Known Risks

The following are deliberate non-blocking limitations:

- Input files are not locked. Concurrent modification can cause a read error
  or produce a result for the bounded bytes actually observed.
- Symlinks follow ordinary platform opening behavior; no cross-platform
  symlink containment policy is claimed.
- Atomic publication requires same-filesystem hard-link support. Unsupported
  filesystems fail safely; there is no non-atomic fallback.
- A cleanup failure after successful hard-link publication can return an
  error while the requested target exists. The CLI therefore instructs users
  to inspect the target before retrying and does not delete it.
- Matrix classification is intentionally conservative and syntax-based.
- Inspection and conversion establish text/shape/parser compatibility only;
  they do not prove units, scientific meaning, or kinetics suitability.
- Full CSV, matrix conversion, metadata removal, unit-row removal, whitespace
  repair, proprietary formats, automatic interpretation, and model assistance
  remain unsupported.
- Real private laboratory data must not be committed as fixtures.

## Functional Readiness

**Yes.** The supported inspection, explicit normalization, re-inspection, and
explicit-column kinetics path is functional, deterministic, bounded, and
disk-safe within the accepted Phase 4 contract. Functional blocking issues:
none.

This conclusion does not claim general laboratory-format support or scientific
validity.

## Version Alignment Readiness

**Yes.** The implemented and audited user-visible Phase 4 path is ready for a
separate focused task that aligns the CLI version from `0.3.0` to `0.4.0`.
This audit intentionally leaves version metadata unchanged.

## Release/Tag Readiness

**No.** This is not the final v0.4 release audit. The audit report must first
be reviewed and committed, the CLI version must then be aligned to `0.4.0`,
and a separate final release audit must validate that exact committed version
state. An annotated `v0.4.0` tag must not be created before those steps pass.

## Blocking Issues

None for functional readiness or the next version-alignment task.

Release/tag readiness remains intentionally blocked by incomplete release
process steps, not by a discovered Phase 4 functional defect.

## Non-blocking Follow-ups

- Review and commit this audit report as its own documentation change.
- Align the CLI package/version-facing metadata to `0.4.0` in a separate
  narrowly scoped task.
- Run one separate final v0.4 release audit after version alignment is
  committed.
- Continue using tiny synthetic or appropriately controlled non-private data
  for future instrument validation.

## Recommended Next Step

Review and commit this audit report. Then align the CLI version to `0.4.0` in
a separate focused task, run and commit the final v0.4 release audit, and only
after that audit passes create the annotated `v0.4.0` tag. Do not create a tag
or GitHub Release from the current `0.3.0` state.
