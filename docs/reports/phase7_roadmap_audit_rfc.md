# Phase 7.0 Read-Only Kinetics Artifact Verification RFC

## Summary

The formal release baseline for this roadmap audit is annotated tag `v0.6.0`.
Its tag message is `v0.6.0`, its tag object is
`45f4200fb62f732d9ab7cf50c5cd987a0ba96dc2`, and it resolves to commit
`4c17e9c875731f2a381f3a8d0c78a4fa2bf14581` with subject
`chore: align v0.6 version and add release audit`.

The only release mainline selected for the next version is:

```text
v0.7.0 = read-only verification of kinetics.artifact.v1 envelopes
```

The user value is one bounded command that can read an existing transported
kinetics artifact envelope, validate its exact schema and internal integrity,
optionally compare the original source bytes, and report precisely what was and
was not verified without changing any file.

Phase 7 has exactly one release mainline.

The selected capability is verification only. Its successful text report also
exposes the already verified metadata, so Phase 7 does not add a second
`inspect` command. It does not import, register, repair, rewrite, recompute, or
authenticate the artifact.

This Phase 7.0 task is docs-only. It performs the roadmap audit and freezes this
RFC only. It implements no Rust, changes no CLI behavior, changes no dependency
or version, and creates or moves no tag or GitHub Release. The existing
`v0.6.0` tag has no GitHub Release, and no local or remote `v0.7.0` tag or
GitHub Release exists at the audit baseline.

RAG is not part of Phase 7 or the planned architecture established by this RFC.

## Audit Scope

### Baseline and identity checks

The audit began only after the required preflight established all of the
following:

- GitHub CLI account: `GTC2080`;
- Git author name: `GTC2080`;
- Git author email: `140309575+GTC2080@users.noreply.github.com`;
- branch: `main`;
- worktree: clean;
- `HEAD == origin/main`;
- exact starting commit:
  `4c17e9c875731f2a381f3a8d0c78a4fa2bf14581`;
- `v0.6.0` is an annotated tag at tag object
  `45f4200fb62f732d9ab7cf50c5cd987a0ba96dc2`;
- local and remote `v0.6.0` tag objects and peeled commits match;
- `v0.6.0^{}` resolves to the starting commit;
- `v0.5.0` remains tag object
  `0ea30cdf1565a459f18260ad0e6603561251e2ce` and resolves to
  `0a36cc300f70124d5b5a2578f770908a5d3176d9`;
- local and remote `v0.7.0` refs are absent; and
- GitHub Releases for `v0.6.0` and `v0.7.0` are absent.

No repository state was repaired, reset, restored, stashed, rebased, merged,
or cherry-picked.

### Documents inspected

The audit completely read the current:

- `README.md`;
- `AGENTS.md`;
- root `Cargo.toml`;
- `Cargo.lock`;
- `docs/reports/phase3_persistence_boundary_rfc.md`;
- `docs/reports/phase6_deterministic_artifact_envelope_rfc.md`;
- `docs/reports/phase6_kinetics_artifact_audit.md`; and
- `docs/reports/phase6_v0_6_release_audit.md`.

The audit also checked the Phase 1 foundation and kernel-integration audits,
the v0.2 roadmap and Phase 2 artifact-mapping design, the Phase 4 laboratory
inspection/conversion RFCs and data-path/release audits, and the Phase 5 SVG
visualization RFC and visualization/release audits. Historical statements were
treated as historical evidence, not as substitutes for current code.

### Code inspected

The audit inspected the requested current implementation and relevant tests in:

- all listed `deepseek-science-core` ID, project, thread, run, event,
  inspection, workflow, and error modules;
- all listed `deepseek-science-artifacts` envelope, manifest, hash, kind,
  review, and error modules;
- all listed `deepseek-science-storage` layout, repository, atomic-write, and
  error modules;
- current kinetics analysis and artifact mapping;
- CLI routing, parsing, bounded reads, serializers, artifact publication,
  private postcondition validation, and process tests;
- provider-neutral model types and the DeepSeek placeholder crate;
- tool definitions, calls, results, permissions, and in-memory registry; and
- sandbox policy and future runner interface.

### Command discipline

The inspection used bounded read-only `git`, `gh`, `rg`, `sed`, `nl`, `head`,
`tail`, `wc`, and `cat` command classes. The only network activity was the
required Git/GitHub preflight, fast-forward pull, remote tag inspection, and
Release inspection.

No Cargo command was run. No project script, Python script, Node command,
database tool, model call, tool execution, build system, watcher, daemon, or
background loop was run.

## Current v0.6.0 Capability

### Capability matrix

| Capability | Current state | Durable? | User-visible? | Deterministic? |
| --- | --- | --- | --- | --- |
| Data inspection | Implemented bounded read-only `data inspect` report | No | Yes | Yes for the same input snapshot and binary |
| Data conversion | Implemented explicit one-file `data convert` | Yes, as one standalone CSV | Yes | Yes under the frozen input/output contract |
| Kinetics analysis | Implemented direct in-memory CLI pipeline | Not as run state; optional result export is separate | Yes | Yes for the same invocation boundary |
| Analysis JSON export | Implemented `kinetics.analysis.v1` stdout and explicit `--output` | Yes only when `--output` is requested | Yes | Yes; it includes the caller-supplied input path |
| SVG visualization | Implemented explicit one-file `kinetics plot` | Yes, as one standalone SVG | Yes | Yes under the same-build boundary |
| Artifact envelope creation | Implemented explicit one-file `kinetics artifact` | Yes, as one standalone unregistered envelope | Yes | Yes for the same complete invocation, not source bytes alone |
| Artifact verification/import | No read/verify/import product API or CLI | No | No | N/A |
| Registered artifact repository | Manifest/ref types and repository trait only; no backend | No | No | N/A |
| Project workspace | Core type and pure computed layout only; no init/load backend | No | No | No durable byte contract; generated IDs are random |
| Run record | In-memory run/skeleton types only; no versioned record | No | No | Only selected in-memory operations are deterministic |
| Event ledger/replay persistence | Event types and in-memory projection only | No | No | In-memory projection is deterministic for a valid supplied slice |
| Model execution | Provider-neutral types/trait and DeepSeek placeholders only | No | No | N/A |
| Tool execution | Metadata, calls/results, permissions, and registry only | No | No | N/A |
| UI | Not implemented | No | No | N/A |

### Core reality

All five core ID newtypes generate UUID v4 values through `new()` and can also
be reconstructed from caller-provided UUID values through `from_uuid`.
`Project::new`, `Thread::new`, `AgentRun::new`, and ordinary `RunStep::new`
generate random IDs. `AgentRun::prepare_from_plan` accepts a caller-provided
`RunId` and deterministically derives planned step IDs from that run ID and
step order, but it creates only an in-memory `Created` skeleton.

The run skeleton does not execute a workflow, call a model, call a tool, read
or write files, persist state, or emit events. `WorkflowPlan` is likewise only
an ordered in-memory description.

`EventSequence` defines zero-based consecutive ordering, but the comments
assign real sequence allocation to a future ledger. A caller can currently
construct `CoreEventEnvelope` values directly. The event model has no schema
version, event ID, timestamp, durable locator, or complete step/artifact
payload. `ArtifactRecorded` does not carry a run ID, and `StepRecorded` does
not carry a step body or state.

`RunInspection::from_events` validates a caller-supplied in-memory slice. It
checks sequence order, the leading `RunCreated`, run identity, and lifecycle
transitions, then returns counts and final state. It does not load an event
ledger, reconstruct steps or artifacts, or persist replay output.

The real kinetics CLI path does not use `AgentRun`, `CoreEvent`, or
`RunInspection`. It directly reads, parses, validates, analyzes, serializes,
and optionally publishes. Writing a run record from the current path would
therefore require a truthful execution integration first; serializing a
workflow skeleton would not prove that those steps executed.

### Artifact reality

`UnregisteredArtifactEnvelope` is the current portable artifact contract. It
contains:

- fixed outer schema metadata;
- an exact UTF-8 payload string;
- exact payload byte length and BLAKE3;
- exact raw-source byte length and BLAKE3 declaration;
- workflow/producer provenance; and
- deterministic review status and finding count.

