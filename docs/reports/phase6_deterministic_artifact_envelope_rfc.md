# Phase 6.0 Deterministic Provenance-Bearing Kinetics Artifact Envelope RFC

## Summary

This RFC audits the repository at the annotated `v0.5.0` baseline and freezes
the only release mainline for `v0.6.0`:

```text
v0.6.0 = first deterministic provenance-bearing kinetics artifact envelope
```

The release adds one explicit future command:

```text
deepseek-science kinetics artifact \
  --input <path> \
  --time-column <column> \
  --concentration-column <column> \
  --output <path.json>
```

The command will reuse the existing deterministic kinetics analysis and its
only `kinetics.analysis.v1` JSON serializer, hash the raw source bytes, hash the
exact serialized payload bytes, wrap the payload and provenance in one
deterministic `kinetics.artifact.v1` JSON envelope, and publish exactly one
new file through one existing atomic `CreateNew` operation.

Phase 6 has exactly one release mainline.

It does not combine model integration, run persistence, project workspaces, new
chemistry workflows, input repair, UI, or a generic reporting framework. This
document freezes design only. It does not implement Rust, change a version,
create or move a tag, or create a GitHub Release.

RAG is not part of Phase 6 or the planned architecture established by this RFC.

## Roadmap Audit Scope

The audit started from a clean synchronized `main` at
`0a36cc300f70124d5b5a2578f770908a5d3176d9`. The annotated `v0.5.0` tag
resolves to that commit. The local and `origin` tag refs matched, and no GitHub
Release existed for `v0.5.0`.

The audit covered:

- project identity, public capabilities, workspace membership, dependency
  policy, and long-term direction in `README.md`, `Cargo.toml`, and `AGENTS.md`;
- the v0.2 roadmap and the Phase 3 persistence-boundary RFC;
- both Phase 4 import/conversion RFCs and the Phase 4 end-to-end data-path audit;
- the Phase 5 SVG RFC, end-to-end visualization audit, and v0.5 release audit;
- current artifact hash, manifest, kind, provenance, and review types;
- current kinetics analysis, artifact proposal, plot-data, and SVG contracts;
- current atomic publication and repository traits;
- current core IDs, run state, and workflow-plan contracts;
- current provider-neutral model types and DeepSeek placeholder crate; and
- current CLI routing, bounded reader, analysis JSON serializer, explicit
  output planning, and package version source.

Inspection used bounded read-only source and Git commands. No Cargo command,
script, generated artifact, fixture, snapshot, database, cache, or cleanup
operation was used for this Phase 6.0 audit.

## Current v0.5.0 Capability

The current repository establishes the following relevant facts.

1. The project is Rust-first and headless-first. It prioritizes artifacts over
   chat logs, provenance by default, cache-aware design, a domain-neutral core,
   disk-safe development, and long-term STEM-wide extensibility.
2. The public CLI already has independent `data inspect`, `data convert`,
   `kinetics analyze`, and `kinetics plot` commands. It supports deterministic
   `kinetics.analysis.v1` JSON stdout and explicit JSON output, plus one fixed
   deterministic SVG plot output.
3. `KineticsArtifactProposal` exists as path-free in-memory metadata, but it is
   not persisted.
4. `KineticsArtifactProposal.input_hashes` is currently empty because source
   input hashing has not been defined at that boundary.
5. `KineticsArtifactProposal.content_hash` is a BLAKE3 hash over a private
   canonical sequence of semantic analysis fields and IEEE-754 bit patterns.
   It is not a hash of the exact exported `kinetics.analysis.v1` JSON bytes.
6. `ArtifactManifest::new` allocates a random UUID through `ArtifactId::new()`.
   It therefore cannot be used directly in byte-identical deterministic CLI
   output.
7. `ArtifactRepository`, `RunRepository`, and `ProjectRepository` remain traits
   only. There is no durable repository backend.
8. The CLI has a proven single-target `AtomicWritePlan::execute` boundary with
   `WriteMode::CreateNew`, an existing-parent requirement, no overwrite, and
   no parent creation.
9. `kinetics plot`, `data inspect`, and `data convert` use the established
   16 MiB bounded regular-file reader. `kinetics analyze` still uses unbounded
   `fs::read_to_string`; this is a known hardening gap, not an authorization to
   change that command in Phase 6.
10. `KineticsAnalysisResult::analyze` performs the deterministic fits and
    deterministic review once. The existing analysis JSON serializer maps the
    result to finite JSON values, serializes it once, and appends one final LF.
11. `kinetics analyze --json` and explicit analysis JSON output share that
    serializer. `kinetics plot` is independent and produces no JSON sidecar.
