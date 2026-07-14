# Phase 6.4 End-to-End Kinetics Artifact Audit

## Audit Scope

This audit evaluates the committed deterministic kinetics artifact envelope from
its generic in-memory contract through the chemistry adapter, CLI orchestration,
actual debug binary, and single-file atomic publication boundary. The audit is
docs-only and changes no implementation, test, manifest, lockfile, README,
version, tag, or GitHub Release.

The reviewed Phase 6 commits are:

| Phase | Commit | Subject |
| --- | --- | --- |
| Phase 6.0 | `f2bff24e89b2225397f9024f5a5c74a41f9d4996` | `docs: freeze phase 6 artifact envelope RFC` |
| Phase 6.1 | `bd73472fbc20cfffc0d10fc12ecc0d53d51c337c` | `feat: add unregistered artifact envelope contract` |
| Phase 6.2 | `ec1c50f1b0a7241cb5ed8668742795f9ffc62227` | `feat: add deterministic kinetics artifact adapter` |
| Phase 6.3 | `00ee3fb1b023550101221fe66d3e84917c3fe07e` | `feat: add kinetics artifact CLI` |

The audit baseline was clean synchronized `main` at
`00ee3fb1b023550101221fe66d3e84917c3fe07e`. The annotated `v0.5.0` tag remained
at tag object `0ea30cdf1565a459f18260ad0e6603561251e2ce` and resolved to
`0a36cc300f70124d5b5a2578f770908a5d3176d9`. No local or remote `v0.6.0` tag
existed, and GitHub reported no Release for `v0.5.0` or `v0.6.0`.

This task does not align the version, create a tag, or create a Release.

## Environment

| Item | Observed value |
| --- | --- |
| Operating system | Darwin 25.5.0, Darwin Kernel Version 25.5.0 |
| Architecture | arm64 / `aarch64-apple-darwin` |
| Rust | `rustc 1.96.1 (31fca3adb 2026-06-26)` |
| LLVM | `22.1.2` |
| Cargo | `cargo 1.96.1 (356927216 2026-06-26)` |
| Cargo target root | `../.cache/deepseek-science-target` |
| Audit root | `/Users/taomu/开发/.cache/deepseek-science-target/tmp/phase6-4-artifact-audit-00ee3fb` |

`.cargo/config.toml` still places Cargo output outside the source tree. All
formal Cargo commands used `CARGO_NET_OFFLINE=true`.

## Source and Contract Inspection

The production `kinetics artifact` path is one linear orchestration:

1. parse the four required arguments once;
2. reject lexical input/output equality and a non-JSON target;
3. open one explicit regular file and perform one bounded raw-byte read;
4. strictly decode the same bytes as BOM-free UTF-8;
5. call the existing simple numeric CSV parser once;
6. construct `KineticsColumns` once;
7. construct `ValidatedKineticsInput` once;
8. call `KineticsAnalysisResult::analyze` once;
9. call the existing `format_kinetics_analysis_json_output` once;
10. call `prepare_kinetics_artifact_envelope` once;
11. call the generic bounded outer serializer once;
12. run one CLI postcondition validation;
13. create one `WriteMode::CreateNew` plan and call `plan.execute` once; and
14. emit the fixed success line only after publication succeeds.

Inspection found no second payload serializer, payload parse-and-reserialize,
second source read, independent fitting, independent reviewer, sidecar writer,
manifest writer, run/project persistence, overwrite fallback, parent creation,
model/tool/network call, or background task in this path.

Crate ownership matches the RFC:

- `deepseek-science-artifacts` owns exact-byte hash descriptors, generic
  unregistered metadata, payload invariants, declaration-ordered serialization,
  and the bounded in-memory writer. It performs no file IO.
- `deepseek-science-chemistry` owns fixed kinetics metadata and maps the review
  status and existing findings length without rerunning review.
- `deepseek-science-cli` owns path validation, bounded input IO, the existing
  payload serializer invocation, producer command/version, postconditions, and
  publication orchestration.
- `deepseek-science-storage` treats envelope bytes as opaque and owns the
  create-new temporary-sibling/hard-link publication sequence.
- `deepseek-science-core` is not introduced into envelope execution and remains
  domain-neutral.

The legacy `KineticsArtifactProposal` remains a separate semantic-hash contract;
the envelope does not copy its canonical semantic hash or empty input hash list.

## Validation Commands