It contains no `ArtifactId`, `RunId`, `ProjectId`, UUID, timestamp, registered
reference, storage path, or repository record. Its exact hashes verify byte
content; they are not semantic hashes or instance identities.

The current public envelope API is write-side only. It constructs validated
in-memory values and serializes deterministic pretty JSON within a caller
limit. The module imports `serde::Serialize` only; it exposes no public bounded
deserialization, load, verify, inspect, or import API.

The CLI has a private `validate_kinetics_artifact_boundary` postcondition, but
it validates bytes and values that the same invocation just created. It is not
a reader for an existing transported artifact.

`ArtifactManifest` is a separate in-memory metadata contract.
`ArtifactManifest::new` allocates a random `ArtifactId`; its content hash is
only checked for non-emptiness, and the manifest has no schema version,
payload locator, byte-length contract, migration contract, or durable backend.
`KineticsArtifactProposal` carries a canonical semantic analysis hash, not the
envelope's exact payload-byte hash. An unregistered envelope must not be
described as a registered `ArtifactManifest`.

### Storage reality

`ProjectRepository`, `RunRepository`, and `ArtifactRepository` are traits only.
There is no implementation of any of them.

`ProjectLayout` computes a broad future tree under
`projects/<ProjectId>/...`, including project metadata, raw/derived files,
runs, artifact-kind directories, and model/tool/event log directories. It
creates none of those paths.

`StorageRoot::join_logical` provides lexical relative-path validation. It does
not canonicalize, inspect symlinks, prove filesystem containment, create a
root, or lock a directory.

`AtomicWritePlan::execute` is a real, narrow single-file writer. In
`CreateNew` mode it requires an existing parent, opens one deterministic
temporary sibling with create-new semantics, writes and file-syncs it,
publishes with a hard link that refuses replacement, and removes the temporary
sibling. `ReplaceExisting` is declared but execution rejects it.

There is no directory-creation transaction, multi-file transaction, journal,
recovery protocol, lock, parent-directory sync contract, registered repository
backend, or project-root symlink containment guarantee.

### Chemistry and CLI reality

The current user-visible commands are:

```text
doctor
version
data inspect
data convert
kinetics analyze
kinetics plot
kinetics artifact
```

There is no artifact verify/inspect/import/register command, no project
command, and no run command.

`data inspect`, `data convert`, `kinetics plot`, and `kinetics artifact` use
bounded readers. The artifact path reads at most 16 MiB of strict BOM-free
UTF-8 CSV and publishes at most one 4 MiB envelope.

Legacy `kinetics analyze` still uses unbounded `fs::read_to_string`. That is a
real trust-boundary hardening gap, but Phase 6 deliberately classified it as a
separate compatibility change rather than silently changing an existing
command.

`kinetics artifact` creates an unregistered envelope only. It creates no
manifest sidecar, project, run record, event ledger, database, lock, journal,
cache, or log. Users can transport that envelope, but the product cannot read
it back or validate it after transport.

### Model, tools, and sandbox reality

The model crate contains provider-neutral request, response, capability,
routing, privacy/cache, usage, and error types plus a synchronous provider
trait. No provider implementation exists.

The DeepSeek crate contains descriptors and mock pricing only. It has no
network client and does not read API keys.

The tools crate contains versioned metadata, JSON-schema declarations, calls,
results, permissions, risks, and an in-memory deterministic registry. It has no
handler/executor, schema-enforcement runtime, approval collection, process or
network implementation, resource enforcement, or durable audit ledger.

The sandbox crate contains deny-by-default policy and a runner trait. Its only
runner implementation is test-only and returns unsupported execution. Lexical
path policy is not a canonical or symlink-safe filesystem sandbox.

No model, tool, or sandbox provenance is durably linked to a real run.

## Historical Commitments

### Phase 1 and Phase 2

Phase 1 established domain-neutral contracts and interfaces, not a real agent
runtime. The first kinetics workflow was explicitly a vertical validation
target rather than a core assumption.

The v0.2 roadmap chose deterministic structured output before artifact
persistence, provider execution, or a second workflow. The Phase 2
artifact-mapping design froze a path-free in-memory proposal and deferred
artifact ID assignment to a future registry, storage, or caller boundary.

### Phase 3 persistence sequencing

Phase 3 established this order:

1. explicit one-file deterministic JSON export;
2. exact-byte artifact identity and an artifact persistence design;
3. a versioned run-record design; and
4. only later a project-backed saved run.

It expressly stated that result JSON export was not a managed artifact.
Separate payload and manifest files required an artifact-ID policy, naming
policy, and multi-file failure policy first. A future run record required
schema versioning, ID generation, deterministic-versus-runtime field
separation, artifact relationships, and event relationships.

Phase 3 also recorded that lexical path planning did not establish symlink-safe
containment and that a project-backed saved run would create several files and
require workspace, run, manifest, and transaction semantics.

### Phase 4 and Phase 5

Phase 4 split read-only data inspection from explicit narrow conversion. It
refused silent metadata removal, unit-row removal, matrix extraction, semantic
column inference, and general CSV repair.

Phase 5 selected one fixed deterministic SVG rather than a generic chart or
report framework. It required a second real consumer before generalizing.

Both phases preserved explicit bounded effects, one target per write command,
no overwrite, existing parents, and no hidden project/run/artifact persistence.

### Phase 6 identity and transaction decisions

Phase 6 closed only this link:

```text
deterministic analysis bytes
  -> portable provenance-bearing unregistered envelope bytes
```

It froze the following distinctions:

- the proposal semantic hash is not the exact payload-byte hash;
- the exact payload/source hashes are content verifiers, not instance IDs;
- no content-addressed UUID is invented;
- an unregistered envelope has no runtime or registered identity;
- registration waits for an explicit project/run repository operation;
- the envelope has no self-hash;
- review metadata maps an existing review and does not rerun it; and
- one envelope file avoids an undefined two-file transaction.

Phase 6 explicitly deferred registered artifact identity, stable repository
references, project workspaces, versioned run/event persistence, multi-file
transactions, recovery journals, locks, provider execution, tool execution, a
second workflow, UI, and generic frameworks.

### Long-term direction and unresolved architecture

The long-term direction remains STEM-wide extensibility across chemistry,
physics, materials science, engineering, mathematics, bioinformatics, and
related domains. Domain-neutral core, artifacts, and storage must not become
kinetics-specific.

Before this RFC, the unresolved architectural decisions included:

- who assigns registered artifact identity;
- whether equal content may have multiple registered instances;
- how a registered reference locates payload bytes;
- whether registration copies, moves, or references an external envelope;
- project schema, directory ownership, migration, and containment;
- truthful run execution and event emission;
- versioned durable run/event schemas; and
- multi-file crash, retry, rollback, and recovery semantics.

This RFC does not answer those future persistence questions by smuggling them
into verification. It selects a read-only prerequisite that needs none of them.

RAG is not part of Phase 7 or the planned architecture established by this RFC.

## Missing Architectural Link

The single most important missing link is:

```text
The product can create and transport kinetics.artifact.v1, but it cannot read
that file back and distinguish a valid internally consistent envelope from a
corrupt, incompatible, or source-mismatched envelope.
```

This is one gap. It is not a request for registration, import, project state,
run persistence, model execution, scientific recomputation, or authenticity.

Closing it makes the v0.6 artifact user path bidirectional at the contract
level:

```text
create one portable envelope -> transport it -> verify what the bytes prove
```

It is also the correct prerequisite for any later import or registration
operation. A repository must not register untrusted envelope bytes before a
bounded schema and integrity reader exists.

## Candidate Direction Comparison

### Scoring method

Each candidate receives an ordinal planning score from 1 (poor) to 5 (strong)
for all required criteria:

| Code | Criterion |
| --- | --- |
| AL | Architectural leverage |
| UV | User-visible value |
| PC | Prerequisite correctness |
| SS | Scope singularity |
| DS | Disk safety |
| DI | Determinism and identity clarity |
| RS | Recovery semantics |
| CP | Compatibility |
| DN | Domain neutrality |
| DD | Dependency discipline |
| AU | Auditability |
| RQ | Roadmap sequencing |

