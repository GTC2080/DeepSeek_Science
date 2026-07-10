# Phase 4.3a Data Inspect User Path Audit

## Summary

The merged `deepseek-science data inspect --input <path>` user path is functionally ready for its deliberately narrow, read-only inspection scope. The implementation reuses the pure Phase 4.1 encoding contract and Phase 4.2 delimited-table contract, opens one explicit regular file, enforces the fixed 16 MiB boundary, emits a deterministic text report, and creates no product state or output files.

The command remains an inspector rather than an importer, converter, or scientific analyzer. Structural limitations are reported without repair, fatal file or decoding failures remain errors, and the compatibility wording does not claim scientific validity or infer chemistry column roles. Existing `kinetics analyze` behavior remains separate and unchanged.

This is not a v0.4 release-readiness finding. It establishes only that the completed inspection path is a suitable foundation for a separate Phase 4.4 conversion RFC.

## Commands Run

Initial repository checks:

- `git status --short`
- `git branch --show-current`
- `git remote -v`
- `git log --oneline --decorate -n 10`

Required validation, each Cargo command run once:

- `cargo fmt --check`
- `cargo check -p deepseek-science-common`
- `cargo test -p deepseek-science-common --lib`
- `cargo check -p deepseek-science-cli`
- `cargo test -p deepseek-science-cli --lib`
- `cargo test -p deepseek-science-cli --test data_inspect_smoke`
- `cargo test -p deepseek-science-cli --test kinetics_analyze_smoke`
- `cargo check --workspace`
- `cargo test --workspace --lib`
- `cargo tree --workspace`

Manual checks, each run once with tiny audit-owned files beneath the configured external Cargo target test area:

- `cargo run -p deepseek-science-cli -- data --help`
- `cargo run -p deepseek-science-cli -- data inspect --help`
- UTF-8 comma narrow-table inspection
- UTF-8 tab narrow-table inspection
- generic numeric-matrix inspection
- quoted structural-unsupported inspection
- fatal NUL/binary encoding inspection

Read-only source searches and reviews covered:

- filesystem writes and file-loading APIs in the `data inspect` path;
- `std::env::temp_dir` and recursive cleanup usage;
- chemistry-keyword conditionals in common inspection production code;
- duplicated encoding or delimiter logic in the CLI;
- Phase 4.1-4.3 Cargo manifest and lockfile changes;
- report ordering, label escaping, bounded display, and compatibility mapping.

After the report was written, only the requested no-index report diff and final short Git status were run.

## Git Status

- Initial branch: `main`.
- Initial `HEAD`: `977f13e3541947a45c2e3fc508c17d58dfe41b37`.
- Initial `main` and `origin/main`: aligned at `977f13e`.
- Initial worktree: clean.
- PR #34 is present in current history through merge commit `977f13e`.
- No task branch was created.
- No file was staged, committed, or pushed during this audit.
- The only final worktree change is this audit report.

## CLI Surface Status

Status: pass.

- `deepseek-science data inspect --input <path>` is routed and available.
- `data inspect --help`, `data inspect -h`, `data --help`, and `data -h` are supported.
- Parent help lists `inspect` and does not expose `data convert` as an implemented command.
- The inspect parser accepts one required `--input <path>` plus `--help` or `-h`.
- Missing input, a missing input value, duplicate `--input`, unknown flags, and unexpected positional arguments are rejected.
- `--json` and `--output` are not accepted by `data inspect`; their existing kinetics meanings are not reused.
- Help documents the fixed 16 MiB limit, supported encodings, comma/tab-only inspection, no-write behavior, and the absence of modification, normalization, conversion, or analysis.

## Bounded Read Status

Status: pass.

- The CLI opens exactly the caller-provided path once through `File::open`.
- Metadata is obtained from the opened handle, and the handle must describe a regular file.
- The input path is not canonicalized for normal reporting; parent and sibling paths are not scanned.
- Metadata length greater than `MAX_INSPECTION_BYTES` is rejected before proportional input allocation.
- The reader is capped with `Read::take(MAX_INSPECTION_BYTES + 1)` and then checks the observed length, so post-metadata file growth cannot bypass the 16 MiB boundary.
- The resulting bytes are passed once to the existing encoding inspector, and the decoded text is passed once to the existing delimited inspector. The file is not reread.
- The `data inspect` path does not use `read_to_string`, unbounded `read`, unbounded `read_to_end`, memory mapping, async I/O, or a loader framework. The bounded `read_to_end` call is applied only to the already capped `Take` reader.

## Encoding Status

Status: pass.

