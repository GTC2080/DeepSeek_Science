# Phase 1 Kernel Integration Audit

## Summary

Phase 1 kernel contracts are coherent and remain inside the intended
Rust-only, headless, domain-neutral scope. The inspected crates connect at the
contract level: workflow plans can prepare run skeletons, lifecycle events can
project into run inspection summaries, prompt hashes are stable-prefix based,
artifacts carry provenance metadata, storage only plans safe paths, and sandbox
policy remains deny-by-default.

No product code changes were needed. The repository appears ready to begin
Phase 2 design work, as long as Phase 2 keeps `chemistry.kinetics_csv` outside
generic crates and treats CSV parsing, tool execution, persistence, and provider
calls as explicit new implementation work.

Ponytail status: lean enough for this phase. Existing registries, traits, and
policies are current contracts rather than speculative execution systems.

## Commands Run

- `git status --short`
- `git branch --show-current`
- `git remote -v`
- `git log --oneline --decorate -n 8`
- Targeted `wc -l`, `sed`, and `rg --files` reads over the requested docs and
  crates
- Targeted `rg` scan for forbidden UI, TypeScript, network, database, CLI
  framework, file-write, subprocess, environment, and domain-leakage terms
- `cargo tree --workspace`
- `cargo fmt --check`
- `cargo check --workspace`
- `cargo test --workspace --lib`
- Source-tree directory presence checks for `target/`, `.cache/`, `tmp/`,
  `runs/`, `artifacts/`, and `test-output/`

All Cargo validation commands passed. Cargo output was written to the configured
external target directory, not the repository source tree.

## Crate Boundary Status

- `deepseek-science-core`: Pass. Owns IDs, `Project`, `Thread`, `AgentRun`,
  `RunStep`, `RunState`, `CoreEvent`, event envelopes, run inspection,
  workflow plan skeletons, and core errors. It remains domain-neutral and has
  no chemistry, UI, provider, storage implementation, or sandbox implementation
  dependency.
- `deepseek-science-model`: Pass. Owns provider-neutral capabilities,
  descriptors, requests, responses, routing decisions, usage accounting,
  cache/privacy policy, and the provider trait. No network client or API key
  logic was found.
- `deepseek-science-model-deepseek`: Pass. Contains DeepSeek descriptors and
  mock pricing/cost estimation only. No network client or API key logic was
  found.
- `deepseek-science-prompt`: Pass. Owns stable prefix compilation, variable
  tail separation, stable version metadata, and BLAKE3 prefix identity hashing.
  Tests confirm variable user request text does not affect `prefix_hash`, while
  stable sections, order, and version metadata do.
- `deepseek-science-tools`: Pass. Owns tool identity, definition schemas,
  calls, results, risk levels, permissions, and deterministic registry metadata.
  It contains no execution engine.
- `deepseek-science-common`: Pass. Owns pure in-memory numeric and table
  contracts: finite mean, simple linear regression, numeric columns, table
  shape, and small unit labels. It contains no CSV/file IO and no chemistry
  workflow.
- `deepseek-science-artifacts`: Pass. Owns artifact kinds, refs, manifests,
  provenance, review status, and content hashing helpers. It records metadata
  only and contains no persistence implementation.
- `deepseek-science-storage`: Pass. Owns deterministic layout helpers, path
  safety checks, atomic write planning contracts, and repository traits. It
  contains no JSONL, SQLite, file-writer, or real persistence implementation.
- `deepseek-science-sandbox`: Pass. Owns permission policy, approval decisions,
  request/result shapes, and runner interface only. Default policy denies
  network, subprocess, project-external path, and environment access.
- `deepseek-science-cli`: Pass. Remains a minimal `std::env::args` command
  surface with direct terminal output isolated here. No heavy CLI framework is
  present.

## Dependency Status

- Active workspace dependencies remain minimal: `serde`, `serde_json`,
  `thiserror`, `uuid`, and `blake3`.
- `cargo tree --workspace` showed no `reqwest`, `tokio`, `sqlx`, `rusqlite`,
  `clap`, UI crates, web framework crates, or TypeScript/Node/Bun/npm tooling.
- No new dependencies were added.
- No obviously unused workspace dependency was found during this audit.

## Disk Safety Status

- `.cargo/config.toml` still sets `target-dir = "../.cache/deepseek-science-target"`.
- `.gitignore` excludes source-tree build/cache/generated output including
  `/target/`, `/.cache/`, `/tmp/`, `/runs/`, `/artifacts/`, `/test-output/`,
  logs, coverage, profiling output, and local environment files.
- `scripts/clean-dev.sh` was inspected only. It targets the configured external
  Cargo target cache, rejects suspicious targets, prints the target, and
  requires exact `DELETE` confirmation.
- Source-tree `target/`, `.cache/`, `tmp/`, `runs/`, `artifacts/`, and
  `test-output/` directories were not present.
- Tests are deterministic unit tests and do not create uncontrolled files in
  the repository.