The numbers are an evidence-backed ordering aid, not a measurement. A high
total cannot waive a material prerequisite, scope, disk, or recovery blocker.

| Candidate | AL | UV | PC | SS | DS | DI | RS | CP | DN | DD | AU | RQ | Total / 60 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| A — read-only artifact verification | 5 | 5 | 5 | 5 | 5 | 5 | 5 | 5 | 4 | 5 | 5 | 5 | **59** |
| B — project init | 3 | 2 | 2 | 4 | 2 | 2 | 1 | 4 | 5 | 5 | 2 | 3 | 35 |
| C — artifact registration | 4 | 4 | 2 | 3 | 2 | 2 | 1 | 4 | 5 | 5 | 2 | 3 | 37 |
| D — single-file run record | 5 | 5 | 2 | 4 | 5 | 2 | 5 | 4 | 5 | 5 | 3 | 4 | **49** |
| E — project-backed saved run | 5 | 5 | 1 | 1 | 1 | 1 | 1 | 2 | 5 | 4 | 1 | 2 | 29 |
| F — provider execution | 4 | 5 | 1 | 3 | 4 | 2 | 2 | 3 | 5 | 1 | 1 | 2 | 33 |
| G — tool/sandbox execution | 4 | 4 | 1 | 3 | 2 | 2 | 2 | 3 | 5 | 4 | 2 | 2 | 34 |
| H — second scientific workflow | 2 | 3 | 3 | 4 | 4 | 4 | 4 | 5 | 3 | 5 | 4 | 2 | 43 |
| I — input/compatibility hardening | 1 | 3 | 5 | 5 | 5 | 5 | 5 | 2 | 5 | 5 | 5 | 2 | 48 |
| J — UI/reporting/generic framework | 2 | 3 | 1 | 2 | 2 | 2 | 2 | 2 | 3 | 1 | 2 | 1 | 23 |

Candidate D is the architectural runner-up because a truthful portable run
record would unlock later model/tool auditability while remaining single-file.
It is not selected because its execution-truth prerequisite fails today.
Candidate I is highly bounded and auditable but closes a compatibility defect,
not the missing artifact lifecycle link.

### Required comparison

| Candidate | Value | Preconditions | Risk / premature concerns | Decision |
| --- | --- | --- | --- | --- |
| A — read-only artifact verification and inspection | Lets a recipient validate an existing v0.6 envelope and see verified metadata without writing | Existing v1 schema, exact hashes, serializer, bounded read, `serde_json`, and BLAKE3 already exist | Must remain schema-specific; without source it cannot validate source bytes; hashes do not prove authenticity or scientific recomputation | **Select verification as the only mainline; use one report, not a second inspect command** |
| B — minimal project workspace initialization | Establishes a durable owner/container for later state | Project schema, ID timing, minimal directory set, containment, symlink, create-new, retry, recovery, and migration contracts | Empty project has little current consumer value; current layout is broad and pure; directory-plus-metadata partial state is undefined | Defer as a separate future release |
| C — registered artifact import/repository | Gives artifact instances repository identity and durable lookup | A verifier, project/repository owner, ID and duplicate policy, copy/move/reference policy, payload locator, and transaction/recovery | Current traits store manifest metadata only; envelope/manifest mapping is absent; split files create partial-state risk | Defer; verification is a prerequisite |
| D — versioned single-file run record | Gives users a portable auditable record and advances later model/tool provenance | Truthful execution integration, versioned run schema, event meaning, artifact reference, and runtime/content identity separation | Current CLI does not execute through `AgentRun`; current events cannot reconstruct the run; writing planned history would be false | Runner-up, but reject for v0.7 |
| E — project-backed saved run | Eventually provides project, run, artifact, and replay state together | B, C, and D plus multi-file transaction, lock, journal, recovery, and migration | It bundles at least four independent problems and cannot state a credible bounded final inventory today | Reject as premature and non-singular |
| F — DeepSeek/provider execution | Adds model-assisted behavior | API-key loading, network permission, redaction, cost limits, timeout/retry, provider errors, privacy enforcement, cache policy, provenance, and offline tests | Large new trust boundary, nondeterminism, no durable run audit, and likely new runtime/network dependencies | Defer |
| G — tool execution and sandbox runner | Enables controlled external actions | Permission mapping, schema validation, path containment, process/network implementation, resource caps, approval, failure isolation, and audit records | Current registry/policy are metadata/interfaces only; no durable execution record exists | Defer until truthful durable run/audit contracts |
| H — second scientific workflow | Tests domain-pack breadth | Real user need, explicit input contract, and a stable reusable platform boundary | Risks adding chemistry breadth while bypassing artifact read, project, run, model, and tool gaps | Defer until platform contracts and demand justify it |
| I — input/compatibility hardening | Bounds legacy analyze or accepts more laboratory formats | Separate compatibility contract and precise scientific transformation policy | Bounding analyze changes an existing trust boundary; richer repair can silently alter scientific meaning; too narrow as the architecture mainline | Track as a separate compatibility release |
| J — UI, reporting, or generic framework | Improves presentation or abstracts output | Stable headless project/run/artifact contracts and a second real consumer | Violates headless sequencing, creates new toolchain/state, and is YAGNI without a second consumer | Reject for Phase 7 |

### Candidate A findings

The selected verifier is not generic artifact infrastructure. It accepts only
the already released `kinetics.artifact.v1` outer schema and its embedded
`kinetics.analysis.v1` payload.

There is no second user-visible artifact schema, so Phase 7 does not add:

- an arbitrary schema registry;
- plugin dispatch;
- a generic top-level artifact framework;
- a second `inspect` command; or
- speculative support for future artifact schemas.

Without `--source`, the command can prove:

- the outer file is bounded regular-file UTF-8 and canonical
  `kinetics.artifact.v1`;
- exact required fields, labels, and field sets are present;
- the decoded payload length and BLAKE3 match the descriptor;
- the payload is strict `kinetics.analysis.v1` with the frozen structural
  invariants;
- outer review summary matches the embedded review; and
- provenance fields satisfy the fixed v1 contract.

Without `--source`, it cannot prove that any currently available source file
matches the declared source descriptor.

With `--source`, it additionally hashes the exact raw bytes read from that one
explicit file and compares both byte length and BLAKE3. It does not decode,
parse, normalize, or analyze the source.

Neither mode proves:

- that kinetics fitting was rerun;
- that the reported science is correct;
- that the producer identity is authentic;
- that maliciously coordinated metadata and hash changes did not occur;
- that the envelope has a trusted signature or external digest; or
- that the envelope is registered.

The output must say that scientific recomputation was not performed and
authenticity was not established.

### Candidate B findings

A credible minimal future project would need a versioned record such as
`project.schema.v1`, an explicit project ID and user name, and a much smaller
initial directory surface than the current broad Phase 1 layout.

If this candidate were later selected:

- the project ID should be generated only after argument, root, existing-path,
  and containment preflight succeeds, but before the first durable project
  byte is rendered;
- only the project directory and one metadata file should be immediate;
- raw, derived, run, artifact-kind, and log directories should be lazy;
- the metadata file and ID need a schema version from the first release;
- existing targets must be create-new failures;
- a failed create must have explicit partial-directory detection and retry
  semantics; and
- filesystem containment and symlink policy must be enforced by real
  filesystem operations, not `Path::starts_with` or lexical joining.

Current code cannot atomically publish a directory plus metadata file and has
no recovery marker. Eagerly creating the current layout would materialize many
unused directories and prematurely freeze Phase 1 planning names. No current
command consumes an empty project. This candidate is therefore not ready.

### Candidate C findings

Registration must have an explicit repository owner. In the current
architecture that should ultimately be a project or another separately
designed repository root. Implementing registration without one would invent
a parallel hidden workspace.

The current code cannot yet answer, with implementation-grade evidence:

- whether `ArtifactId` is generated by the repository or caller;
- whether equal bytes may create multiple registered instances;
- whether an envelope is copied, moved, or externally referenced;
- whether manifest and payload are one file or multiple files;
- where a stable `ArtifactRef` resolves;
- how duplicate registration is detected; or
- how partial multi-file state is recovered.

