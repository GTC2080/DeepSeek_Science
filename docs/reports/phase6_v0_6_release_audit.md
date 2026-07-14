# Phase 6.5 v0.6.0 Version Alignment and Release Audit

## Summary

The `deepseek-science-cli` package is aligned from `0.5.0` to `0.6.0`.
The user-visible `version`, `doctor.version`, `doctor.prompt_kernel_version`,
and kinetics artifact `producer_version` surfaces all report `0.6.0` through
their existing `CARGO_PKG_VERSION` derivation.

The Phase 6 implementation and committed Phase 6.4 end-to-end audit passed.
Post-alignment formatting, workspace checks, tests, dependency inspection,
help checks, and an actual artifact producer-version check also passed. This
task does not create an annotated tag or a GitHub Release.

The repository is ready for a separate authorized annotated `v0.6.0` tag task.

## Baseline

The release baseline is annotated tag `v0.5.0`. Its tag object is
`0ea30cdf1565a459f18260ad0e6603561251e2ce`, and it resolves to commit
`0a36cc300f70124d5b5a2578f770908a5d3176d9`.

Phase 6 was delivered by these focused commits before version alignment:

| Phase | Commit | Subject |
| --- | --- | --- |
| 6.0 | `f2bff24e89b2225397f9024f5a5c74a41f9d4996` | `docs: freeze phase 6 artifact envelope RFC` |
| 6.1 | `bd73472fbc20cfffc0d10fc12ecc0d53d51c337c` | `feat: add unregistered artifact envelope contract` |
| 6.2 | `ec1c50f1b0a7241cb5ed8668742795f9ffc62227` | `feat: add deterministic kinetics artifact adapter` |
| 6.3 | `00ee3fb1b023550101221fe66d3e84917c3fe07e` | `feat: add kinetics artifact CLI` |
| 6.4 | `e4fd43726af2053112be35c9322ed6acba337bac` | `docs: add phase 6 kinetics artifact audit` |

Phase 6.5 started from clean synchronized `main` at
`e4fd43726af2053112be35c9322ed6acba337bac`. The pre-alignment actual CLI
reported `deepseek-science 0.5.0`; `doctor` reported `version: 0.5.0`,
`prompt_kernel_version: 0.5.0`, and `status: ok`.

The audit environment was:

| Item | Observed value |
| --- | --- |
| Operating system | Darwin 25.5.0, Darwin Kernel Version 25.5.0 |
| Architecture | arm64 / `aarch64-apple-darwin` |
| Rust | `rustc 1.96.1 (31fca3adb 2026-06-26)` |
| LLVM | `22.1.2` |
| Cargo | `cargo 1.96.1 (356927216 2026-06-26)` |
| Cargo target root | `../.cache/deepseek-science-target` |

All formal Cargo commands used `CARGO_NET_OFFLINE=true`.

## Version Alignment

The only version values changed are:

- `crates/deepseek-science-cli/Cargo.toml`: `0.5.0` to `0.6.0`;
- `Cargo.lock`: the matching local `deepseek-science-cli` package entry from
  `0.5.0` to `0.6.0`.

The version diff is exactly two insertions and two deletions. No dependency
entry, source, checksum, feature, workspace setting, edition, Rust version, or
profile changed. Every other workspace crate remains at `0.1.0`.

No Rust source, test, README, RFC, Phase 6.4 audit, or other crate manifest was
modified. README already describes the artifact command without a current
package-version claim, so it requires no alignment edit.

## Version Surfaces

Post-alignment actual output established:

```text
deepseek-science 0.6.0
```

and:

```text
version: 0.6.0
prompt_kernel_version: 0.6.0
status: ok
```

An actual artifact envelope recorded:

```text
artifact.provenance.producer_command = kinetics.artifact
artifact.provenance.producer_version = 0.6.0
```

Source inspection confirms that the version command, doctor version,
`PromptVersionInfo`, and artifact producer version all receive
`env!("CARGO_PKG_VERSION")` from the CLI crate. No second CLI version constant
or hard-coded production-source `0.6.0` was added.

The schema labels remain unchanged and are not package versions:

```text
kinetics.artifact.v1
kinetics.analysis.v1
```

## Feature Scope

Phase 6 adds one release mainline: the first deterministic provenance-bearing
kinetics artifact envelope. Its focused feature delta is:

- a domain-neutral unregistered artifact envelope contract;
- typed exact-byte BLAKE3 descriptors for content and inputs;
- a bounded deterministic outer JSON serializer;
- a chemistry-owned kinetics metadata and review adapter;
- exact raw-source and exact `kinetics.analysis.v1` payload hashes;
- no random artifact identity, run identity, project identity, or timestamp;
- an independent `kinetics artifact` command;
- a 16 MiB bounded source read and 4 MiB bounded envelope;
- one atomic `WriteMode::CreateNew` output; and
- no payload, manifest, SVG, project, or run sidecar.

Phase 6 does not include registered artifact repositories, run/project
persistence, model integration, network access, RAG, database persistence, UI,
overwrite, multi-file transactions, or a new chemistry workflow.

## Phase 6.4 Evidence

The committed Phase 6.4 audit used the pre-alignment `0.5.0` binary and proved:

- two 2,029-byte artifact envelopes were byte-identical for the same exact
  invocation inputs;
- decoded `payload_utf8` bytes were identical to `kinetics analyze --json`
  stdout, including the payload final LF;
- source and payload byte lengths and exact BLAKE3 values were independently
  verifiable;
- provenance and existing deterministic review metadata were preserved;
- existing targets were not overwritten, missing parents were not created,
  ordinary failures left no operation-owned temporary sibling, and each
  success produced one final file;
- analyze, plot, inspect, and convert compatibility remained intact;
- 474 workspace library tests, 131 CLI package tests, and 14 artifact process
  tests passed; and
- all 66 audit-owned files and the one external audit directory were exactly
  cleaned without recursive deletion.

Those facts are functional evidence for the `0.5.0` pre-alignment build. This
Phase 6.5 change alters package metadata only. The post-alignment tests and
actual producer check below establish the `0.6.0` version surfaces without
misrepresenting the Phase 6.4 binary hash or size.

## Validation

The following pre-alignment checks ran and reported `0.5.0`:

- `CARGO_NET_OFFLINE=true cargo run -p deepseek-science-cli -- version`;
- `CARGO_NET_OFFLINE=true cargo run -p deepseek-science-cli -- doctor`.

After the two version-value edits, the same commands reported `0.6.0`. Final
release validation then ran once:

- `CARGO_NET_OFFLINE=true cargo fmt --check`;
- `CARGO_NET_OFFLINE=true cargo check --workspace`;
- `CARGO_NET_OFFLINE=true cargo test --workspace --lib`;
- `CARGO_NET_OFFLINE=true cargo test -p deepseek-science-cli`;
- `CARGO_NET_OFFLINE=true cargo tree --workspace`.

Formatting and workspace check passed. The workspace check emitted no warning.
Workspace library tests passed with these actual counts:

| Crate | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| `deepseek-science-artifacts` | 38 | 0 | 0 |
| `deepseek-science-chemistry` | 144 | 0 | 0 |
| `deepseek-science-cli` library | 55 | 0 | 0 |
| `deepseek-science-common` | 129 | 0 | 0 |
| `deepseek-science-core` | 47 | 0 | 0 |
| `deepseek-science-model` | 8 | 0 | 0 |
| `deepseek-science-model-deepseek` | 2 | 0 | 0 |
| `deepseek-science-prompt` | 9 | 0 | 0 |
| `deepseek-science-sandbox` | 10 | 0 | 0 |
| `deepseek-science-storage` | 16 | 0 | 0 |
| `deepseek-science-tools` | 16 | 0 | 0 |
| **Total** | **474** | **0** | **0** |

The complete CLI package passed 131 tests:

| Suite | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| CLI library | 55 | 0 | 0 |
| CLI binary | 0 | 0 | 0 |
| `data_convert_smoke` | 18 | 0 | 0 |
| `data_inspect_smoke` | 17 | 0 | 0 |
| `kinetics_analyze_smoke` | 12 | 0 | 0 |
| `kinetics_artifact_smoke` | 14 | 0 | 0 |
| `kinetics_plot_smoke` | 15 | 0 | 0 |
| CLI doc tests | 0 | 0 | 0 |
| **Total** | **131** | **0** | **0** |

Final smoke checks passed for `version`, `doctor`, `kinetics artifact --help`,
`kinetics analyze --help`, `kinetics plot --help`, `data inspect --help`, and
`data convert --help`. Version and doctor still reported `0.6.0`.

## Artifact Producer Version Check

The post-alignment producer check used only this external directory:

```text
/Users/taomu/开发/.cache/deepseek-science-target/tmp/phase6-5-version-audit-e4fd437
```

Its synthetic, LF-only input was 43 bytes:

```csv
time_s,concentration_mol_l
0,1
1,0.8
2,0.6
```

The actual debug binary ran `kinetics artifact` with exact time and
concentration columns and a fresh JSON target. The reliable recorded invocation
exited zero, wrote exact 27-byte `kinetics artifact complete\n` stdout, and
wrote empty stderr.

The resulting 1,897-byte JSON envelope had SHA-256
`f5caffb63e5a32a43850a2048e4d74f47fcfbfea5378025c1d85a7b8bd86cf75`.
System JSON conversion and extraction succeeded. The observed values were:

```text
outer schema = kinetics.artifact.v1
content and payload schema = kinetics.analysis.v1
producer command = kinetics.artifact
producer version = 0.6.0
```

The first audit wrapper used the zsh read-only variable name `status` after
the product process completed, so it could not retain a trustworthy exit-code
record. The three exact generated output paths were removed, and the command
was rerun once with `command_status`; that invocation supplied the evidence
above. This was an audit-wrapper correction, not a product or repository
change.

The audit used four unique file paths: input, artifact, stdout, and stderr. The
output trio was created twice because of the controlled wrapper retry, for
seven file-creation events in total. All seven generated file instances and the
one owned directory were removed by exact path. No generated artifact was
committed.

## Dependency and Boundary Audit

`cargo tree --workspace` shows `deepseek-science-cli v0.6.0` and every other
workspace crate at `0.1.0`. No dependency was added, removed, upgraded, or
feature-adjusted.

The tree contains no network client, async runtime, database, browser/UI
framework, JSON canonicalization library, temporary-file framework, logging
framework, or RAG/vector/retrieval dependency. The existing `uuid` dependency
remains part of earlier registered-identity contracts; the unregistered
envelope path does not use it.

There is no Rust source delta, test delta, README delta, schema delta, or crate
ownership change. Artifacts still owns the generic envelope, chemistry owns the
kinetics adapter, CLI owns version and orchestration, and storage owns opaque
atomic publication.

## Release Delta

The committed pre-alignment range `v0.5.0..e4fd437` consists exactly of the five
Phase 6 commits listed in the baseline. It changes 15 files with 4,573
insertions and 26 deletions.

The version-alignment candidate additionally changes exactly three paths:

1. `crates/deepseek-science-cli/Cargo.toml`, one version value;
2. `Cargo.lock`, the matching local package version value;
3. this final release audit report.

The intended commit subject is:

```text
chore: align v0.6 version and add release audit
```

Historical Phase 1 through Phase 5 capabilities are not reclassified as Phase
6 changes.

## Disk Safety

Cargo wrote only to the configured external target root:

```text
/Users/taomu/开发/.cache/deepseek-science-target
```

The version audit created one owned directory and four unique small file paths.
Because the audit wrapper was corrected once, the three generated output paths
were created and removed twice: seven file-creation events and seven exact-file
removals total. Cleanup used explicit `rm -f` paths followed by one `rmdir` for
the verified-empty owned directory.

No glob, recursive deletion, `find -delete`, `remove_dir_all`, or `cargo clean`
was used. No release build, package, publish, install, documentation build,
benchmark, fuzzing, coverage, profiling, browser, rasterizer, watcher, daemon,
or background loop ran. No generated JSON, CSV, fixture, snapshot, golden file,
database, cache, or log remains in the repository.

The product version/artifact paths performed no model, tool, network, RAG,
database, registered persistence, watcher, daemon, or background activity. The
only network operations in this task were the required Git/GitHub preflight and
final push/status checks.

## Tag and Release Status

At task start, local and remote `v0.6.0` refs were absent and
`gh release view v0.6.0` reported `release not found`. No command in this task
creates or pushes a tag, and no GitHub Release write command is used.

The annotated `v0.5.0` tag remains unchanged and resolves to
`0a36cc300f70124d5b5a2578f770908a5d3176d9`. Tag creation is reserved for a
separate explicitly authorized task. A GitHub Release must not be created
unless separately requested.

## Blocking Issues

None.

## Conclusion

The v0.6.0 version alignment and final release audit passed.

After this focused three-file candidate is committed on synchronized `main`,
the repository is ready for a separate authorized annotated-tag task. No tag
or GitHub Release was created by Phase 6.5.