12. The DeepSeek crate contains descriptor and mock-pricing logic only. It has
    no network client, API-key reader, or concrete provider execution.
13. The repository contains no RAG, embedding, vector database, document
    indexing, or retrieval pipeline.

These facts confirm that the missing link is not another computational or
visual surface. It is the first explicit connection from deterministic
analysis bytes to portable provenance-bearing artifact bytes.

## Historical RFC Commitments

The roadmap has repeatedly deferred artifact persistence until its byte and
identity semantics could be stated precisely.

- The v0.2 roadmap prioritized deterministic structured JSON and deferred
  artifact persistence until the output contract was stable. It also deferred
  DeepSeek integration because secrets, network behavior, costs, privacy,
  cache policy, non-determinism, and model provenance were unresolved.
- The Phase 3 RFC delivered one opt-in JSON output first. It explicitly deferred
  exact-byte artifact hashing, optional manifest persistence, artifact identity,
  run records, project workspaces, databases, JSONL ledgers, multi-file
  transactions, background persistence, caches, watchers, and daemons.
- Phase 3 also recorded the exact unresolved distinction now addressed here:
  the existing kinetics proposal hash is semantic, while a persisted payload
  hash must cover exact serialized file bytes or have a separately named
  meaning.
- Phase 4 delivered bounded read-only inspection and narrow explicit
  normalization. It intentionally refused metadata removal, unit-row removal,
  matrix extraction, semantic inference, general CSV repair, automatic
  chemistry interpretation, and implicit artifact/project/run persistence.
- Phase 5 delivered one fixed SVG rather than a generic plotting framework. It
  preserved the independent analysis JSON schema and command, used a 16 MiB
  input boundary and 4 MiB output boundary for the new plot path, and again
  excluded model calls, network activity, RAG, UI, hidden persistence, and
  multi-output behavior.

The deferred inventory relevant to future releases remains:

| Deferred item | Why it remains deferred after this RFC |
| --- | --- |
| Registered artifact identity | Instance identity depends on future project/run semantics |
| Separate manifest persistence | Two-file commit and recovery semantics are not defined |
| Run record and replay persistence | Requires stable artifact references and multi-file policy |
| Project workspace | Requires directory ownership, naming, recovery, and migration rules |
| DeepSeek/provider execution | Requires network, secret, cost, privacy, cache, and model-provenance policy |
| UI/desktop application | Must follow stable headless artifact and run contracts |
| Database/JSONL/event ledger | Access, append durability, migration, and recovery requirements are not established |
| Background persistence/cache | Would violate the current explicit bounded-effect model |
| Broader chemistry workflows | The first workflow's artifact/provenance chain should be completed first |
| Automatic laboratory-data repair | Requires explicit scientific transformation and ambiguity policy |

## Missing Architectural Link

The repository currently has all of these pieces separately:

- a deterministic kinetics result and deterministic review;
- one stable analysis JSON schema and serializer;
- generic artifact kinds, provenance records, review status, and BLAKE3 helper;
- a path-free kinetics artifact proposal;
- domain-neutral run and workflow descriptions; and
- a safe single-file atomic publication boundary.

It does not yet have one value that binds an exact source byte sequence, an
exact payload byte sequence, deterministic workflow provenance, and review
metadata into a portable single file.

This is the narrowest architectural gap between the current code and the
project principles "Artifacts over chat logs" and "Provenance by default".
Closing it before run/project persistence also gives those later systems a
concrete artifact contract to reference instead of forcing them to invent one.

## Candidate Direction Comparison

| Candidate | Value | Blocking or premature concerns | Decision |
| --- | --- | --- | --- |
| DeepSeek API integration | Adds model-generated assistance | Network, API keys, cost, privacy, cache behavior, non-determinism, failure policy, and model provenance are not established | Reject as the v0.6 mainline |
| Run record / replay persistence | Begins durable replay | Depends on stable artifact identity, artifact references, schema versioning, and multi-file persistence/recovery | Defer until the artifact contract exists |
| Project workspace | Creates a durable user container | Introduces directory creation, generated naming, multi-file state, recovery, migration, and ownership policy | Defer |
| New chemistry workflow | Expands domain coverage | Broadens the domain before the first workflow has a complete artifact/provenance chain | Defer |
| Stronger experimental-data auto-repair | Accepts more exports | Requires metadata deletion, unit-row policy, matrix selection, semantic inference, and explicit scientific transformation choices | Defer |
| UI / desktop application | Improves interactive use | Premature before stable headless artifact and run contracts; expands the toolchain and state surface | Defer |
| Generic plotting/report framework | Generalizes presentation | Phase 5's fixed SVG already satisfies the current visualization goal; there is no second real renderer/report consumer | Reject under YAGNI |
| Only bound `kinetics analyze` input | Hardens one legacy trust boundary | Important compatibility work, but too narrow to be the sole v0.6 architectural mainline | Record as a separate compatibility review |
| Artifact envelope | Completes source hash, exact payload hash, provenance, deterministic review metadata, and one-file publication | Requires careful byte/identity contracts, all addressable within existing crates and dependencies | Select as the only v0.6 mainline |