The audit ran the following command classes:

- identity, clean-tree, synchronized-branch, tag, and GitHub Release preflight;
- `uname -a`, `rustc -vV`, `cargo -V`, and `.cargo/config.toml` inspection;
- bounded `rg`, `sed`, `nl`, `git show`, and `git diff` source/contract checks;
- `CARGO_NET_OFFLINE=true cargo fmt --check`;
- `CARGO_NET_OFFLINE=true cargo check --workspace`;
- `CARGO_NET_OFFLINE=true cargo test --workspace --lib`;
- `CARGO_NET_OFFLINE=true cargo test -p deepseek-science-cli`;
- `CARGO_NET_OFFLINE=true cargo tree --workspace`;
- actual binary `version`, `doctor`, and all relevant help commands;
- direct actual-binary artifact, analyze, plot, inspect, and convert calls;
- `cmp`, `wc`, `shasum`, `od`, `head`, `tail`, `grep`, `plutil -extract`, and
  explicit inventory checks; and
- exact-path external asset cleanup followed by Git diff/status checks.

No release build, documentation build, coverage, profiling, benchmark, fuzzing,
installation, dependency download, or `cargo clean` command was run.

## Test Results

`cargo fmt --check` and `cargo check --workspace` passed without warnings.

Workspace library tests passed as follows:

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
| **Workspace library total** | **474** | **0** | **0** |

The CLI package test command passed 131 tests with no failures or ignored tests:

| Suite | Passed |
| --- | ---: |
| CLI unit tests | 55 |
| `data_convert_smoke` | 18 |
| `data_inspect_smoke` | 17 |
| `kinetics_analyze_smoke` | 12 |
| `kinetics_artifact_smoke` | 14 |
| `kinetics_plot_smoke` | 15 |
| **CLI total** | **131** |

The 14 artifact process tests independently parse actual envelope JSON, compare
decoded payload bytes with actual analyze stdout, recompute source and payload
BLAKE3 with the production `hash_bytes` function, test deterministic output,
exercise review mapping and failure paths, and verify exact cleanup.

## Synthetic Data

All data was synthetic, non-private, LF-only where valid, and created only in
the external audit root.

| Input | Bytes | Lines | SHA-256 | Intended property |
| --- | ---: | ---: | --- | --- |
| `warning.csv` | 214 | 11 | `8c5f834459ca1b37cf1e1dd5aa6e44504e968b60ceb67b19e25fb8c429e656f1` | 9 accepted, 1 zero-concentration rejected, nonmonotonic caller row order |
| `clean.csv` | 209 | 10 | `607346f2d3f2f40d897b7cfeb359f825b1481de17c734dd10a9d7a81f8debe69` | 9 accepted, 0 rejected |
| `warning-alias.csv` | 214 | 11 | `8c5f834459ca1b37cf1e1dd5aa6e44504e968b60ceb67b19e25fb8c429e656f1` | Byte-identical alias at a different path |
| `bom.csv` | 46 | 4 | `af93d2f508a6b957c508eab3b08f697cecc7bd576b0ad50409f19d0ee6a1fd18` | UTF-8 BOM refusal |
| `invalid-utf8.csv` | 35 | 3 | `deac94954f3c61ea2096f99fc5bfc39cbfcf7ddaf7a6de96dbd668b2e5204adb` | Invalid UTF-8 refusal |
| `invalid.csv` | 39 | 2 | `d6a7f68417e5188eb04e06014a133ddd089951f496bc4380b965e5f654a4f857` | Invalid numeric CSV refusal |
| `missing-column.csv` | 30 | 4 | `68a93ad5a5232beb3238f727b28104a19ba346d41c826bd413aa70edf748dbdf` | Missing selected concentration column |
| `too-few.csv` | 35 | 3 | `c1d55723726ead7dcc127caeba6502780292d0380ae92128ceccac6b1cddda6d` | One accepted point after rejection |
| `compatible.tsv` | 43 | 4 | `4fd97e197242f615511ab10e04ced52fb2f5dd69d306d08d0eb22a20e406c66a` | Eligible data-convert compatibility input |
| `over-limit.csv` | 16,777,217 | 0 | `1003b1b5dc078189799a1216ce0f9fbcebb94e8b6b83c58c4b03345f07f94ced` | Sparse input at limit plus one |

The warning input began with `74 69 6d 65` (`time`), had no BOM, and ended in
`0a`. It was not derived from experimental or private data.

