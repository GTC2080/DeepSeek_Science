# Phase 3.0 Persistence Boundary RFC

## Summary

Phase 03 should add explicit, user-controlled persistence without changing the
current no-write default. The first implementation target should be:

```sh
deepseek-science kinetics analyze ... --output result.json
```

This opt-in path writes exactly one deterministic JSON result file. It should
reuse the existing stdout JSON contract, refuse overwrite by default, require
an existing parent directory, and commit the file through the storage crate's
path-safe atomic-write boundary.

Artifact manifests, run records, project workspaces, databases, model calls,
and background persistence remain later work.

## Motivation

v0.2 proves that the kinetics pipeline can produce deterministic reviewed
results as text or JSON. Users can redirect stdout, but redirection does not
express overwrite policy, artifact provenance, atomicity, or future run-record
semantics.

Phase 03 should define those responsibilities before adding filesystem side
effects. Persistence must be a visible user choice, not a hidden consequence of
analysis.

## Current v0.2 State

The current CLI:

- reads one explicitly supplied CSV file;
- parses it into an in-memory `DataTable`;
- performs deterministic kinetics analysis;
- prints text by default or one JSON object with `--json`;
- writes no output files;
- creates no artifact files, manifests, storage records, or run records;
- performs no model call, tool execution, workflow execution, UI, or
  TypeScript behavior.

Existing contracts are deliberately persistence-free:

- `KineticsAnalysisResult` contains reviewed deterministic analysis data;
- `KineticsArtifactProposal` contains path-free generic artifact metadata;
- `ArtifactManifest` is serializable metadata but does not write itself;
- `StorageRoot`, `AtomicWriteRequest`, and `AtomicWritePlan` validate and plan
  paths without touching the filesystem;
- repository traits define future save/load boundaries without a backend.

The storage path module is currently named `layout.rs`; there is no
`deepseek-science-storage/src/path.rs`.

## User Goal

A CLI user should be able to opt into saving a deterministic analysis result at
an exact path they chose, with predictable overwrite behavior and without
creating any other files, directories, logs, caches, or project state.

The same command without a persistence flag must retain the v0.2 no-write
behavior.

## Primary Phase 03 Direction

Phase 03 should focus on:

1. explicit deterministic JSON result export;
2. path validation and conservative overwrite behavior;
3. atomic filesystem commit semantics;
4. later artifact-manifest persistence;
5. later run-record design.

Real DeepSeek API integration should not be the primary Phase 03 goal. Network
calls, secrets, cost, privacy, non-determinism, and model provenance would
expand the trust boundary before local persistence is defined.

## Non-goals

- No implicit or background writes.
- No project workspace auto-initialization.
- No artifact directory tree in the first implementation.
- No persisted `ArtifactManifest` in the first implementation.
- No run record or event ledger in the first implementation.
- No database, JSONL ledger, or migration system.
- No overwrite flag in the first implementation.
- No logging framework, cache, watch mode, or daemon.
- No model calls, model-generated explanations, or tool execution.
- No UI, TypeScript, plotting, notebook, Jupyter, R, PubMed, or HPC support.
- No full CSV dialect or instrument-import expansion.

## Persistence Principles

- **No write by default:** existing text and `--json` commands remain
  filesystem-output-free.
- **Explicit intent:** persistence requires `--output <path>` or a later
  equally explicit project flag.
- **Bounded effects:** one invocation declares the maximum final files it may
  create.
- **No hidden directories:** parent directories must already exist unless a
  future command explicitly creates a project.
- **Conservative overwrite:** create-new is the default; an existing target is
  an error.
- **Single source of content:** stdout JSON and saved JSON use the same
  serializer.
- **Deterministic bytes:** identical analysis and schema produce identical
  UTF-8 JSON bytes.
- **Atomic commit:** a final target should never expose partially written JSON.
- **Layer ownership:** CLI selects behavior, chemistry computes, artifacts
  describe, and storage validates and performs writes.

## Disk Safety Rules

- No persistence flag means no filesystem write.
- `--output` writes exactly one final file.
- An atomic implementation may use one bounded sibling temporary file only
  during the opt-in write; it must not create a temp directory.
- A temporary file created by the command must be removed on failure where
  safely possible and must not remain after success.
- Never delete a pre-existing temporary-looking file that the command did not
  create.
- Do not create parent directories automatically.
- Do not overwrite an existing target in the first implementation.
- Do not create logs, caches, project metadata, artifact directories, run
  directories, or storage records as a side effect.
- Do not scan or clean unrelated directories.
- One failed output write must not modify the input CSV or an existing target.

## Proposed User Interfaces

| Option | Example | Final writes | Advantages | Costs |
| --- | --- | ---: | --- | --- |
| A: result file | `--output result.json` | 1 | Small, explicit, scriptable, no workspace | Needs exact-path and atomic-write semantics |
| B: artifact directory | `--artifact-dir artifacts/` | 1 or more | Natural future artifact grouping | Requires directory creation, naming, manifest, and collision policy |
| C: saved project run | `--project <dir> --save-run` | Several | Supports replay and durable provenance | Requires workspace layout, run IDs, manifests, and multi-file transaction policy |