Moving would mutate user input and is an unsafe default. Referencing an
external path would not give the repository ownership. Copying is the likely
future safe default, but it immediately needs owner, naming, collision, and
commit semantics. The existing repository trait saves only a manifest and does
not store payload bytes. Because these questions cannot yet be answered within
a bounded one-feature contract, this candidate is judged premature rather than
left for an implementation to improvise.

### Candidate D findings

A future single-file run record is the strongest runner-up because it can avoid
project directories and multi-file transactions. A credible record would need:

- an explicit run schema version;
- a real `RunId`;
- workflow ID and ordered executed steps;
- truthful state transitions;
- artifact references with defined semantics;
- ordered events or a clearly different summarized history;
- validation/replay rules; and
- an explicit statement that runtime identity makes otherwise equal runs
  distinct instances.

If this candidate is selected in a future release, the record must be one
portable, unregistered file. It must not require a project, allocate registered
repository identity, or imply that embedded artifact references are registered.
Its `RunId` would be runtime instance identity carried by the portable record,
not registered repository identity or deterministic content identity.

Embedding a random runtime `RunId` means complete record bytes are not
deterministic across executions. A separate content identity could still cover
deterministic portions, but it must not be called the run instance ID. A
caller-provided ID could make test bytes repeatable, but does not solve who owns
runtime identity.

The current CLI does not execute through `AgentRun`; the skeleton does not emit
events; event payloads do not reconstruct full steps or artifact linkage.
Generating a completed run record from current static workflow labels would
fabricate execution history. The candidate remains premature until truthful
execution/event semantics are separately established.

### Candidate E findings

`--project <path> --save-run` would combine:

1. project initialization;
2. registered artifact import or creation;
3. a versioned run record; and
4. a multi-file commit/recovery protocol.

It would write several final files and directories, but the repository cannot
currently state the maximum count, generated names, ownership, crash points,
rollback, journal, lock, parent sync, or retry behavior without first deciding
B, C, and D. It therefore fails scope singularity and the mandatory multi-file
decision gate.

### Candidate F findings

Provider execution is not selected merely because the repository name contains
DeepSeek. A production provider path would require all of:

- API-key source and precedence;
- zero accidental key logging and explicit redaction;
- opt-in network permission consistent with `LocalOnly`;
- request size and cost budgets;
- rate-limit, timeout, cancellation, and bounded retry policy;
- provider error mapping;
- prompt-cache semantics;
- user-data privacy policy enforcement;
- model/version/request/usage provenance;
- nondeterminism disclosure;
- offline deterministic tests; and
- a real provider implementation and likely network/runtime dependencies.

Adding those boundaries before a truthful durable run/audit record would make
model activity harder, not easier, to audit. It remains deferred.

### Candidate G findings

Tool execution requires a concrete mapping between tool and sandbox
permissions, validated arguments/results, explicit approval, canonical
filesystem containment, network/process implementation, CPU/time/memory/output
limits, child cleanup, failure isolation, idempotency policy, and durable call
records.

The current registry has metadata but no handler. The current runner is an
interface and lexical policy is not an OS sandbox. Model/tool provenance cannot
currently be persisted as a truthful run. Run persistence or an equivalent
durable audit contract should precede real tool execution.

### Candidate H findings

A second workflow could eventually prove domain-pack extensibility only if it
uses stable reusable platform contracts instead of copying the kinetics
vertical slice. Today:

- the first workflow is not executed through the core run skeleton;
- artifact creation is one-way;
- project/run/model/tool boundaries are not closed; and
- there is no repository evidence of a second user input contract.

Adding another chemistry workflow would demonstrate breadth, not platform
extensibility. A future workflow requires real user demand and an explicit
bounded input contract.

### Candidate I findings

Legacy `kinetics analyze` has an unbounded read while newer commands use the
16 MiB bounded reader. Aligning it is worthwhile, but it changes an existing
command's accepted inputs and should receive a separate compatibility review,
help change, and exact-limit/limit-plus-one tests.

Richer CSV, unit rows, metadata removal, matrix extraction, and instrument
imports are not interchangeable hardening. Automatic repair can be
scientifically ambiguous and must remain explicit. Neither track is sufficient
as the sole Phase 7 architecture mainline.

### Candidate J findings

The repository does not yet have stable durable project, run, or registered
artifact contracts for a UI to consume. There is no second renderer/report
consumer that justifies a generic framework. UI would also introduce a
forbidden Phase 1 toolchain expansion such as Node, TypeScript, browser, or
desktop dependencies.

The existing fixed JSON and SVG outputs remain the correct headless surfaces.

## Decision

Phase 7 has exactly one release mainline.

The frozen release decision is:

```text
v0.7.0 = read-only verification of kinetics.artifact.v1 envelopes
```

This selection closes the current write-side/read-side asymmetry with the
smallest correct contract:

- one existing envelope file is required;
- one original source file is optional;
- zero files and zero directories are created;
- exact payload integrity is always checked;
- exact source integrity is checked only when explicitly supplied;
- one deterministic human-readable verification report is emitted;
- no scientific recomputation or authenticity claim is made;
- no runtime or registered identity is allocated; and
- no new dependency is needed.

Candidate D is the runner-up because a versioned single-file run record has
high architectural leverage and can remain disk-bounded. It is not selected
because current production execution does not pass through `AgentRun` or emit a
complete event stream. Verification is already supported by real v0.6 bytes;
a run record would currently be based on planned rather than proven execution.

The decision is frozen unless implementation evidence reveals a correctness,
security, compatibility, or bounded-resource contradiction. Convenience is not
authorization to add import, registration, project, run, provider, tool,
workflow, UI, generic framework, or RAG work.

## User Story

As a recipient of one `kinetics.artifact.v1` file, I can ask the CLI to verify
the bounded envelope without modifying it. I receive a concise report that
confirms the outer schema, embedded payload integrity, provenance/review
invariants, and whether source bytes were checked.

### Acceptance criteria

- Given a canonical v0.6 envelope with a valid embedded payload, when I run the
  verifier without `--source`, then it exits zero, reports payload integrity as
  verified, reports source integrity as not checked, and states that scientific
  recomputation and authenticity were not established.
- Given the same valid envelope and its exact original raw source, when I add
  `--source`, then it also reports source integrity as verified.
- Given a source whose bytes or byte length differ, then verification fails,
  stdout is empty, and neither input is changed.
- Given corrupt, noncanonical, unknown-version, duplicate-field, unknown-field,
  over-limit, or structurally invalid envelope bytes, then verification fails
  before any success report.
- Given a valid envelope whose declared payload hash or length does not match
  its decoded payload, then verification fails as an integrity error.
- Existing create, inspect, convert, analyze, export, and plot behavior remains
  unchanged.

## Proposed User Interface

### Command

The only new execution surface is:

```text
deepseek-science kinetics artifact verify \
  --input <envelope-path> \
  [--source <original-source-path>]
```

It is nested under the existing kinetics artifact surface because only
`kinetics.artifact.v1` is supported. There is no generic top-level
`artifact verify`.

`--input` is required exactly once. `--source` is optional and may appear at
most once. Options may appear in either order. Empty values, values beginning
with `-`, duplicates, unknown options, missing values, positional values, and
missing `--input` are user errors.

The command does not require a `.json` extension. Content and schema, not a file
name suffix, determine compatibility. It accepts no stdin marker and performs
no directory, recursive, glob, or batch input.

The existing create command remains:

```text
deepseek-science kinetics artifact \
  --input <source-path> \
  --time-column <column> \
  --concentration-column <column> \
  --output <path.json>
```

The dispatcher recognizes the exact first argument `verify` before invoking
the existing create parser. Existing flag-first creation invocations are
unchanged.

### Help

The exact verifier help is:

```text
Usage:
  deepseek-science kinetics artifact verify \
    --input <envelope-path> \
    [--source <original-source-path>]

Options:
  --input <envelope-path>          Existing kinetics.artifact.v1 envelope
  --source <original-source-path>  Optional original file for exact-byte comparison
  -h, --help                       Show this help

Limits and behavior:
  The envelope is limited to 4 MiB and must be canonical UTF-8 kinetics.artifact.v1.
  The optional source is limited to 16 MiB and is compared as exact raw bytes.
  The command writes no files or hidden state.
  It does not import, register, repair, recompute, or authenticate the artifact.
```