- No generated files were staged or intentionally created inside the source
  tree.

## Phase 1 Contract Status

- Run lifecycle: Present via `RunState`, transition validation, terminal-state
  helper, `AgentRun::transition_to`, and `transition_to_with_event`.
- Event envelope: Present via `CoreEventEnvelope` and deterministic
  `EventSequence`.
- Run inspection projection: Present via `RunInspection::from_events`, with
  sequence, run-id, and lifecycle consistency checks.
- Workflow plan skeleton: Present via `WorkflowPlan`, `WorkflowStepPlan`,
  `WorkflowStepKey`, and `WorkflowStepKind`.
- Workflow plan to run skeleton bridge: Present via
  `AgentRun::prepare_from_plan`; it is pure in-memory and does not execute
  steps, emit events, call models/tools, or persist data.
- Artifact manifest: Present via `ArtifactManifest`, `ArtifactRef`,
  `ProvenanceRecord`, `ArtifactKind`, `ReviewStatus`, and `hash_bytes`.
- Storage path safety: Present via `StorageRoot::join_logical` and rejection of
  empty, absolute, root, prefix, current-dir, and parent traversal components.
- Atomic write planning contract: Present via `AtomicWriteRequest`,
  `AtomicWritePlan`, and explicit `WriteMode`, with no actual writer.
- Prompt prefix compiler: Present and covered by tests for stable/variable
  separation and version-sensitive prefix identity.
- Hybrid model gateway types: Present via provider-neutral model messages,
  descriptors, modalities, routing decisions, privacy policy, cache policy,
  usage, and provider trait.
- Tool registry permission model: Present via tool definitions, JSON schemas,
  risk levels, permissions, calls/results, and `ToolRegistry`.
- Sandbox approval contract: Present via `SandboxPolicy`, `SandboxDecision`,
  `ApprovalRequirement`, `ExecutionPermission`, and `SandboxRunner` interface.
- Science common numerical/table contracts: Present via finite `mean`, simple
  linear regression, numeric `DataColumn`, `DataTable`, and `TableShape`.

## Phase 2 Readiness

The repository is ready to begin Phase 2 design, not Phase 2 execution by
default.

Ready pieces for `chemistry.kinetics_csv`:

- Pure numeric table input can be represented in memory by `DataTable` and
  `DataColumn`.
- Linear regression can reuse `simple_linear_regression`.
- Workflow intent can be described with `WorkflowPlan` and bridged into a
  `Created` run skeleton.
- Outputs can be described with `ArtifactManifest` and provenance records.
- Run lifecycle and inspection can audit future run progress once events are
  emitted by a real runner.
- Tool and sandbox permissions can model future file/tool/network approval
  needs before any side effects occur.
- Prompt/model interfaces can support future model-assisted planning without
  binding the kernel to DeepSeek-specific APIs.

Not ready by design:

- No CSV parser exists yet.
- No chemistry workflow pack is implemented.
- No real run executor emits events from workflow steps.
- No storage backend persists runs or artifacts.
- No CLI inspect/replay commands exist yet.

## Known Deferred Work

- Real DeepSeek API client.
- Real tool execution.
- Real sandbox runner.
- Real storage persistence.
- CSV parsing.
- Chemistry kinetics workflow.
- CLI inspect/replay commands.
- UI.
- Multimodal provider implementation.
- Plugin marketplace.

## Issues Found

No blocking issues were found.

Minor observations to carry into Phase 2:

- `deepseek-science-common::Unit` already includes `MolePerLiter`; this is a
  generic scientific unit label rather than workflow leakage, but Phase 2
  should avoid expanding common units into a chemistry-specific model.
- `ArtifactKind::Table` mentions CSV-derived tables in documentation. This is
  acceptable as an example, not a parser or workflow implementation.
- The previous foundation audit is historical and says Git was unavailable at
  that time; this audit confirmed the checkout is a Git repository with
  `origin` set to `GTC2080/DeepSeek_Science`. Branch state is time-sensitive
  and should be rechecked before publish work.

## Minimal Fixes Applied

None.

## Remaining Risks

- Phase 2 can accidentally leak chemistry concepts into `deepseek-science-core`
  or `deepseek-science-common` if workflow design starts in generic crates.
- Adding CSV parsing, persistence, provider calls, or tool execution will create
  new trust boundaries that need focused validation and disk-safety review.
- Current validation is library-focused; no CLI smoke command was run because
  this was a documentation audit and the requested validation set did not
  include `cargo run`.

## Recommended Next Step

Start Phase 2 with a small design pass for `chemistry.kinetics_csv`:

- Keep the workflow in a domain/workflow boundary outside generic crates.
- Reuse `DataTable`, `simple_linear_regression`, `WorkflowPlan`,
  `ArtifactManifest`, tool permissions, sandbox policy, and prompt/model
  contracts.
- Define the smallest CSV input contract and artifact output contract before
  implementing parsing or execution.
