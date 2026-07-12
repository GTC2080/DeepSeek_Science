# Phase 5.5 v0.5.0 Version Alignment and Release Audit

## Summary

The Phase 5 visualization scope is ready for v0.5.0 tag preparation. The
`deepseek-science-cli` package is aligned from `0.4.0` to `0.5.0`, and the
user-visible `version`, `doctor`, and prompt-kernel version surfaces all derive
the same value from `CARGO_PKG_VERSION`.

The deterministic kinetics plot-data boundary, fixed SVG renderer, explicit
`kinetics plot` CLI, bounded input and output contracts, and atomic
create-new publication remain unchanged from their audited implementations.
All required validation passed. No `v0.5.0` tag or GitHub Release was created;
both are deliberately outside this task.

## Baseline

The released baseline is annotated tag `v0.4.0` at
`6996f404386abeed84514c1cce8ea32b4a413181`.

Phase 5 implementation and audit commits before version alignment are:

- Phase 5.0 RFC: `8ed2a77c887e0ca0d9cb123117dd0780ed9c31c6`;
- Phase 5.1 plot data: `54c266c4e574c932ef87c8cd0006a84cb4f86438`;
- Phase 5.2 SVG renderer: `f5a22f3fab4d4e13d9842ba6b37a284ae226c6f5`;
- Phase 5.3 CLI: `0c10c05bc0aea61d8f977b8b0a3ba7c823fd3dca`;
- Phase 5.4 end-to-end audit:
  `33d3f2434c1ead1bcf3f34bda0e24564f072a57c`.

The version-alignment work started from clean, synchronized `main` at
`33d3f2434c1ead1bcf3f34bda0e24564f072a57c`, with `HEAD == origin/main` and no
tag pointing at `HEAD`.

Audit environment:

- operating system: Darwin 25.5.0, arm64 (`aarch64-apple-darwin`);
- Rust: `rustc 1.96.1 (31fca3adb 2026-06-26)`;
- Cargo: `cargo 1.96.1 (356927216 2026-06-26)`.

## Version Alignment

The root workspace does not define a shared package version. The CLI package
is independently versioned in
`crates/deepseek-science-cli/Cargo.toml`, matching the existing repository
policy used for prior CLI releases.

The minimum alignment changed only:

- `crates/deepseek-science-cli/Cargo.toml`: package version `0.4.0` to
  `0.5.0`;
- `Cargo.lock`: the local `deepseek-science-cli` package entry from `0.4.0`
  to `0.5.0`.

No other workspace crate version changed. All other workspace packages remain
at `0.1.0`, and no dependency name or dependency version changed. README
inspection found no current-version or release-status statement requiring an
edit; its existing Phase 5 plot documentation remains accurate.

Before alignment, the actual CLI reported:

```text
deepseek-science 0.4.0
```

and `doctor` reported both `version: 0.4.0` and
`prompt_kernel_version: 0.4.0` with `status: ok`.

After alignment, the actual CLI reported:

```text
deepseek-science 0.5.0
```

and `doctor` reported:

```text
version: 0.5.0
prompt_kernel_version: 0.5.0
status: ok
```

The `version` command, `doctor` command, and prompt version information all
continue to use `env!("CARGO_PKG_VERSION")`; no second version constant or
hard-coded Rust version was introduced.

## Feature Scope

The v0.5 candidate adds the first deterministic visualization path for the
existing kinetics workflow:

- chemistry-owned immutable `KineticsPlotData` retaining accepted
  observations, fit metadata, deterministic predictions, curve segments,
  comparison/review state, counts, and bounded ordered warnings;
- a chemistry-owned, fixed 960 by 640 standalone SVG renderer with no plotting
  or XML dependency;
- the independent `kinetics plot` CLI with explicit input, selected columns,
  and one explicit `.svg` output;
- a 16 MiB plot-input limit, strict UTF-8 handling, exactly 128 candidate
  positions per model, at most three visualization warnings, and a 4 MiB SVG
  output limit;