- Strict UTF-8 without a BOM succeeds.
- A single UTF-8 BOM is recognized, stripped, and reported.
- BOM-marked UTF-16LE and UTF-16BE succeed, including strict surrogate handling.
- BOM-free invalid UTF-8 is rejected as unsupported or ambiguous rather than guessed as UTF-16.
- Invalid UTF-8 after a UTF-8 BOM and invalid UTF-16 after a UTF-16 BOM are fatal and preserve original-input byte offsets.
- UTF-32 BOMs are rejected before the UTF-16LE prefix can match.
- NUL and binary evidence are rejected before table inspection.
- Decoding is non-lossy; no replacement-character or statistical detection path exists.
- Encoding failures return nonzero with empty success stdout and concise stderr without byte or decoded-content dumps.

## Delimiter and Shape Status

Status: pass.

- Production inspection considers only comma and tab delimiters.
- No semicolon, locale, whitespace, arbitrary delimiter, or RFC 4180 inference exists.
- A delimiter requires a stable multi-field rectangular region with numeric-row evidence; isolated punctuation is insufficient.
- Narrow-table classification requires a named header and a finite rectangular numeric body.
- Matrix classification is driven by monotonic first-column structure and repeated sibling-header syntax, not chemistry vocabulary.
- Empty, quoted, ambiguous, mixed, unit/header-row, non-finite, empty-cell, and inconsistent-width structures are reported conservatively rather than repaired.
- A numeric matrix remains generically classified as a matrix and is never converted into time/concentration semantics.
- Common production code contains no classification branch keyed on wavelength, absorbance, kinetics, time, concentration, molarity, spectrum, or other chemistry terms.

## Exit-status Status

Status: pass.

Completed inspections return exit status 0 with a report on stdout and empty stderr for:

- normal comma narrow tables;
- tab-delimited narrow tables;
- numeric matrices;
- empty input;
- quoted input;
- mixed or unsupported structure;
- delimiter ambiguity.

Fatal failures return nonzero with empty success stdout and concise stderr for:

- invalid arguments;
- missing input files;
- directory input;
- the fixed size limit being exceeded;
- unsupported or ambiguous encoding;
- invalid UTF-8 or UTF-16;
- binary or NUL evidence.

Structural findings do not leak unrelated usage text, while argument parser failures retain the existing concise parser style. Fatal messages do not expose internal enum debug output.

## Report Contract Status

Status: pass.

- Successful output is line-oriented, deterministic, and emitted in a stable field order.
- Output ends with exactly one trailing newline.
- The report contains no timestamp, random identifier, ANSI color, progress output, terminal-width-dependent layout, absolute derived path, or model-generated prose.
- Stable fields cover inspection status, encoding, BOM, original byte count, delimiter, physical/blank/nonblank line counts, selected table region, field count, header evidence and labels, numeric/nonnumeric row evidence, metadata evidence, generic shape, simple CSV compatibility, current kinetics-workflow assessment, and structured findings.
- Inapplicable values are represented deterministically with `none` rather than omitted unpredictably.
- Repeated process inspection produces byte-identical stdout.

## Untrusted Output Safety

Status: pass.

- Header newline, carriage return, tab, backslash, general control characters, U+2028, and U+2029 are escaped before display.
- A header cannot inject an additional report line or report field.
- Header output is bounded to 32 displayed labels and 120 Unicode scalar values per label.
- Truncation uses a fixed deterministic marker.
- Normal Unicode labels are retained; the formatter does not rename scientific labels.
- Data-body cells are not printed.
- Raw binary bytes and decoded private file contents are not included in fatal errors.

## Kinetics Compatibility Wording

Status: pass.

- A compatible narrow UTF-8 comma CSV is described only as `potentially-compatible-after-explicit-column-selection`.
- The wording preserves the requirement for users to name exact time and concentration columns and does not claim scientific suitability.
- BOM-marked, UTF-16, or tab-delimited narrow tables report `requires-normalization-before-analysis` and explicitly state that conversion is not implemented.
- Matrices, empty input, and mixed or unsupported structures report `incompatible`.
- No time, concentration, wavelength, absorbance, unit, reaction role, or scientific validity is inferred.
- `data inspect` never invokes `kinetics analyze` or constructs a chemistry analysis request.

## Disk Safety Status

Status: pass.

Production command:

- reads one explicit file;
- writes no file;
- creates no directory or temporary file;
- creates no cache, log, project, workspace, run, or artifact record;
- performs no deletion or recursive scan;
- starts no background task.

Process tests:

- create only tiny inputs beneath `CARGO_TARGET_TMPDIR`;
- use PID plus a bounded atomic counter for exact test-owned directories;
- inspect directory contents before cleanup to prove the command created no side effects;
- remove only the exact test input and exact empty owned directory;
- do not use `std::env::temp_dir` or `remove_dir_all`;
- leave no test-owned files behind.