`--help` or `-h` succeeds only when it is the sole argument after `verify`.
The root help and existing `kinetics artifact --help` remain byte-for-byte
unchanged. `deepseek-science kinetics` usage adds exactly this one line
immediately after its existing `artifact` line:

```text
  artifact verify   Verify one existing kinetics artifact envelope without writing files
```

### Successful stdout

On success, stdout is exactly these LF-terminated lines, with numeric and enum
placeholders replaced by validated values:

```text
verification: passed
artifact_schema: kinetics.artifact.v1
payload_schema: kinetics.analysis.v1
payload_bytes: <validated-u64>
payload_integrity: verified
source_bytes: <declared-u64>
source_integrity: <not_checked|verified>
workflow_id: chemistry.kinetics_csv
workflow_step: produce_analysis_result
producer_command: kinetics.artifact
review_status: <passed|passed_with_warnings|failed>
review_finding_count: <validated-u64>
scientific_recomputation: not_performed
authenticity: not_established
```

There is exactly one final LF. Success writes nothing to stderr.

The report intentionally omits:

- input and source paths;
- raw hash values;
- payload JSON;
- review messages;
- producer version values;
- UUIDs;
- timestamps;
- environment/build paths; and
- any claim that the scientific result was reproduced.

Omitting untrusted strings keeps terminal output bounded and avoids control
character injection. The verifier preserves `producer_version` only while
reconstructing and canonical-reserializing the existing v1 envelope. It does
not print that value or impose a new character, length, or current-binary
version rule.

The report is a human-readable CLI contract, not a new JSON or durable schema.
There is no `--json` in Phase 7.

### Stderr and exit status

- help: exit `0`, help on stdout, empty stderr;
- successful verification: exit `0`, exact report on stdout, empty stderr;
- user, IO, compatibility, or integrity failure: exit `1`, empty stdout, one
  concise `error: ...` line on stderr, with usage appended only for argument
  errors;
- internal invariant failure: exit `2`, empty stdout, one concise error on
  stderr.

### Forbidden flags and modes

The verifier has no:

```text
--output
--json
--inspect
--import
--register
--artifact-id
--project
--save-run
--run-id
--repair
--rewrite
--recompute
--trust
--signature
--force
--overwrite
--schema
--format
--model
--tool
--rag
```

There is no second `artifact inspect` command. The verified stdout report is
the only inspection surface.

## Schema and Identity Contract

### No new durable schema

Phase 7 introduces no new persisted schema. It consumes exactly:

```text
outer:   kinetics.artifact.v1
payload: kinetics.analysis.v1
```

It emits text only and writes no record.

### Strict outer schema

The verifier accepts the exact Phase 6 outer shape:

| Path | Required contract |
| --- | --- |
| `schema_version` | exact `kinetics.artifact.v1` |
| `artifact.kind` | exact `json` |
| `artifact.title` | exact `Chemistry kinetics analysis result` |
| `artifact.content.media_type` | exact `application/json` |
| `artifact.content.schema_version` | exact `kinetics.analysis.v1` |
| `artifact.content.encoding` | exact `utf-8` |
| `artifact.content.byte_length` | non-negative `u64`, exact decoded payload byte length |
| `artifact.content.hash.algorithm` | exact `blake3` |
| `artifact.content.hash.value` | exactly 64 lowercase hexadecimal ASCII bytes |
| `artifact.inputs` | exactly one element |
| `artifact.inputs[0].role` | exact `source_csv` |
| `artifact.inputs[0].byte_length` | non-negative `u64` |
| `artifact.inputs[0].hash.algorithm` | exact `blake3` |
| `artifact.inputs[0].hash.value` | exactly 64 lowercase hexadecimal ASCII bytes |
| `artifact.provenance.workflow_id` | exact `chemistry.kinetics_csv` |
| `artifact.provenance.workflow_step` | exact `produce_analysis_result` |
| `artifact.provenance.producer_command` | exact `kinetics.artifact` |
| `artifact.provenance.producer_version` | exact non-empty v1 string; preserved for canonical reserialization but not printed |
| `artifact.review.status` | `passed`, `passed_with_warnings`, or `failed` |
| `artifact.review.finding_count` | non-negative `u64`, exact payload findings length |
| `payload_utf8` | non-empty strict UTF-8 payload text with exactly one final LF |

Every object must contain exactly its v1 fields. Unknown fields and duplicate
keys are compatibility errors, including unknown fields that a permissive
Serde default would otherwise ignore. Missing nullable payload members are
also rejected.

The outer bytes must match the frozen Phase 6 canonical envelope byte contract:

- UTF-8 without BOM;
- LF only;
- two-space pretty indentation;
- normative declaration order;
- no trailing spaces;
- exactly one outer final LF; and
- byte-for-byte equality with reserialization through the existing typed v1
  serializer.

No self-hash is added. Canonical reserialization validates the v1 encoding; it
does not authenticate the producer.

### Strict embedded payload

The decoded payload must:

- be strict UTF-8 without BOM or carriage return;
- end with exactly one LF;
- parse as one JSON object before that final LF;
- contain exactly the established top-level fields
  `schema_version`, `command`, `input`, `columns`, `counts`, `fits`,
  `comparison`, and `review`;
- declare `schema_version: kinetics.analysis.v1`;
- declare `command: kinetics.analyze`;
- contain exactly `input.path`;
- contain exactly `columns.time` and `columns.concentration`;
- contain exactly `counts.valid_points` and `counts.rejected_rows` as
  non-negative integers;
- contain exactly `fits.first_order` and `fits.second_order`;
- give each fit exactly `k`, `slope`, `intercept`, `r_squared`, and
  `valid_point_count`;
- use finite JSON numbers for all fit metrics;
- use a fit `valid_point_count` equal to `counts.valid_points`;
- contain the fixed comparison basis
  `finite_r_squared_mvp_heuristic`;
- use `first_order` or `second_order` as the preferred model;
- contain the fixed caution
  `preferred_by_mvp_r_squared_heuristic_not_final_scientific_model_selection`;
- contain exactly `review.status` and `review.findings`;
- use only the existing review status, severity, check, and model labels;
- include all five finding members even when `model` or
  `rejected_row_count` is null; and
- contain at most six findings, the maximum possible from the frozen current
  finite review-check surface.

The verifier validates deterministic review consistency without rerunning the
review:

- any error finding requires status `failed`;
- warnings without errors require `passed_with_warnings`;
- no findings require `passed`;
- a `rejected_rows_visible` finding must carry the payload's nonzero rejected
  row count; and
- outer review status/count must equal payload review status/length.

It validates stable labels and structural relationships only. It does not:

- recompute either fit;
- compare `k` with slope;
- select a model from `r_squared`;
- parse the source as CSV;
- validate columns against the source;
- reproduce row rejection; or
- rerun deterministic reviewer functions.

The exact payload byte length and BLAKE3 are always recomputed from
`payload_utf8` after JSON string decoding, including its final LF.

### Versioning and unknown data

Only exact v1 schemas are supported. An unknown outer or payload version is a
compatibility error, not an invitation to best-effort parsing.

There is no migration, upconversion, downgrade, version registry, or
`--schema` override. Supporting a future v2 requires a separately reviewed
adapter and compatibility plan.

Unknown fields are rejected. The verifier never strips unknown data and then
reports success.

### Identity separation

The selected capability freezes these four meanings:

| Identity category | Phase 7 contract |
| --- | --- |
| Content identity | Existing exact payload and source BLAKE3 descriptors; they verify bytes but are not IDs |
| Runtime instance identity | None generated, read, inferred, or persisted |
| Registered repository identity | None; `ArtifactId` and `ArtifactManifest` are not used |
| User-facing name/path | Explicit read selectors only; paths are not identity and are not printed or persisted |

Specific consequences:

- no ID is random because the verifier creates no ID;
- no caller, CLI, repository, or storage layer assigns an ID;
- no ID is written into deterministic bytes;
- equal envelopes at different user paths are both valid independent files;
- equal payload bytes may appear in multiple unregistered envelopes;
- the verifier performs no deduplication;
- no ID is replayed;
- an artifact/source path is not content, runtime, or repository identity;
- no timestamp is accepted or generated;
- an unexpected ID or timestamp field is an unknown-field error;
- duplicate registration is not applicable because registration does not
  occur; and
- ID collision handling is not applicable.

The existing payload includes the original producer's caller-supplied input
path. Same raw source bytes produced under different input path strings can
therefore have the same source hash but different payload and envelope bytes.
The verifier preserves that fact and does not canonicalize, compare, resolve,
or print the embedded path.

For hashes, a computed length or digest mismatch is an integrity error. A
matching internal digest is not a signature, trusted external anchor, or
proof against malicious coordinated replacement. `authenticity:
not_established` is mandatory.

## Execution Flow

The command has one linear bounded flow:

1. recognize exact help or parse `--input` and optional `--source`;
2. reject all argument errors before opening a file;
3. open the explicit envelope path read-only;
4. read metadata from the opened handle and require a regular file;
5. reject metadata length greater than 4 MiB;
6. bounded-read at most 4 MiB plus one detection byte;
7. reject an over-limit, empty, BOM-prefixed, non-UTF-8, CR-containing, or
   incorrectly terminated outer byte sequence;
8. strictly parse the outer v1 object while rejecting duplicates, missing
   fields, unknown fields, wrong types, invalid hash text, and unknown version;
9. reconstruct the typed unregistered envelope and serialize it once through
   the existing bounded canonical serializer;
10. require canonical bytes to equal the original envelope bytes exactly;
11. validate fixed kind, title, content, input, provenance, and review fields;
12. obtain the exact decoded `payload_utf8` bytes without normalization;
13. recompute payload byte length and BLAKE3 and compare both descriptors;
14. strictly parse the existing `kinetics.analysis.v1` payload and validate its
    exact field sets, types, finite values, labels, counts, and review
    consistency without fitting or review execution;
15. cross-check outer review status/count against the payload;
16. preserve the existing non-empty producer-version value for typed canonical
    reserialization while excluding it from terminal output;
17. if `--source` is absent, set source integrity to `not_checked`;
18. if `--source` is present, open that explicit path read-only, require the
    opened target to be a regular file, reject metadata length over 16 MiB, and
    bounded-read at most 16 MiB plus one detection byte;
19. hash the exact raw source bytes once and compare declared length and
    BLAKE3, without decoding or parsing them;
20. construct the complete fixed report in memory;
21. verify the report contains only validated fixed labels and integers; and
22. emit the report once and exit zero.

The command must not:

- reopen or reread either file;
- canonicalize a path;
- search a directory;
- decode or parse the optional source;
- run kinetics analysis or reviewer code;
- rewrite or normalize JSON;
- write a file;
- create a temporary sibling;
- call storage publication;
- allocate an ID;
- import or register the envelope;
- create project/run/event state;
- call a model or tool;
- access the network; or
- perform background activity.

An explicitly supplied symlink may follow the platform's normal read-only open
semantics, after which metadata on the opened handle must identify a regular
file. No project-root containment is claimed because both paths are explicit
read selectors and no write occurs.

The optional source comparison covers the exact bytes actually returned by the
one bounded read. It does not claim the file remained unchanged before or after
that read.

## Resource Bounds

The limits are fixed, non-configurable, and reuse released boundaries:

| Resource | Maximum |
| --- | ---: |
| Envelope metadata length | `4 * 1024 * 1024` bytes |
| Envelope bounded-reader observation | envelope maximum plus one byte |
| Decoded payload | no more than `4 * 1024 * 1024` bytes and must fit inside the envelope |
| Optional source metadata length | `16 * 1024 * 1024` bytes |
| Optional source bounded-reader observation | source maximum plus one byte |
| Outer inputs opened | 1 required, 1 optional |
| Persistent final files | 0 |
| Created directories | 0 |
| Network/model/tool operations | 0 |
| Review findings | at most 6 |

Allocation remains in memory and bounded by the two input limits:

- one envelope byte buffer of at most 4 MiB;
- one strict parsed outer value/typed envelope derived only from that buffer;
- one canonical outer serialization buffer of at most 4 MiB;
- one decoded payload owned within or derived from the bounded envelope;
- one strict parsed payload whose arrays and fields are bounded by the exact
  schema and six-finding maximum;
- optionally one source byte buffer of at most 16 MiB; and
- one small fixed report string.

No streaming spool, memory map, decompression, archive reader, recursive JSON
extension, cache, log, worker, watcher, daemon, or configurable size flag is
introduced.

The exact v1 shape and pre-parse file cap bound parser work. The implementation
must keep Serde's recursion protection enabled and must not enable unbounded
recursion.

## Disk Safety

### Explicit intent

Verification happens only after the user supplies one exact envelope path.
Source comparison happens only after the user also supplies one exact source
path.

### Filesystem effects

Product effects are:

```text
required reads: 1
optional reads: 1
final writes: 0
temporary files: 0
directories created: 0
directories removed: 0
files deleted: 0
hidden state: 0
```

There is no parent-directory requirement because there is no output. There is
no overwrite, create-new, replace, sync, rename, hard link, lock, journal,
index, database, cache, or log.

`--output`, `--force`, `--overwrite`, `--repair`, and `--rewrite` are rejected
as argument errors before any read.

### Crash and recovery

Every crash point is read-only:

- before an open: no effect;
- during bounded read: no effect on either input;
- during parse/hash/validation: no effect;
- during report construction: no effect; and
- during stdout write: a caller may observe partial terminal output if the
  process or pipe fails, but no repository or input state requires recovery.

There is no partial durable state to detect, retry, roll back, or recover.
Retry reopens and revalidates the explicitly supplied current bytes.

Parent-directory sync, multi-file transaction, journal, and rollback are not
applicable.

### Test cleanup

Pure parsing and integrity tests should use in-memory bytes. Process tests may
create only tiny harness-owned envelope/source files beneath Cargo's configured
external target temp root, must inventory every path, and must remove only
their exact files plus a verified-empty owned directory. They must not use
recursive cleanup, globs, `remove_dir_all`, repository fixtures, snapshots,
golden files, caches, databases, logs, or `cargo clean`.

The product verifier itself creates nothing for tests to clean.

## Crate Ownership

### `deepseek-science-artifacts`

Owns:

- strict bounded deserialization of the existing generic unregistered envelope
  shape from caller-supplied bytes;
- duplicate/missing/unknown outer-field rejection;
- exact hash-text parsing;
- generic payload byte-length/hash verification;
- existing payload text invariants;
- reconstruction of the existing typed envelope; and
- canonical outer reserialization comparison.

It receives bytes and performs no file IO. It does not know
`kinetics.artifact.v1`, parse `kinetics.analysis.v1`, allocate an
`ArtifactId`, authenticate a producer, or register anything.

This is the read-side counterpart of an existing type, not a generic schema
registry or a new service abstraction.

### `deepseek-science-chemistry`

Remains the owner of kinetics computation, workflow constants, model labels,
review types, and review semantics.

Verification does not call `KineticsAnalysisResult::analyze`, fit functions,
or reviewer functions. No filesystem or CLI JSON responsibility moves into
chemistry merely to support this command.

### `deepseek-science-cli`

Owns:

- nested manual command parsing and exact help;
- bounded read-only regular-file IO;
- exact dispatch to `kinetics.artifact.v1`;
- the strict existing `kinetics.analysis.v1` input DTO/field-set validation
  beside the serializer it already owns;
- kinetics-specific fixed-label and cross-envelope review checks;
- optional exact raw-source comparison;
- safe terminal report construction;
- path-specific user errors; and
- exit-code/stdout/stderr behavior.

It does not implement generic schema plugins, registration, persistence, or
scientific recomputation.

### `deepseek-science-storage`

Remains unchanged and is not called. Verification has no output plan.
Repository traits remain interfaces only and must not be presented as a
backend.

### `deepseek-science-core`

Remains unchanged, domain-neutral, and unused by verification. No
`AgentRun`, event stream, `ProjectId`, `RunId`, or `ArtifactId` is created.