Recommendation: implement Option A first. Options B and C should not block the
first Phase 03 release.

Proposed first behavior:

```sh
deepseek-science kinetics analyze \
  --input kinetics.csv \
  --time-column time_s \
  --concentration-column concentration_mol_l \
  --output result.json
```

- The saved file is deterministic JSON.
- Human-readable text remains the default stdout output.
- If `--json` is also present, stdout and the saved file contain the same JSON
  bytes.
- Output is not printed until the atomic write succeeds.
- An output failure leaves stdout empty, writes a concise error to stderr, and
  exits non-zero.
- An existing target is rejected. A future `--overwrite` option requires a
  separate review.

## Artifact Export Options

### Result JSON only: recommended first

Serialize `KineticsAnalysisResult` through the existing CLI JSON mapping and
atomically write those bytes. This is an explicit export, not yet a managed
artifact. It requires no artifact ID, manifest file, or project directory.

### Result JSON plus manifest: later

The future flow can be:

```text
KineticsAnalysisResult
  -> deterministic JSON bytes
  -> KineticsArtifactProposal
  -> ArtifactManifest
  -> explicit storage requests
```

This should write at most a documented result file and manifest file. It needs
an artifact-ID policy, a naming policy, and a multi-file failure policy first.

There is an important hash distinction to resolve: the current
`KineticsArtifactProposal.content_hash` hashes canonical analysis fields,
whereas `ArtifactManifest.content_hash` is documented as a hash of artifact
content. A manifest for a persisted JSON file should hash the exact serialized
JSON bytes, or the schema must explicitly distinguish semantic and file-content
hashes. The first `--output` implementation should not claim a manifest until
this is resolved.

### Artifact directory: deferred

`--artifact-dir` implies generated names, directory creation, multiple files,
and collision policy. It should follow, not precede, the one-file export.

## Run Record Options

| Option | Description | Recommendation |
| --- | --- | --- |
| No run record | Save only the requested result JSON | Phase 03 first implementation |
| Single run JSON | Save workflow ID, ordered steps, status, result reference, and review state | Design after artifact export stabilizes |
| JSONL event ledger | Append replay events | Defer until append durability and recovery are designed |
| Database backend | Queryable projects, runs, and artifacts | Defer until access patterns justify it |

`RunRepository` is currently an interface only. A future run record must define
schema versioning, ID generation, deterministic versus runtime fields, and the
relationship between `AgentRun`, artifacts, and persisted events. `--output`
must not silently create a run record.

## Storage Boundary

Responsibilities should remain separated:

- **CLI:** parse `--output`, retain the exact user path for errors, select
  stdout mode, and request one write after successful analysis.
- **CLI-local serializer:** produce the same deterministic JSON bytes for
  stdout and persistence. Do not add serde requirements to chemistry solely for
  CLI output.
- **Chemistry:** produce `KineticsAnalysisResult` and optional in-memory
  artifact proposal; no filesystem awareness.
- **Artifacts:** hold generic manifest, provenance, kind, review status, and
  hashes; no filesystem writes.
- **Storage:** validate targets, apply overwrite policy, plan temporary and
  final paths, and own the eventual filesystem write executor.
- **Core:** remain domain-neutral and persistence-backend-neutral.

The existing `AtomicWritePlan` is planning-only. The implementation phase must
add or identify a narrow executor before the CLI writes files; the CLI should
not duplicate atomic-write mechanics.

## Path Safety Requirements

For direct `--output <path>`:

- reject an empty path;
- reject a target without a file name;
- reject `..` traversal components in the first implementation;
- accept relative or absolute targets only when supplied explicitly by the
  user;
- do not canonicalize, expand, or rewrite the displayed path;
- require the parent directory to exist;
- write only the exact requested target and its transient sibling;
- do not follow a derived project layout or create a workspace.

The implementation can map the validated parent directory to `StorageRoot` and
the final file name to a logical path for `AtomicWriteRequest`. It must validate
the complete caller path before splitting it so parent traversal cannot be
hidden in the root.

For later `--artifact-dir` or `--project` modes, every generated child path must
pass `StorageRoot::join_logical`, which already rejects empty, absolute, root,
prefix, parent, and current-directory logical components.

Path planning alone does not protect a project root from symlink redirection.
Any later containment guarantee must define symlink behavior at the filesystem
executor boundary rather than claiming that lexical validation is sufficient.

## Atomic Write Requirements

The future writer should:

1. validate the requested path and overwrite mode before writing;
2. require the target parent to exist;
3. use `WriteMode::CreateNew` for the first implementation;
4. create the planned sibling temporary file without overwriting an existing
   temporary file;
5. write all bytes and flush/synchronize the temporary file as required by the
   durability contract;