No candidates are bundled together. In particular, the envelope does not imply
a run, project, database, model call, visualization artifact, or input-repair
feature.

## Decision

Phase 6 will introduce the first deterministic provenance-bearing kinetics
artifact envelope as one additive explicit command and one versioned single-file
schema.

The release decision is:

```text
v0.6.0 = first deterministic provenance-bearing kinetics artifact envelope
```

The envelope is deterministic and unregistered. It carries exact source and
payload hashes, but no runtime instance identity. It is published once through
the existing storage boundary and changes no existing command or schema.

This decision is frozen unless implementation evidence exposes a correctness,
security, disk-safety, or compatibility contradiction. Scope convenience is not
sufficient reason to widen it.

## User Story

As a user with one supported simple numeric kinetics CSV, I can explicitly
select the exact time and concentration columns and request one new JSON
artifact envelope. The envelope contains the exact deterministic analysis JSON
that the existing `kinetics analyze --json` path would emit for the same input
and arguments, plus independently verifiable source-byte and payload-byte
hashes and deterministic workflow/review provenance.

If any read, decode, parse, analysis, serialization, invariant, size, or
publication step fails, the command reports an error and does not report
success. It never overwrites an existing target or creates a parent directory.

## Proposed CLI

The frozen command is:

```text
deepseek-science kinetics artifact \
  --input <path> \
  --time-column <column> \
  --concentration-column <column> \
  --output <path.json>
```

It accepts only:

```text
--input <path>
--time-column <column>
--concentration-column <column>
--output <path.json>
--help
-h
```

All four execution options are required and may appear exactly once. Help
succeeds only when `--help` or `-h` is the sole artifact argument, matching the
existing plot/data command pattern. Unknown options, duplicates, missing
values, positional values, and missing required options are errors.

The command has no:

```text
--json
--manifest
--manifest-output
--payload-output
--artifact-id
--project
--save-run
--force
--overwrite
--format
--model
--explain
--rag
```

It is independent of `kinetics analyze` and `kinetics plot`. It is not an
implicit sidecar mode for either command.

After and only after atomic publication succeeds, stdout is exactly:

```text
kinetics artifact complete
```

followed by one LF. Errors use stderr, leave stdout empty, and return nonzero.
No JSON error schema is introduced.

## Why a Single-File Envelope

A payload JSON plus a separate manifest JSON would create two final files.
Although each file could be atomically published independently, the repository
has no multi-file transaction. A crash between two atomic writes could leave a
payload without its manifest or a manifest without its payload.

Adding a directory transaction, journal, rollback protocol, lock, or recovery
index solely to publish the first manifest would substantially enlarge the
release and precede evidence for those mechanisms.

The v1 envelope therefore embeds the exact payload text and all provenance in
one JSON file and uses one `AtomicWritePlan::execute`. It does not write:

- result JSON plus manifest JSON;
- JSON plus SVG;
- an artifact directory;
- a project directory;
- a run record;
- a JSON sidecar;
- a lock, journal, index, or cache.

The single-file decision is an atomicity boundary, not a claim that future
registered projects can never use multiple files. Future multi-file state must
define its own transaction and recovery contract before implementation.

## Unregistered Artifact Identity

The v1 value is an **unregistered deterministic artifact envelope**.

It contains no:

- random UUID;
- runtime-generated `ArtifactId`;
- `RunId`;
- `ProjectId`; or
- timestamp.

The implementation must not call `ArtifactId::new()` or
`ArtifactManifest::new()` to fill an identity field. It must not invent a
content-addressed UUID. Whether equal content represents one logical artifact
or multiple artifact instances depends on future project/run registration
semantics.

The identity distinction is:

- **unregistered artifact envelope:** deterministic, portable bytes that have
  not entered a project/run repository and have no instance identity;
- **registered `ArtifactManifest`:** future repository metadata whose instance
  identity is assigned only by an explicit project/run persistence operation.

The envelope's exact payload hash is a content verifier, not an instance ID.
Existing `ArtifactManifest`, `ArtifactRef`, and `ArtifactRepository` remain in
place and are not refactored or removed by this RFC.

## Input Hash Contract

The source input descriptor hashes exactly the bytes returned by the bounded
reader:

```text
BLAKE3(raw input bytes)
```