### Model, tools, and sandbox

Remain unchanged and inactive. The verifier performs no provider call, tool
call, permission request, sandbox execution, network access, process spawn, or
provenance persistence.

No new crate, repository implementation, registry, factory, service layer, or
framework is created.

## Compatibility

The new command is additive and read-only.

| Existing surface | Frozen compatibility |
| --- | --- |
| `data inspect` | Same arguments, 16 MiB boundary, report, and zero-write behavior |
| `data convert` | Same eligibility, 16/24 MiB limits, one create-new CSV, and refusal behavior |
| `kinetics analyze` | Same text mode, unbounded legacy read, analysis, and errors |
| `kinetics analyze --json` | Same complete `kinetics.analysis.v1` stdout bytes |
| `kinetics analyze --output` | Same one-file deterministic JSON export and no-overwrite behavior |
| `kinetics plot` | Same one-file bounded deterministic SVG behavior |
| `kinetics artifact` | Same four creation options, v1 envelope bytes, one-file output, and no sidecars |
| `kinetics.analysis.v1` | No field, value, ordering, or semantic change |
| `kinetics.artifact.v1` | No field, serialization, hash, identity, or semantic change |
| annotated `v0.6.0` tag | Remains at the existing tag object and commit; never moved or rewritten |

The only routing change in a future Phase 7.3 implementation is recognizing
the exact word `verify` immediately after `kinetics artifact`. Existing
flag-first creation syntax is unaffected.

The root help and `kinetics artifact --help` remain unchanged. The
`deepseek-science kinetics` usage gains only the exact frozen discoverability
line above, and `kinetics artifact verify --help` is new.

The v0.7 verifier must accept canonical envelopes emitted by the released
v0.6 binary. It must not require `producer_version` to equal the verifier
version. A later Phase 7.5 version alignment may cause newly created
`kinetics.artifact.v1` envelopes to declare producer version `0.7.0`; this
does not create a new artifact schema.

No existing command implicitly verifies, imports, registers, or creates extra
state.

## Error Handling

### User/input errors

Exit `1`, empty stdout:

- missing, duplicate, unknown, positional, or valueless arguments;
- artifact or optional source open failure;
- a path whose opened target is not a regular file;
- metadata or bounded-read IO failure; and
- envelope/source size over the fixed limit.

Argument errors include verifier usage. Read errors do not dump backtraces or
unrelated paths.

### Integrity errors

Exit `1`, empty stdout:

- noncanonical outer bytes;
- invalid hash syntax;
- payload byte-length mismatch;
- payload BLAKE3 mismatch;
- outer/payload review mismatch;
- structurally inconsistent payload counts/review;
- supplied source byte-length mismatch; or
- supplied source BLAKE3 mismatch.

Messages must say what category failed without printing payload, hash secrets,
review messages, or debug structures. A hash mismatch is not reported as a
scientific failure.

### Compatibility errors

Exit `1`, empty stdout:

- unknown outer or payload schema version;
- unknown, missing, or duplicate field;
- wrong fixed kind/media/encoding/workflow/step/command label;
- empty producer-version value;
- unknown enum/check labels; or
- v1 structure that cannot be interpreted without repair.

The verifier never ignores unknown fields, guesses a version, or mutates input
to continue.

### Internal invariant errors

Exit `2`, empty stdout:

- typed reconstruction succeeds but the existing serializer cannot reproduce a
  bounded canonical envelope;
- a validated integer cannot be represented where the internal contract
  requires it;
- the fixed report builder observes an impossible unvalidated value; or
- another bug violates a postcondition after external input has already passed
  validation.

### IO/publication errors

All IO is read-only and maps to user/input error unless an impossible internal
postcondition is violated. There is no publication, so target-exists,
missing-parent, temporary cleanup, commit, or sync errors cannot occur.

### Recovery-required errors

None. No durable state is created or modified. No error instructs the user to
repair a repository, delete a temporary file, or roll back a transaction.

## Dependency Policy

Default and frozen decision: no new dependency.

Use:

- `std::fs::File`, `Read`, and opened-handle metadata for bounded read-only IO;
- existing `serde` and `serde_json` for strict typed parsing;
- existing `blake3` through the current artifact hash helper;
- existing envelope types and serializer;
- existing CLI parser/report conventions; and
- current kinetics labels and constants where already public.

Do not add:

- a JSON canonicalization dependency;
- schema registry or JSON Schema engine;
- signature/PKI library;
- temporary-file library;
- database;
- async runtime;
- network client;
- CLI framework;
- logging framework;
- model/tool runtime;
- RAG/retrieval/vector dependency;
- UI framework; or
- new workspace crate.

The standard library and existing dependencies cover the complete selected
capability. There is therefore no transitive, network-fetch, build-size, or
rollback impact from a dependency change.

## Testing Plan

Phase 7.0 runs no tests or Cargo commands because it changes documentation
only. Later implementation must use the smallest relevant checks and obey the
repository's external target and cleanup rules.

### Pure unit tests

Test the artifact read-side contract entirely in memory:

- parse a canonical envelope produced through the existing typed serializer;
- reject empty, over-limit, BOM, CR, invalid UTF-8, invalid JSON, missing final
  LF, multiple final LF, and noncanonical outer bytes;
- reject every missing, duplicate, unknown, or wrong-typed outer field;
- reject unknown schema, kind, media type, encoding, role, algorithm,
  provenance, review label, and an empty producer-version value;
- accept exact 64-character lowercase BLAKE3 text and reject other lengths,
  case, or characters;
- recompute exact payload byte length/hash including its final LF;
- reject payload length/hash corruption;
- reject every missing, duplicate, unknown, or wrong-typed payload field;
- reject non-finite or nonnumeric fit data, invalid labels, invalid nullable
  members, count disagreement, more than six findings, and review-summary
  disagreement;
- prove parsing/verification creates no ID or timestamp;
- prove the original input bytes are unchanged; and
- prove a successful verification can reconstruct byte-identical canonical
  outer bytes.

Use small private limits to test exact limit and limit-plus-one behavior rather
than allocating committed 4 MiB fixtures.

### Process tests

Add one focused `kinetics_artifact_verify_smoke` suite that:

- verifies exact help and argument failures;
- verifies one valid existing v0.6-compatible envelope without source;
- verifies the same envelope with its exact raw source;
- rejects a one-byte source mutation;
- rejects a one-byte payload mutation without updating its hash;
- rejects a coordinated payload/hash change that violates strict payload
  structure;
- confirms stdout/stderr/exit codes;
- confirms the report says `not_checked`, `not_performed`, and
  `not_established` in the correct cases;
- confirms no path or raw hash appears in stdout;
- confirms `--output`, `--import`, `--register`, `--project`, `--save-run`,
  `--model`, `--tool`, and `--rag` are rejected;
- confirms artifact and source bytes, names, and entry inventory remain
  unchanged without asserting access-time stability; and
- confirms no output, temp sibling, directory, sidecar, log, cache, project,
  run, manifest, or database appears.

Generate tiny envelope bytes during the test through existing in-memory
builders. Do not commit a golden envelope, snapshot, generated JSON, or large
fixture.

### Determinism

- identical envelope/source bytes produce byte-identical reports;
- the same envelope at different read paths produces the same report;
- source omitted always reports `not_checked`;
- source supplied and matching always reports `verified`;
- no random ID, timestamp, path, environment value, or iteration-order output
  enters the report.

This is report determinism, not scientific recomputation.

### Identity

- no call reaches `ArtifactId::new`, `RunId::new`, or `ProjectId::new`;
- equal content at multiple paths is not deduplicated or registered;
- unexpected identity/timestamp fields are rejected;
- hashes are never labeled IDs; and
- paths are never labeled identity.

### Corruption and compatibility

- exercise corruption separately in outer formatting, outer metadata, payload
  length, payload hash, source length, source hash, payload schema, and review
  cross-checks;
- verify canonical envelopes emitted by the released v0.6 contract;
- reject unknown future versions without fallback; and
- retain all existing `data inspect`, `data convert`, `kinetics analyze`,
  analyze JSON/output, plot, and artifact creation process tests.