## Actual Binary

The audited executable was:

```text
/Users/taomu/开发/.cache/deepseek-science-target/debug/deepseek-science
```

Observed properties:

- file type: Mach-O 64-bit executable arm64;
- size: 2,239,056 bytes;
- SHA-256: `54f293fec96c7caf917febf273f4f1de8a0dd8b73049b98e8de2480acd915b5f`;
- `version`: `deepseek-science 0.5.0`;
- `doctor`: version and prompt kernel version `0.5.0`, sandbox network false,
  registered tool count zero, status `ok`; and
- artifact help documented the exact four options, 16 MiB input, BOM-free
  UTF-8, case-insensitive `.json`, no overwrite, existing parent, one envelope,
  and no sidecars.

The audited producer version is truthfully `0.5.0`. Version alignment belongs
to Phase 6.5.

## End-to-End Publication and Determinism

Two invocations used the same exact absolute input argument string, column
arguments, raw source bytes, binary, and runtime platform, with fresh outputs
`artifact-a.json` and `artifact-b.JSON`.

| Property | Artifact A | Artifact B |
| --- | --- | --- |
| Exit | 0 | 0 |
| Stdout | exact `kinetics artifact complete\n` (27 bytes) | same |
| Stderr | empty | empty |
| Envelope bytes | 2,029 | 2,029 |
| SHA-256 | `ce11fdcf54e76b3b42bcaae98d6d1d5da4311f3d9e3e576ad214ae9c9d6a7b9b` | same |
| `cmp` | byte-identical | byte-identical |

A separate fresh uppercase `uppercase.JSON` invocation also produced the same
2,029 bytes and SHA-256. Each invocation produced only its requested target and
left no operation-owned temporary sibling or sidecar.

An initial audit invocation passed a relative output containing `..`; the CLI
correctly rejected it before publication under the existing path-traversal
policy. The audit then used the same external directory through its absolute
path. This was an audit-command correction, not an implementation change or
product failure.

## Envelope Byte and Schema Contract

The actual 2,029-byte warning envelope satisfied the frozen byte contract:

- UTF-8, with leading bytes `7b 0a 20 20` and no BOM;
- no CR and no trailing spaces;
- two-space pretty indentation;
- top-level field order at lines 2, 3, and 37 was `schema_version`, `artifact`,
  `payload_utf8`;
- nested fields appeared in the declaration order frozen by the RFC;
- final bytes ended with the escaped payload LF, closing quote, outer closing
  brace, and exactly one outer LF;
- size was below the 4 MiB maximum;
- `plutil -convert json -o /dev/null` and every `plutil -extract` operation
  succeeded; and
- the Rust process suite parsed the envelope with `serde_json::from_slice`.

On this Darwin environment, `plutil -lint` reported `Unexpected character { at
line 1` for the top-level JSON even though `plutil -convert json`, extraction,
and the Rust JSON parser succeeded. This is recorded as an observed tool-mode
limitation; it is not treated as evidence of invalid JSON.

The actual schema and fixed labels were:

```text
schema_version = kinetics.artifact.v1
artifact.kind = json
artifact.title = Chemistry kinetics analysis result
artifact.content.media_type = application/json
artifact.content.schema_version = kinetics.analysis.v1
artifact.content.encoding = utf-8
artifact.inputs.0.role = source_csv
```

## Exact Payload Identity

The actual `kinetics analyze --json` invocation used the exact same input-path
argument and columns as artifact A. Its stdout and the decoded `payload_utf8`
were both 933 bytes with SHA-256
`36567c97d094bc5efe7ccbef99f7f6b69d17b8ab9f9a23b53026a5ebf943653c`.
`cmp` returned success.

Both byte sequences ended in `69 73 2e 76 31 22 7d 0a`, proving preservation of
the complete payload final LF. Stderr was empty. The payload remained compact
`kinetics.analysis.v1`; no artifact metadata was inserted into it.

## Exact Source and Payload Hashes

Artifact A recorded:

| Descriptor | Byte length | Algorithm | Value |
| --- | ---: | --- | --- |
| Raw source | 214 | `blake3` | `42f1cf3ddff1eaa05bd003886712585f7d8364461e60db487749dcd32e7fcfbc` |
| Exact payload | 933 | `blake3` | `ddfc0b04b67490ede3d541f6a10ffc043052a6c9c39a76e3c420c4acc9329a43` |