Hashing occurs before UTF-8 decoding, BOM policy checks, CSV parsing, column
selection, row rejection, or kinetics analysis.

The descriptor records:

- `algorithm`: exactly `blake3`;
- `value`: exactly 64 lowercase hexadecimal ASCII characters;
- `byte_length`: the raw bounded byte length as a non-negative JSON integer;
- `role`: exactly `source_csv`.

There is exactly one input descriptor in v1. The hash does not cover:

- the input path string;
- a canonicalized or absolute path;
- normalized text;
- a decoded `String`;
- a `DataTable`;
- only the selected columns; or
- data remaining after rejected rows are removed.

The path selects the file to open but is not part of source content identity.
The implementation must compute the hash from the already bounded in-memory
byte vector and must not reopen or reread the input.

## Payload Hash Contract

The payload is produced by invoking the existing and only kinetics analysis
JSON serializer exactly once for the already computed analysis. That serializer
produces the complete `kinetics.analysis.v1` UTF-8 bytes and appends its current
final LF.

The payload content descriptor hashes:

```text
BLAKE3(exact payload UTF-8 bytes, including the final LF)
```

It records:

- `algorithm`: exactly `blake3`;
- `value`: exactly 64 lowercase hexadecimal ASCII characters;
- `byte_length`: the complete payload byte length including its final LF.

This hash is intentionally different from the existing
`KineticsArtifactProposal.content_hash`:

- the proposal hash covers canonical semantic analysis fields and numeric bit
  patterns;
- the Phase 6 content hash covers exact serialized payload bytes, including
  JSON keys, string escaping, number spelling, ordering, and the final LF.

The two meanings must not share one ambiguous Rust type or field contract.
Implementation should introduce a clearly named exact-byte hash descriptor (or
equally explicit field type) rather than silently changing the existing
proposal hash semantics. The semantic proposal hash is not emitted in the v1
envelope.

The envelope does not embed a hash of its own complete bytes. Such a field
would be self-referential. A caller may hash the finished envelope externally
if transport-level integrity is needed.

## Envelope Schema

The frozen schema version is:

```text
kinetics.artifact.v1
```

The conceptual schema and normative field order are:

```json
{
  "schema_version": "kinetics.artifact.v1",
  "artifact": {
    "kind": "json",
    "title": "Chemistry kinetics analysis result",
    "content": {
      "media_type": "application/json",
      "schema_version": "kinetics.analysis.v1",
      "encoding": "utf-8",
      "byte_length": 0,
      "hash": {
        "algorithm": "blake3",
        "value": "lowercase-64-character-hex"
      }
    },
    "inputs": [
      {
        "role": "source_csv",
        "byte_length": 0,
        "hash": {
          "algorithm": "blake3",
          "value": "lowercase-64-character-hex"
        }
      }
    ],
    "provenance": {
      "workflow_id": "chemistry.kinetics_csv",
      "workflow_step": "produce_analysis_result",
      "producer_command": "kinetics.artifact",
      "producer_version": "0.6.0"
    },
    "review": {
      "status": "passed",
      "finding_count": 0
    }
  },
  "payload_utf8": "{\"command\":\"kinetics.analyze\",...}\n"
}
```

The example byte lengths and hashes are placeholders. Actual output must carry
the computed values.

Field semantics are frozen as follows:

- `kind` is the lowercase stable machine label `json`, corresponding to the
  existing generic JSON artifact kind without serializing the Rust enum name.
- `title` is exactly `Chemistry kinetics analysis result`.
- `content.media_type` is exactly `application/json`.
- `content.schema_version` is exactly `kinetics.analysis.v1`.
- `content.encoding` is exactly `utf-8`.
- `inputs` contains exactly one `source_csv` descriptor.
- `workflow_id` comes from the existing
  `CHEMISTRY_KINETICS_CSV_WORKFLOW_ID` contract.
- `workflow_step` is exactly `produce_analysis_result`, the existing artifact
  production step.
- `producer_command` is exactly the stable machine label
  `kinetics.artifact`.
- `producer_version` comes from `env!("CARGO_PKG_VERSION")` evaluated in
  `deepseek-science-cli`; no second version constant may be maintained. The
  released v0.6 binary must therefore emit `0.6.0`, while pre-alignment
  implementation builds truthfully emit their current CLI package version.
- `review.status` maps the existing deterministic kinetics review without
  rerunning it: `passed`, `passed_with_warnings`, or `failed`.
- `review.finding_count` is exactly the existing findings vector length.
- `payload_utf8`, after JSON string decoding, is the complete exact payload
  text including its one final LF.

No additional optional fields are permitted in `kinetics.artifact.v1`.

## Deterministic Serialization

The outer envelope byte contract is:

- UTF-8 without a BOM;
- LF line endings only;
- exactly one trailing LF after the outer closing brace;
- no trailing spaces;
- two ASCII spaces per indentation level;
- one object member or array element per formatted line;
- stable field order exactly as listed in the schema above;
- finite JSON numbers only;
- byte lengths rendered as base-10 JSON integers; and
- stable lowercase machine labels.

Nested field order is also normative:

1. top level: `schema_version`, `artifact`, `payload_utf8`;
2. `artifact`: `kind`, `title`, `content`, `inputs`, `provenance`, `review`;
3. `content`: `media_type`, `schema_version`, `encoding`, `byte_length`, `hash`;
4. each input: `role`, `byte_length`, `hash`;
5. each hash: `algorithm`, `value`;
6. `provenance`: `workflow_id`, `workflow_step`, `producer_command`,
   `producer_version`;
7. `review`: `status`, `finding_count`.

Implementation should use ordered typed structs and the existing `serde_json`
dependency's pretty serializer, then append one LF. It must not construct the
outer envelope through an unordered map, add `preserve_order`, add JSON
Canonicalization Scheme, hand-write a general JSON serializer, or add a new
dependency.

The current analysis serializer uses compact JSON plus one final LF. Phase 6
must not pretty-print, parse-and-reserialize, reorder, normalize, or reconstruct
that payload. The exact compact payload is stored as the decoded value of
`payload_utf8`; the outer JSON serializer only performs the required JSON string
escaping. Therefore:

```text
UTF-8 bytes(JSON-decode(envelope.payload_utf8))
  == stdout bytes(kinetics analyze ... --json)
```

for the same exact input path argument, column arguments, source bytes, and
binary version.

The envelope metadata must not add the output path, temporary path, current
directory, canonicalized path, timestamp, UUID, platform, hostname, username,
Rust toolchain path, or build directory. It must not add model-generated text,
RAG metadata, or retrieval metadata.

The existing `kinetics.analysis.v1` payload includes the caller-supplied input
path string. Preserving it is required for byte identity and compatibility. The
artifact command must pass through exactly that caller input argument and must
not discover or inject an environment-derived replacement. Removing or
redacting the existing payload field would require a separately versioned
analysis-schema compatibility decision and is not part of Phase 6.

## Execution Flow

The future command has one linear flow:

1. identify exact help or parse the four required artifact options;
2. reject lexical equality between input and output paths;
3. require an output extension equal to `.json` under ASCII
   case-insensitive comparison;
4. open one explicit regular input file;
5. bounded-read at most `16 * 1024 * 1024` bytes plus one detection byte;
6. compute BLAKE3 over the returned raw input bytes and record their length;
7. decode those same bytes as strict UTF-8;
8. call the existing simple numeric CSV parser once;
9. construct exact `KineticsColumns`;
10. construct `ValidatedKineticsInput` once;
11. call `KineticsAnalysisResult::analyze` once;
12. call the existing analysis JSON serializer once;
13. compute BLAKE3 over the exact payload UTF-8 bytes and record their length;
14. map the existing analysis/review result to envelope metadata;
15. render one complete envelope in memory;
16. validate outer UTF-8, BOM/LF/trailing-LF rules, stable schema, exact field
    invariants, both byte lengths, both hashes, and the fixed size maximum;
17. create one `AtomicWriteRequest` for the explicit target;
18. use `WriteMode::CreateNew`;
19. call `AtomicWritePlan::execute` exactly once with the completed envelope
    bytes; and
20. emit the fixed success line only after publication succeeds.

The flow must not:

- reopen or reread the source;
- parse the CSV twice;
- rerun kinetics fitting;
- rerun the reviewer;
- call a model or tool;
- access the network;
- write a second file;
- create a parent directory;
- overwrite a target;
- create a workspace; or
- write a run record.

## Resource Bounds

The frozen fixed non-configurable limits are:

- raw input: `16 * 1024 * 1024` bytes;
- bounded reader observation: at most the input limit plus one byte;
- final outer envelope: `4 * 1024 * 1024` bytes;
- persistent final outputs per invocation: at most one.

The 16 MiB source limit reuses the established inspect/convert/plot trust
boundary. Metadata length is checked before allocation when available, and a
limit-plus-one bounded read remains required to handle concurrent size changes
or unreliable metadata safely.

The 4 MiB envelope limit is intentionally conservative and consistent with the
already accepted Phase 5 in-memory output ceiling. The current analysis JSON is
a bounded summary rather than a row dump; audit evidence produced a payload
under 1 KiB, and the review has a fixed finite check surface. The larger 4 MiB
ceiling leaves substantial room for caller-supplied path/column strings while
preventing unbounded serialization. There is no repository evidence requiring
a larger artifact envelope.