Manual audit inputs were created only under:

`/Users/taomu/开发/.cache/deepseek-science-target/tmp/phase4-3a-data-inspect-audit-manual`

Before cleanup, the directory contained exactly the five explicitly created tiny audit inputs. Those exact files and the resulting empty audit-owned directory were removed; no audit input remains.

## Existing CLI Compatibility

Status: pass.

- Existing `kinetics analyze` text output behavior remains covered by its smoke suite.
- Existing `--json` output behavior remains unchanged and covered.
- Existing explicit `--output` persistence, no-overwrite, and related failure behavior remain covered.
- No existing output schema or storage semantic changed in the Phase 4.3 path.
- `version` and `doctor` routing and library coverage remain intact.
- The CLI version remains `0.3.0`; this audit makes no version change and does not assess v0.4 release readiness.

## Crate Boundary Status

Status: pass.

- Encoding and delimited-table inspection remain pure, standard-library logic in `deepseek-science-common`.
- File opening, bounded reading, fatal error presentation, and report formatting remain in `deepseek-science-cli`.
- Chemistry remains responsible for kinetics analysis and is not used by `data inspect`.
- Storage is not called by `data inspect`.
- No common-to-chemistry dependency appears.
- The CLI reuses `inspect_text_encoding`, `inspect_delimited_text`, and the generic simple CSV compatibility assessment; it does not duplicate encoding or delimiter algorithms.
- No new workspace crate was introduced for Phase 4 inspection.

## Dependency Status

Status: pass.

- `cargo tree --workspace` completed successfully.
- The Phase 4.1-4.3 history contains no Cargo manifest or lockfile change relative to the accepted Phase 4.0 RFC baseline.
- No dependency was added for encoding detection, delimited inspection, bounded file reading, or CLI parsing.
- There is no `clap`, broad CSV framework, encoding detector, async runtime, database, logging framework, UI framework, or TypeScript toolchain in this path.
- The implementation relies on the Rust standard library and the existing workspace contracts.

## Test Status

Status: pass.

- Formatting and all required package/workspace checks passed.
- `deepseek-science-common` library tests: 103 passed.
- `deepseek-science-cli` library tests: 32 passed.
- `data_inspect_smoke`: 17 passed.
- `kinetics_analyze_smoke`: 12 passed.
- Workspace library tests: 337 passed in total across the workspace crates.
- Manual help, narrow UTF-8, tab, matrix, structural unsupported, and fatal binary/NUL checks matched the documented exit and report contract.
- No validation command was rerun after this report was written.

## Known Risks

These risks are deliberate and non-blocking for the current inspection contract:

- The input file is not locked. Concurrent modification can cause a read failure or produce a report for the bounded bytes actually observed.
- Symlinks follow ordinary platform opening behavior; the resolved opened handle must still describe a regular file.
- Matrix classification is intentionally conservative and syntax-driven, so some real matrices may remain `NumericNarrowTable` or `MixedOrUnsupported` rather than being guessed.
- Inspection establishes text and table-structure evidence only; it does not prove scientific suitability, correct units, or valid kinetics semantics.
- Conversion and normalization are not implemented.
- Real private laboratory or instrument data must not be committed as fixtures; future validation should continue using synthetic or appropriately controlled tiny inputs.

None of these risks contradicts the documented Phase 4.3 contract.

## Functional Readiness

**Yes.** The merged `data inspect` path is functionally ready for bounded, deterministic, read-only inspection of the explicitly supported encoding and comma/tab structures. The finding does not extend to full CSV compatibility, arbitrary instrument exports, conversion, direct UTF-16 kinetics analysis, or scientific correctness.

## Phase 4.4 RFC Readiness

**Yes.** The inspection path provides the necessary deterministic evidence and separation of responsibilities for designing a separate explicit conversion contract. The Phase 4.4 RFC can define conversion eligibility, normalized output rules, atomic `CreateNew` persistence, and failure behavior without changing the current inspection semantics.

## Blocking Issues

None.

## Non-blocking Follow-ups

- A future focused regression may directly exercise the 120-scalar label truncation and 32-label display cap; current implementation review and control-character tests already establish bounded safe display.
- The 16 MiB filesystem boundary should continue to be covered through the private small-limit reader unit test rather than a large committed or runtime-created fixture.
- Future real-instrument validation should use tiny synthetic or project-controlled non-private samples and must not broaden supported formats implicitly.

## Recommended Next Step

Review and commit this audit report, then design the explicit conversion contract in a separate Phase 4.4 RFC-only task. Do not add conversion behavior until that RFC defines eligible inputs, normalization semantics, resource limits, no-overwrite atomic output, and rollback expectations.