Both values are 64-character lowercase hexadecimal strings. The source length
equals the raw warning CSV bytes. The payload length equals the decoded payload
bytes and includes its final LF.

No external `b3sum` executable was installed, and none was installed for this
audit. BLAKE3 correctness is instead supported by source inspection showing
`ExactByteHash::blake3` delegates to the existing `hash_bytes`, the 38 passing
artifact tests, and the 14 passing actual CLI process tests that independently
recompute both hashes from the actual source and decoded payload bytes.

The payload hash is not the legacy proposal semantic hash and is not an artifact
instance identity. The envelope contains no self-hash.

## Provenance and Review

Actual warning-envelope provenance was:

```text
workflow_id = chemistry.kinetics_csv
workflow_step = produce_analysis_result
producer_command = kinetics.artifact
producer_version = 0.5.0
```

The warning artifact mapped the existing analysis review to
`passed_with_warnings` with `finding_count = 1`. The decoded payload also
reported `passed_with_warnings` and contained exactly one finding.

The independent clean input produced `passed` with `finding_count = 0`.
Source inspection shows the adapter maps `analysis.review.status` and performs a
checked conversion of `analysis.review.findings.len()`; it does not invoke a
reviewer.

## Unregistered Identity and Metadata Exclusions

The extracted `artifact` metadata contained no:

- artifact UUID or UUID-like value;
- `ArtifactId`, `RunId`, or `ProjectId` field;
- timestamp, created/updated time, hostname, or username;
- input, output, temporary, project, database, or storage path;
- model/model-call/prompt metadata; or
- RAG, embedding, vector, retrieval, or database metadata.

Searches also found no requested output basename or `.atomic-write.tmp` in the
complete envelope. The existing caller-supplied input path appeared exactly in
the decoded `kinetics.analysis.v1` payload, as required for compatibility, and
was not copied into new artifact metadata.

The envelope is therefore an unregistered portable content envelope, not a
registered artifact instance or repository record.

## Same-Content Different-Path Behavior

`warning.csv` and `warning-alias.csv` were byte-identical: both were 214 bytes
with the same SHA-256. Their envelopes recorded the same source BLAKE3
`42f1cf3ddff1eaa05bd003886712585f7d8364461e60db487749dcd32e7fcfbc`.

The original payload was 933 bytes with BLAKE3
`ddfc0b04b67490ede3d541f6a10ffc043052a6c9c39a76e3c420c4acc9329a43`.
The alias payload was 939 bytes with BLAKE3
`e55e8260cb89d2ed3865b5ce3c1c85d308caa1be120c1cd23afbd490bf4753a0`.
The complete envelopes differed, as expected, because the existing payload
preserves the caller-supplied input path.

This is not nondeterminism. Determinism is defined for the same exact input path
argument, column arguments, source bytes, binary, and runtime platform.

## No-Overwrite and Failure Safety

Actual binary checks produced these results:

| Case | Exit | Stdout | Evidence |
| --- | ---: | --- | --- |
| Existing target | nonzero | empty | 18-byte sentinel SHA-256 `b259aa079780f083d4654cacb9be0d6250ae00bb189fe572542524bcb9245a7e` unchanged |
| Missing parent | nonzero | empty | parent and target remained absent |
| `.svg` target | nonzero | empty | stable `.json` extension error; no target |
| No extension | nonzero | empty | stable `.json` extension error; no target |
| Uppercase `.JSON` | 0 | fixed success line | fresh target published successfully |
| Lexical equality | nonzero | empty | 214-byte input and SHA-256 unchanged |
| UTF-8 BOM | nonzero | empty | BOM-specific error; no output |
| Invalid UTF-8 | nonzero | empty | byte-offset error; no output |
| Invalid CSV | nonzero | empty | numeric CSV error; no output |
| Missing selected column | nonzero | empty | concentration-column error; no output |
| Too few valid points | nonzero | empty | 1 valid versus minimum 2; no output |
| 16 MiB plus one sparse input | nonzero | empty | fixed-limit error; no output |

The existing-target attempt also preserved the warning input SHA-256. Success
and ordinary failures left no operation-owned temporary sibling. Error text did
not expose `.atomic-write.tmp`. The passing stale-sibling process test also
proved that an unowned stale sibling is preserved while the user sees only the
conservative requested-target inspection message.

## Compatibility

Actual binary compatibility checks passed:

- `kinetics analyze` text mode exited 0 and produced the established summary;
- `kinetics analyze --json` exited 0, produced 933-byte
  `kinetics.analysis.v1`, and contained no outer `artifact` or `payload_utf8`
  fields;
- `kinetics analyze --output` created one 933-byte file byte-identical to JSON
  stdout; a second write failed and preserved size and SHA-256;
- `kinetics plot` created one 11,800-byte SVG with SHA-256
  `c37e86f7f4e0947fa0a94e9a5910c56a05a31d7adcb7878419c4bf7395aa1a31`
  and no envelope sidecar;
- `data inspect` exited 0, produced a 621-byte report, and wrote no output; and
- `data convert` created one 43-byte CSV with SHA-256
  `30b3170e675e6e7fbf81e4d5ed9faf694088bed3e4b39aeadea3982727e89413`
  and no artifact sidecar.

No command created a parent directory or unexpected directory. Existing
CreateNew behavior and established help/error paths remained covered by the
passing process suites.

## Dependency and Crate Boundary

`cargo tree --workspace` and a manifest/lockfile diff from the Phase 6.0 RFC
commit through the audit baseline showed no Phase 6 dependency addition or
feature churn.

The dependency graph contains no network client, async runtime, database,
browser/UI framework, JSON canonicalization library, temporary-file framework,
logging framework, or RAG/vector/retrieval dependency. The existing workspace
`uuid` dependency remains reachable through core registered-identity contracts;
it predates Phase 6 and is not called by the unregistered envelope path.

No new crate was added. Artifacts, chemistry, CLI, storage, and core ownership
remain aligned with the RFC.

## Resource Boundaries

The fixed production boundaries are:

```text
artifact input max = 16 * 1024 * 1024 bytes
bounded reader observation = at most max + 1 byte
outer envelope max = 4 * 1024 * 1024 bytes
persistent outputs per artifact invocation = 1
```

Evidence is deliberately separated:

- **Actual end to end:** the sparse 16 MiB plus one input was refused before
  publication; the actual warning envelope was 2,029 bytes; each successful
  invocation created one requested final file.
- **Unit/process boundary:** CLI tests accept the exact input limit and reject
  limit plus one; generic serializer tests accept an exact output limit and
  reject limit minus one without returning partial bytes; process tests check
  publication and cleanup.
- **Source contract:** production constants are exactly 16 MiB and 4 MiB, the
  reader uses `take(max + 1)`, and the outer serializer reserves its final LF
  within the caller-provided inclusive limit.

No 16 MiB repository fixture, 4 MiB envelope, large write loop, streaming spool,
cache, watcher, daemon, or worker was created.

## Determinism Boundary

The audit establishes same-build, same-platform determinism for the same exact
input argument string, column arguments, source bytes, binary version, and
runtime platform.

It does not establish cross-architecture, cross-operating-system, cross-Rust,
or cross-build bit-for-bit determinism.

## Disk Safety

The audit used exactly one external root:

```text
/Users/taomu/开发/.cache/deepseek-science-target/tmp/phase6-4-artifact-audit-00ee3fb
```

Before cleanup, the frozen inventory contained exactly 66 expected files, zero
unexpected entries, zero subdirectories, and zero hidden/temp/lock/journal/cache
entries. It included all inputs, outputs, extracted payload, sentinel, and
stdout/stderr captures.

Cleanup removed exactly 66 explicitly named files with one exact-path `rm -f`
command, verified the directory empty, and removed exactly one empty directory
with `rmdir`. No glob, recursive deletion, `find -delete`, `remove_dir_all`, or
`cargo clean` was used. The external target root and its `tmp` parent were not
removed or modified manually.

No generated CSV, JSON, SVG, fixture, snapshot, golden file, database, log,
cache, or audit directory remains in the repository. Cargo wrote only to the
configured external target root. The product audit path performed no model,
tool, network, RAG, database, watcher, daemon, or background activity. The only
network access in this audit task was the explicitly required Git/GitHub
preflight (`git pull`, remote tag checks, and Release checks).

## Blocking Issues

None.

## Conclusion

Phase 6.4 audit passed. The committed Phase 6 artifact implementation is ready
for Phase 6.5 version alignment and release audit.

`v0.6.0` has not been published. The CLI package version remains `0.5.0`, no
`v0.6.0` tag was created, and no GitHub Release was created.