Rendering must use checked growth or a limit-aware writer so it does not build
an arbitrarily larger `String` merely to discover the final limit violation. A
final exact byte-length check is mandatory before output planning.

There is no streaming envelope writer, memory mapping, spool file, cache,
worker, watcher, daemon, or background task.

## Disk Safety

The command is an explicit one-file write surface. Before publication, all
source reading, parsing, analysis, payload serialization, hashing, envelope
rendering, and invariant checks occur in memory.

Publication rules are:

- the output parent must already exist and be a directory;
- the final target must not exist;
- the command uses the existing deterministic operation-owned temporary sibling;
- the temporary sibling is create-new and bounded by the envelope maximum;
- the final target is atomically published without replacement;
- no non-atomic fallback exists;
- no success line is emitted before publication completes;
- the command never deletes an existing target as rollback;
- an unexpected publication error conservatively tells the user the requested
  target may exist and should be inspected before retrying; and
- success and ordinary failure leave no operation-owned temporary sibling.

The command creates no parent, project tree, artifact directory, run directory,
manifest sidecar, cache, database, log, journal, lock, index, or background
state. It never scans or cleans unrelated directories.

## Crate Ownership

### `deepseek-science-artifacts`

Owns:

- a domain-neutral exact hash descriptor;
- domain-neutral unregistered artifact envelope metadata;
- payload and input descriptors;
- deterministic envelope invariants and serialization contract.

It performs no file IO, knows no chemistry schema, and allocates no random ID
for an unregistered envelope.

### `deepseek-science-chemistry`

Owns:

- `chemistry.kinetics_csv` workflow identity;
- `produce_analysis_result` workflow-step identity;
- mapping existing kinetics review status/count into artifact review metadata;
- the chemistry-specific title, kind selection, and domain provenance adapter.

It receives no path, reads no file, writes no file, serializes no outer target,
and generates no random identity. It does not rerun analysis or review in the
adapter.

### `deepseek-science-cli`

Owns:

- manual artifact argument parsing and help;
- lexical input/output inequality and `.json` suffix validation;
- bounded regular-file reading;
- raw input byte hashing;
- existing CSV/column/analysis orchestration;
- invoking the existing analysis JSON serializer once;
- exact payload byte hashing;
- supplying `kinetics.artifact` and the CLI package version from
  `env!("CARGO_PKG_VERSION")` as producer provenance;
- envelope orchestration and final invariant checks;
- output-path-specific errors and the fixed success line.

### `deepseek-science-storage`

Owns:

- opaque envelope-byte publication through one create-new atomic plan.

It does not interpret JSON, kinetics, schemas, hashes, review, or provenance.

### `deepseek-science-core`

Remains domain-neutral and unchanged. Phase 6 does not force an `AgentRun`,
`RunId`, `ProjectId`, or workflow execution runtime into envelope generation.

This ownership uses existing crates and real boundaries. It creates no new
crate, service layer, repository implementation, registry, factory, or generic
workflow executor.

## Compatibility

The Phase 6 command is additive. These interfaces and schemas remain unchanged:

```text
kinetics analyze
kinetics analyze --json
kinetics analyze --output
kinetics plot
data inspect
data convert
kinetics.analysis.v1
```

Compatibility rules are:

- `kinetics analyze` never implicitly creates an artifact;
- `kinetics plot` never implicitly creates a manifest or envelope;
- `kinetics artifact` does not modify `kinetics.analysis.v1`;
- one existing serializer continues to serve analyze JSON stdout, analyze JSON
  output files, and the artifact payload;
- artifact metadata is not inserted into `kinetics.analysis.v1`;
- existing fit equations, exact columns, finite-r-squared heuristic,
  deterministic review, wording, and row-rejection behavior do not change;
- existing inspect/convert/plot bounds and publication behavior do not change;
- existing `ArtifactManifest` and repository traits are not redefined; and
- no version changes until the separately scoped Phase 6.5 alignment.

The unbounded `fs::read_to_string` in current `kinetics analyze` is a known
hardening gap. The new artifact command uses the 16 MiB bounded reader, but
Phase 6 must not silently change the old analyze command. Any future alignment
of analyze with the bounded reader is a separate explicit compatibility change
with updated help, README, and over-limit tests.

## Error Handling

All errors leave stdout empty and write a concise message to stderr. The command
distinguishes at least:

- invalid, duplicate, missing, or unknown arguments;
- lexical input/output equality;
- missing or invalid `.json` output extension;
- input open, metadata, non-regular-file, read, and over-limit failures;
- BOM or strict UTF-8 failures under the existing simple-CSV policy;
- simple numeric CSV and exact-column failures;
- kinetics validation, fit, and review-pipeline failures;
- non-finite JSON serialization values;
- payload/envelope serialization and invariant failures;
- envelope size overflow;
- existing output target;
- missing or invalid output parent; and
- other atomic publication failures.