- one atomic opaque-byte publication through `AtomicWriteRequest`,
  `WriteMode::CreateNew`, and `AtomicWritePlan::execute`, without overwrite or
  parent creation;
- cautious scientific wording that reports the existing finite-r-squared MVP
  heuristic preference without claiming a confirmed reaction order or
  validated mechanism.

The plot flow parses once, validates once, calls
`KineticsAnalysisResult::analyze` once, constructs plot data once, renders
once, validates the completed SVG boundary, and executes publication once.
The renderer neither parses CSV nor refits or regenerates chemistry
predictions. The CLI creates no JSON sidecar, and `kinetics.analysis.v1`
remains unchanged.

Compatibility is preserved for `kinetics analyze`, `data inspect`, and
`data convert`. Plotting does not change the existing unbounded
`fs::read_to_string` behavior of `kinetics analyze`; the 16 MiB limit is
specific to `kinetics plot`.

## Validation

Identity and repository checks confirmed GitHub account `GTC2080`, matching
Git author metadata, clean synchronized `main`, the expected Phase 5.4
baseline, and absence of local/remote `v0.5.0` tags and a GitHub Release.

The pre-alignment red check ran:

- `cargo run -p deepseek-science-cli -- version`;
- `cargo run -p deepseek-science-cli -- doctor`.

Both succeeded and established the prior `0.4.0` output. The same commands
after the version change succeeded with aligned `0.5.0` output.

Release validation ran once with these commands:

- `cargo fmt --check`;
- `cargo check --workspace`;
- `cargo test --workspace --lib`;
- `cargo test -p deepseek-science-cli`;
- `cargo tree --workspace`;
- `cargo run -p deepseek-science-cli -- kinetics plot --help`;
- `cargo run -p deepseek-science-cli -- kinetics analyze --help`;
- `cargo run -p deepseek-science-cli -- data inspect --help`;
- `cargo run -p deepseek-science-cli -- data convert --help`.

All commands passed. `cargo fmt --check` found no formatting change, and
`cargo check --workspace` completed without warnings. Workspace library tests
passed with these per-crate counts:

- artifacts: 8;
- chemistry: 134;
- CLI library: 42;
- common: 129;
- core: 47;
- model: 8;
- model-deepseek: 2;
- prompt: 9;
- sandbox: 10;
- storage: 16;
- tools: 16.

That is 421 passing workspace library tests, with zero failures and zero
ignored tests. The complete CLI package passed 104 tests: 42 library tests,
18 data-convert process tests, 17 data-inspect process tests, 12 kinetics
analysis process tests, and 15 kinetics plot process tests. Its binary and
documentation test targets contained no additional tests.

The plot process suite includes the real
`exact_limit_is_read_and_limit_plus_one_is_rejected` command path, so both the
exact 16 MiB boundary and the limit-plus-one rejection remain covered. The
plot, analyze, inspect, and convert help commands all exited successfully and
retained their existing command-specific contracts.

## Phase 5.4 Evidence

The committed Phase 5.4 audit used one non-private, dynamically generated,
LF-only synthetic CSV containing nine accepted observations and one rejected
zero-concentration row. The 130-byte input had SHA-256
`85ac6f132f59f4e04abc7395abc0f91a7e645faad515c6d4993b705ffd10101f`
and was not committed as a fixture.

Two different fresh output paths produced byte-identical SVGs. Each SVG was
11,796 bytes and had SHA-256
`8c0d0e84e17b7df37084bd79882c28614dbf4e69813c853ca80d26f871755729`.
The result establishes same-build, same-platform byte identity only. It does
not claim cross-architecture identity; in particular, standard-library
`f64::exp` does not provide a formal cross-platform bit-for-bit guarantee.

The audit also confirmed:

- nine observation circles, first-order and second-order curves, deterministic
  review/preference text, accepted/rejected counts, and one rejected-row
  visualization warning;