### No-overwrite and recovery

The verifier has no output. Tests must prove:

- artifact and source sentinel bytes remain unchanged;
- no metadata-changing operation is requested; tests do not assert access-time
  stability because a normal read may update it on some filesystems;
- `--output` and overwrite-like flags fail before reads;
- product execution creates zero files;
- interruption before stdout cannot leave recoverable state; and
- retry needs no cleanup or rollback.

### Disk inventory

Before and after every process case, inventory the harness-owned directory.
The product delta must be exactly zero entries. Harness inputs are small,
explicit, externally located, and exactly cleaned.

### No model, network, tool, or RAG

Source inspection and process behavior must prove no model provider, network,
tool registry execution, sandbox runner, subprocess, RAG, retrieval, vector,
database, watcher, daemon, or background path is reachable.

## Phase Breakdown

### Phase 7.0: roadmap audit and RFC freeze

- audit the annotated v0.6 baseline, historical commitments, current code, and
  all reasonable candidates;
- select exactly one mainline;
- create only this RFC;
- run no Cargo command; and
- make no implementation, version, tag, or Release change.

### Phase 7.1: pure bounded envelope read-side contract

- add strict in-memory deserialization for the existing unregistered envelope
  shape in `deepseek-science-artifacts`;
- reject duplicate, missing, and unknown fields;
- parse declared exact-byte hashes safely;
- verify payload text, byte length, and BLAKE3;
- reconstruct and canonical-reserialize the existing envelope within the
  4 MiB limit; and
- add pure unit tests only.

Phase 7.1 adds no CLI, file IO, source comparison, kinetics schema adapter,
identity, registration, persistence, dependency, version, tag, or Release.

### Phase 7.2: strict kinetics v1 verification adapter

- add the exact `kinetics.artifact.v1` dispatch and strict existing
  `kinetics.analysis.v1` field/type/label validation beside the current CLI
  serializer boundary;
- cross-check payload and outer review metadata;
- produce one small validated report value;
- make no filesystem call and perform no fitting or review execution; and
- keep generic artifacts free of chemistry concepts.

### Phase 7.3: read-only verifier CLI

- add only `kinetics artifact verify --input ... [--source ...]`;
- use one 4 MiB bounded envelope read and at most one 16 MiB bounded raw-source
  read;
- emit the exact frozen report;
- add focused process tests; and
- preserve zero writes and all existing commands.

### Phase 7.4: end-to-end verification audit

- audit actual binary help, success, corruption, optional-source, compatibility,
  determinism, identity absence, stdout safety, resource bounds, and zero disk
  effects;
- inventory and exactly clean only small external audit-owned inputs;
- confirm no dependency, model, network, tool, project, run, registered
  persistence, UI, or RAG path; and
- create an audit report only if separately authorized by that phase task.

### Phase 7.5: version alignment and release audit

- only after 7.1 through 7.4 pass, align the CLI package and matching lock entry
  to `0.7.0`;
- verify existing and new version surfaces and v0.6 artifact compatibility;
- perform the final v0.7 release audit;
- do not create or move a tag; and
- do not create a GitHub Release.

An annotated `v0.7.0` tag is reserved for a separate explicitly authorized
tag-only task after Phase 7.5. A GitHub Release must not be created
automatically.

## Deferred Work

All unselected candidates remain outside the Phase 7 release mainline:

- **Project initialization:** defer until minimal schema, containment,
  directory ownership, retry, recovery, and immediate user value are frozen.
- **Registered artifact import/repository:** defer until verification exists
  and project/repository ownership, instance identity, payload location,
  duplicate, and transaction semantics are independently frozen.
- **Versioned single-file run record:** defer until real execution emits
  truthful complete run/event/artifact relationships.
- **Project-backed saved run:** reject as a bundle until project,
  registration, run, and multi-file recovery each exist independently.
- **DeepSeek/provider execution:** defer until secret, network, cost, privacy,
  cache, failure, provenance, and durable audit contracts are ready.
- **Tool execution and sandbox runner:** defer until permission mapping,
  containment, resource limits, failure isolation, and durable audit are ready.
- **Second scientific workflow:** defer until a real user contract exists and
  reusable platform boundaries, rather than copied chemistry code, can support
  it.
- **Input/compatibility hardening:** keep legacy analyze bounding and richer
  laboratory import as separately reviewed compatibility work.
- **UI, reporting, and generic frameworks:** defer until stable headless
  project/run/artifact contracts and a second real consumer exist.

Also outside Phase 7:

- a separate `artifact inspect` command;
- arbitrary future artifact schemas;
- artifact repair or rewrite;
- signatures or authenticity infrastructure;
- artifact import, registration, repository backend, or database;
- project/run/event persistence;
- model or tool execution;
- another chemistry workflow; and
- UI or a new frontend toolchain.

RAG, embeddings, vector databases, vector stores, document indexing, retrieval
pipelines, semantic-search infrastructure, and automatic document retrieval are
explicitly excluded. They are not a deferred Phase 7.x subphase.

RAG is not part of Phase 7 or the planned architecture established by this RFC.

## Open Questions Resolved

### Is Phase 7 one release feature?

Yes. It is read-only verification of existing `kinetics.artifact.v1`
envelopes.

### Is inspection a second command?

No. The one verifier emits a bounded report of already validated metadata.

### Is the verifier generic across future artifact schemas?

No. There is one real user-visible envelope schema. Unknown versions fail.

### Does verification import or register the artifact?

No. It allocates no ID and writes no repository state.

### Is the original source required?

No. Payload/envelope verification is useful without it. The report must say
`source_integrity: not_checked`.

### What does `--source` do?

It reads at most one explicit 16 MiB regular file as raw bytes and compares
exact length and BLAKE3. It does not decode, parse, normalize, or analyze.

### Does source verification require the source path to match the embedded payload path?

No. Paths are not source content identity. Only exact bytes and byte length are
compared.

### Does verification rerun kinetics?

No. It checks structure and internal consistency only. The report says
`scientific_recomputation: not_performed`.

### Does a matching hash prove authenticity?

No. The envelope has no self-hash, signature, or trusted external digest. The
report says `authenticity: not_established`.

### Are exact payload/source hashes IDs?

No. They are content verifiers only.

### Is any runtime or repository ID generated?

No. No UUID, `ArtifactId`, `RunId`, or `ProjectId` is generated, accepted, or
written.

### Can equal content appear at multiple paths or in multiple envelopes?

Yes. Verification neither deduplicates nor registers. Path is not identity.

### Are timestamps accepted?

No. They are not part of v1; an added timestamp field is rejected as unknown.

### How are unknown fields and duplicate keys handled?

They are rejected at every v1 object boundary. No permissive ignore behavior.

### How are unknown schema versions handled?

As compatibility errors. There is no migration or best-effort fallback.

### What outer byte form is accepted?

Only the canonical Phase 6 pretty JSON byte contract, proven by bounded typed
reserialization comparison.

### Is a `.json` suffix required?

No. The read-only verifier dispatches by validated content, not a path suffix.

### What is the producer-version rule?

It is the existing exact non-empty v1 metadata value, not authenticated
identity. The verifier preserves it for canonical reserialization, does not
print it, imposes no new character or length rule, and does not require it to
equal the verifier's version.

### What are the exact read limits?

4 MiB for the required envelope and 16 MiB for the optional source, each with
one limit-plus-one detection read.

### What are the exact write, file, and directory effects?

Zero final writes, zero temporary files, zero created directories, and zero
deletions.

### What recovery protocol applies?

None. There is no durable mutation or partial state.

### Is a new dependency required?

No. The standard library and existing Serde/JSON/BLAKE3 contracts suffice.

### Does Phase 7 include project, run, model, tool, workflow, UI, or RAG work?

No.

## Recommended Next Step

Phase 7.1: implement only the pure bounded read-side contract for the existing
unregistered envelope in `deepseek-science-artifacts`, including strict
field-set parsing, exact payload length/hash verification, canonical
reserialization comparison, and focused in-memory unit tests. Add no CLI, file
IO, source comparison, kinetics adapter, identity, registration, persistence,
dependency, version, tag, or GitHub Release.