Input/output equality is lexical only, matching the current plot/convert
boundary. This RFC does not claim canonical, symlink, inode, or alias equality.
The storage boundary still prevents replacement of an existing target.

Pre-publication failures execute no output plan. On an unexpected execution
failure, the CLI must not claim rollback or delete a possibly published final
target. It must advise inspection before retrying without exposing the
operation-owned temporary path.

## Dependency Policy

Phase 6 requires no new dependency:

- existing `blake3` and `hash_bytes` cover both exact byte hashes;
- existing `serde` and `serde_json` cover typed envelope serialization and
  validation;
- the standard library covers bounded IO and path/suffix checks;
- existing chemistry covers analysis and review; and
- existing storage covers atomic `CreateNew` publication.

Do not add a JSON canonicalization library, new hash library, UUID scheme,
database, async runtime, network client, CLI framework, temporary-file crate,
logging framework, RAG/retrieval dependency, UI dependency, or new workspace
crate.

No dependency feature should be enabled solely to preserve insertion order.
Typed struct field order is the frozen serialization boundary.

## Testing Plan

Later implementation must prove at least:

1. two identical invocations using different fresh output paths produce
   byte-identical envelope files;
2. the envelope is valid JSON;
3. top-level `schema_version` is exactly `kinetics.artifact.v1`;
4. decoded `payload_utf8` bytes are byte-identical to the complete stdout bytes
   from `kinetics analyze ... --json` for the same exact arguments;
5. the payload BLAKE3 value can be recomputed exactly from decoded
   `payload_utf8` bytes;
6. the input BLAKE3 value can be recomputed exactly from the original raw input
   bytes;
7. the payload byte length includes the final LF and is exact;
8. the source input byte length is exact;
9. review status and finding count equal the already computed analysis review;
10. the envelope adds no timestamp, UUID, output path, temporary path,
    platform, hostname, username, Rust/build path, model output, or
    environment-derived path;
11. the production input constant is exactly `16 * 1024 * 1024`, generic
    bounded-reader behavior accepts exact limit and rejects limit plus one, and
    the CLI maps the over-limit path without publication;
12. the `4 * 1024 * 1024` envelope boundary is enforced before publication;
13. an existing target is rejected and its bytes remain unchanged;
14. a missing parent is not created;
15. lexical input/output equality is rejected;
16. a target without a case-insensitive `.json` extension is rejected;
17. success and failure leave no operation-owned temporary sibling;
18. each invocation creates at most one persistent output;
19. `kinetics analyze` text/JSON/output, `kinetics plot`, `data inspect`, and
    `data convert` compatibility remains intact; and
20. no network, provider, model, tool, RAG, retrieval, database, workspace, run
    record, cache, log, watcher, daemon, worker, or background activity occurs.

Serialization tests must also assert the complete normative field order,
two-space indentation, LF-only bytes, no BOM, no trailing spaces, lowercase
labels, finite numbers, exactly one outer trailing LF, and preservation of the
payload's own decoded final LF.

Resource-boundary tests must use private small-limit seams and in-memory readers
where possible. They must assert the production constants separately rather
than create 16 MiB or 4 MiB repository fixtures merely for confidence.

Filesystem/process tests must:

- resolve and use Cargo's external target temporary root only;
- create only small, explicit, operation-owned files;
- record every created path;
- remove only exact files they own;
- remove an owned empty directory only with non-recursive removal;
- never use glob deletion, recursive deletion, `remove_dir_all`, or
  `cargo clean`; and
- leave no generated fixture, snapshot, golden file, example envelope, cache,
  database, or repository asset.

The smallest relevant crate/unit/process checks should run once per
implementation phase. Phase 6.4 owns the final compatibility and disk-safety
audit rather than repeating full validation in every phase.

## Phase Breakdown

### Phase 6.0: Roadmap audit and RFC freeze

- audit current capability and historical commitments;
- select and freeze exactly one release mainline;
- create this RFC only;
- implement no Rust and make no version, tag, or Release change.

### Phase 6.1: Pure domain-neutral envelope contract

- implement the in-memory unregistered artifact envelope, exact hash
  descriptor, typed fields, deterministic serialization, and pure invariants;
- add no CLI, file IO, chemistry adapter, version, tag, or release change.

### Phase 6.2: Deterministic kinetics adapter

- map existing kinetics analysis/review and workflow constants into envelope
  metadata and exact payload inputs;
- add no CLI publication and rerun no independent reviewer.

### Phase 6.3: `kinetics artifact` CLI