6. atomically commit the temporary file to the final target;
7. avoid replacing a target that appeared concurrently;
8. remove only a temporary file created by this invocation when a later step
   fails;
9. return success only after the final target is committed.

The current deterministic suffix, `.atomic-write.tmp`, is a plan, not an
execution guarantee. Cross-platform create-new rename behavior and stale-temp
handling must be verified before implementation. A check-then-rename sequence
that can overwrite a concurrently created target is not acceptable.

## Output Format Policy

The first saved file should closely match the v0.2 stdout JSON contract:

- UTF-8 JSON with a trailing newline;
- `schema_version: "kinetics.analysis.v1"`;
- `command: "kinetics.analyze"`;
- `input`, `columns`, `counts`, `fits`, `comparison`, and `review` fields;
- finite JSON numbers only;
- stable lowercase machine labels;
- no timestamp, random ID, storage path, output path, environment data, or
  model-generated prose;
- cautious comparison wording based only on the finite `r_squared` MVP
  heuristic.

The CLI should use one serializer for stdout and saved bytes. A schema change
must receive a new schema version rather than silently changing the v1 file
contract.

## Error Handling

- Empty, traversal-containing, file-name-less, or missing-parent output paths
  are user/input errors.
- Existing targets are user/input errors in create-new mode.
- Temporary-file, write, sync, or commit failures produce concise stderr
  messages and a non-zero exit.
- Output failures must not print a success object or summary to stdout.
- Errors may include the path as supplied by the user but should not print
  debug dumps, backtraces, or unrelated internal paths.
- JSON mode does not add a JSON error schema in the first persistence phase.
- Internal invariant failures remain distinct from recoverable path and IO
  errors where the CLI's existing exit-code policy supports it.

## Dependency Policy

Default: no new dependencies.

Use:

- `std::fs` and `std::io` for the eventual writer;
- existing `serde_json` for deterministic JSON serialization;
- existing artifact types and `hash_bytes` when artifact persistence is added;
- existing storage path and atomic-write contracts.

Do not add a database, logging framework, async runtime, `clap`, `reqwest`, UI
framework, TypeScript, or a second serialization stack.

## Testing Plan

Future implementation tests should prove:

- default text and `--json` analysis write no files;
- `--output` writes exactly one final JSON file;
- saved JSON parses and matches the stdout schema and bytes;
- repeated identical analysis produces identical saved bytes;
- existing targets are refused and remain unchanged;
- empty, traversal, missing-parent, and file-name-less paths fail safely;
- a failed write exposes no partial final file;
- no sibling temporary file remains after success or handled failure;
- no log, cache, artifact directory, project directory, or run record appears;
- atomic planning and execution honor create-new semantics, including a
  concurrent target-creation test where practical;
- text, JSON, help, and existing failure-path smoke tests continue to pass.

Pure path and planning tests should remain in-memory. Filesystem tests should
use one bounded harness-owned test directory, create only tiny files, clean only
files they created, and add no committed fixtures unless necessary.

## Phase Breakdown

1. **Phase 3.0:** accept this RFC and freeze the no-write default.
2. **Phase 3.1:** implement and test a narrow storage atomic-write executor with
   create-new semantics; no CLI integration.
3. **Phase 3.2:** add `--output <path>` and reuse one JSON serializer for stdout
   and file bytes.
4. **Phase 3.3:** design and implement exact-byte artifact hashing and optional
   manifest persistence.
5. **Phase 3.4:** define a versioned run-record schema and explicit
   `--project ... --save-run` behavior.
6. **Phase 3.5:** audit disk safety and release readiness before any broader
   persistence surface.

## Deferred Work

- DeepSeek API integration and API-key handling.
- Model-generated explanation persistence.
- Tool execution records.
- Project workspace auto-initialization.
- Artifact-directory export and full artifact browser.
- Run/event JSONL ledger.
- Database storage and migrations.
- Background logging, caches, watchers, and daemons.
- Overwrite mode and output-file rotation.
- Multi-file transaction or recovery journal.
- UI, TypeScript, plotting, notebook, Jupyter, R, PubMed, or HPC integrations.

## Open Questions

- Should later manifests carry both a canonical semantic hash and an exact file
  content hash?
- What cross-platform primitive will guarantee create-new atomic commit without
  a check/rename race?
- Should a future `--overwrite` replace only an existing regular file, and how
  should symlinks be handled?
- Should a later run ID be caller-provided, content-derived, or generated only
  after explicit `--save-run` intent?
- Should `--output` permit all explicit absolute paths, or should a future
  policy optionally restrict them to a caller-provided project root?
- What durability level is required: file sync only, or file plus parent
  directory sync where supported?

## Recommended Next Step

Review and accept this RFC, then implement Phase 3.1 as a small storage-only
atomic-write executor contract. Do not add the CLI `--output` flag until
create-new, cleanup, cross-platform commit, and path-safety behavior have focused
tests. Once that boundary is proven, add the single-file JSON export without
manifests, run records, workspace creation, or new dependencies.