- no overwrite of an existing sentinel target;
- no creation of a missing parent;
- unchanged input bytes;
- no operation-owned temporary sibling after success or refusal;
- exact success stdout and empty failure stdout;
- no input/output/temp paths, scripts, external resources, timestamps, UUIDs,
  or definitive scientific claims in the SVG;
- unchanged `kinetics analyze`, `data inspect`, `data convert`, explicit JSON
  CreateNew behavior, and `kinetics.analysis.v1` schema.

All Phase 5.4 synthetic CSV, SVG, sentinel, JSON, converted output, and capture
files were external audit-owned assets and were precisely removed after the
audit. None became a committed fixture, snapshot, or golden output.

## Dependency and Boundary Audit

`cargo tree --workspace` shows `deepseek-science-cli v0.5.0` and every other
workspace crate at `0.1.0`. The manifest/lock diff changes only the local CLI
package version. No dependency was added, removed, or upgraded.

The dependency tree contains no plotting library, XML DOM, browser or headless
Chromium, JavaScript or TypeScript runtime, async runtime, database client,
network client, or UI framework.

Crate ownership remains intact:

- `deepseek-science-common` contains generic bounded table/encoding utilities
  and no chemistry logic;
- `deepseek-science-chemistry` owns kinetics validation, analysis, plot-data
  preparation, model sampling, and fixed SVG rendering;
- `deepseek-science-cli` owns argument parsing, the 16 MiB bounded input path,
  orchestration, post-render byte checks, user output, and explicit output-path
  handling;
- `deepseek-science-storage` receives opaque bytes and owns atomic CreateNew
  publication without chemistry or SVG interpretation.

No model call, network request, UI, tool execution, background work, cache, or
hidden persistence is introduced by the visualization path.

## Release Delta

The committed implementation delta from `v0.4.0` through the pre-alignment
HEAD `33d3f2434c1ead1bcf3f34bda0e24564f072a57c` consists of five focused
commits: the Phase 5 RFC, chemistry plot-data contract, deterministic SVG
renderer, `kinetics plot` CLI integration, and Phase 5.4 end-to-end audit.
That range changes nine files with 4,800 insertions and 11 deletions.

The version-alignment candidate additionally changes only the CLI manifest,
the matching local package entry in `Cargo.lock`, and this release audit
report. README requires no current-version correction.

The v0.5 scope does not include PNG or PDF export, HTML or UI, interactive or
customizable charts, RAG, model calls, network behavior, overwrite support, a
generic plotting framework, a tag, or a GitHub Release. Historical Phase 1
through Phase 4 behavior is not reclassified as new Phase 5 functionality.

## Disk Safety

Cargo wrote only to the repository's configured external target directory:
`/Users/taomu/开发/.cache/deepseek-science-target`. This task did not run
`cargo clean`, a release build, `cargo install`, benchmarks, coverage, fuzzing,
documentation builds, browsers, rasterizers, watchers, daemons, or background
test loops.

No target, cache, Git object, or user file was deleted or cleaned. No recursive
deletion was performed. This task created no SVG, CSV, fixture, snapshot,
golden file, database, log, or other generated repository asset.

Repository modifications are limited to the CLI package version, the
corresponding lockfile package version, and this release audit report.

## Tag and Release Status

Immediately before final staging:

- no tag pointed at `HEAD`;
- local `v0.5.0` was absent;
- `refs/tags/v0.5.0` was absent from `origin`;
- `gh release view v0.5.0` reported `release not found`.

Tag creation is deliberately deferred. This candidate is ready only for a
separate, explicitly authorized annotated `v0.5.0` tag task. A GitHub Release
must not be created unless it is separately requested.

## Blocking Issues

None.

## Conclusion

The v0.5.0 version alignment and release audit passed. The CLI package and all
version surfaces derived from it report `0.5.0`; required validation,
dependency review, crate-boundary review, compatibility checks, and disk-safety
checks passed without a blocking issue.

After this focused alignment and audit commit is present on synchronized
`main`, the repository is ready for a separate authorized annotated-tag task.
No GitHub Release should be created unless separately requested.