- add only the frozen manual CLI surface;
- reuse one bounded read, one analysis, one existing payload serialization, two
  exact byte hashes, one in-memory envelope, and one atomic `CreateNew`
  publication.

### Phase 6.4: End-to-end artifact audit

- audit byte determinism, exact input/payload hash correctness, schema and
  payload compatibility, resource limits, no-overwrite behavior, temporary
  cleanup, crate boundaries, dependencies, and disk safety;
- create no committed example, fixture, snapshot, or golden artifact.

### Phase 6.5: Version alignment and release audit

- align the CLI package and its matching lock entry to `0.6.0` only after all
  implementation and audit checks pass;
- verify `producer_version` derives from `env!("CARGO_PKG_VERSION")`;
- perform the final v0.6 release audit;
- do not create a tag or GitHub Release.

An annotated `v0.6.0` tag requires a separate explicitly authorized task.
A GitHub Release must not be created automatically.

## Deferred Work

Phase 6 explicitly excludes:

- RAG, embeddings, vector databases, vector stores, document indexing,
  automatic document retrieval, and retrieval pipelines;
- DeepSeek or any other model API, API-key reading, model-generated
  explanation, model execution, or model provenance;
- tool execution;
- run persistence, replay persistence, event ledgers, and JSONL append;
- project workspaces and project auto-initialization;
- database backends and migrations;
- background logging, cache directories, watchers, daemons, and workers;
- overwrite, force, replacement, and output rotation;
- multi-file transactions, recovery journals, locks, and indexes;
- separate manifest/payload files and artifact directories;
- SVG artifact envelopes, JSON plus SVG output, PNG, PDF, HTML, and automatic
  reports;
- UI, TypeScript, Node, Tauri, Electron, GPUI, browser, and desktop work;
- a generic plotting, reporting, artifact registry, plugin, or service
  framework;
- new chemistry workflows;
- automatic column, unit, chemistry, or reaction-role inference;
- metadata or unit-row removal, matrix extraction, whitespace repair, and
  automatic laboratory-data repair;
- full CSV/RFC 4180, quoted or multiline fields, arbitrary delimiters, locale
  inference, and proprietary instrument formats;
- changing the existing `kinetics analyze` input limit without separate review;
  and
- Jupyter, R, PubMed, and HPC integration.

These exclusions are release boundaries, not placeholders for hidden work.
None may be introduced as an implementation convenience for the envelope.

## Open Questions Resolved

### Is Phase 6 one feature or a bundle?

One feature. Phase 6 has exactly one release mainline: the first deterministic
provenance-bearing kinetics artifact envelope.

### Is the envelope a registered artifact?

No. It is unregistered and has no instance identity. Registration waits for
explicit project/run persistence semantics.

### Should a content-addressed UUID be invented now?

No. The exact payload hash verifies content; it does not decide whether equal
content represents the same future artifact instance.

### Are the proposal semantic hash and payload hash the same?

No. The existing proposal hash remains semantic/canonical. The v1 content
descriptor hashes exact serialized payload bytes and receives a separately
named contract.

### Should the envelope hash itself?

No. The envelope does not contain a self-referential hash.

### Should payload JSON be nested as a parsed object?

No. `payload_utf8` stores the exact decoded text so the payload hash can be
recomputed without relying on object reserialization, whitespace, or key order.

### Should the outer envelope use JSON Canonicalization Scheme?

No. Typed ordered structs, frozen two-space formatting, the existing
`serde_json` dependency, and explicit byte-level tests are sufficient.

### Should the output be split into payload and manifest files?

No. One file avoids a partial two-file state without inventing a transaction
system.

### What is the envelope size limit?

`4 * 1024 * 1024` bytes. It is consistent with the existing bounded in-memory
SVG output policy and far above current fixed-summary analysis payload evidence.

### Where does the producer version come from?

Only `env!("CARGO_PKG_VERSION")`. No hard-coded duplicate version constant is
permitted.

### Does review run again for artifact metadata?

No. Status and finding count map from the one existing analysis result.

### Does Phase 6 change `kinetics analyze` input behavior?

No. Its unbounded read is recorded as a hardening gap for separate compatibility
review. The new artifact command is bounded from its first version.

### How are existing caller-supplied input paths handled?

The exact input argument remains in the existing `kinetics.analysis.v1`
payload because byte identity is mandatory. It is not hashed as source content,
canonicalized, expanded, or copied into new envelope metadata. Output and
temporary paths are never serialized.

### Is RAG a deferred Phase 6 subphase?

No. RAG is not part of Phase 6 or the planned architecture established by this RFC.

## Recommended Next Step

Phase 6.1: implement the pure in-memory unregistered artifact envelope contract.
