# DeepSeek_Science Kernel RFC

## Vision

DeepSeek_Science is a Rust-only Science Agent Kernel for future scientific
agent workbenches. The kernel should support replayable, auditable, low-churn
scientific workflows before any native UI exists.

## Phase 1 Goals

- Establish a clean Rust workspace.
- Define domain-neutral core entities.
- Define provider-neutral model gateway types.
- Compile stable prompt prefixes separately from variable user requests.
- Define tool metadata, permissions, and registry behavior.
- Define artifact manifests and provenance records.
- Define storage traits without choosing a database yet.
- Define sandbox policies without executing arbitrary code.
- Provide a minimal headless CLI with a `doctor` command.
- Document code style, disk safety, and architecture decisions.

## Phase 1 Non-goals

- No UI.
- No TypeScript, Node, Tauri, Electron, GPUI, egui, or Slint.
- No DeepSeek network client.
- No API key loading.
- No chemistry implementation in the core crate.
- No full plugin marketplace.
- No database engine integration.
- No long-running watcher or daemon.

## Core Design Principles

- Core is domain-neutral.
- UI is optional and future-facing.
- Model providers are adapters behind a neutral gateway.
- Tool execution must be permission-aware.
- Artifacts must be traceable.
- Prompt prefixes must be cache-friendly and deterministic.
- Disk writes must be controlled, small, and explicit.

## Crate Boundaries

- `deepseek-science-core`: projects, threads, runs, steps, events, and IDs.
- `deepseek-science-model`: provider-neutral model requests and responses.
- `deepseek-science-model-deepseek`: DeepSeek descriptors and mock pricing only.
- `deepseek-science-prompt`: prompt prefix compiler and prefix hashes.
- `deepseek-science-tools`: tool definitions, calls, results, and permissions.
- `deepseek-science-common`: small pure-Rust shared scientific helpers.
- `deepseek-science-artifacts`: artifact manifests, hashes, and review status.
- `deepseek-science-storage`: repository traits and deterministic layouts.
- `deepseek-science-sandbox`: policy boundary for future execution.
- `deepseek-science-cli`: minimal headless command surface.

## Core Entity Model

The kernel starts with `Project`, `Thread`, `AgentRun`, `RunStep`, `RunState`,
and `CoreEvent`. These types are intentionally small so domain workflows can
compose them without changing the core model.

## Hybrid Model Gateway

The model gateway normalizes descriptors, messages, cache policy, privacy
policy, responses, and usage. DeepSeek is the first intended provider, but the
gateway must be able to route to future local, remote, and multimodal providers.

## Prompt Prefix Compiler

The compiler separates stable sections from variable user requests. The stable
prefix gets a BLAKE3 hash so long-context scientific instructions can be cached
and audited without allowing per-run user text to perturb the prefix hash.

## Tool Registry

Tools are described by name, JSON schemas, output schemas, and permission
metadata. Phase 1 does not execute tools. Future runners must pass through
approval and sandbox boundaries before side effects occur.

## Workflow Engine

The workflow engine is future work. Phase 1 only creates the kernel types that a
workflow engine will record against: runs, steps, tool metadata, events, and
artifacts.

## Artifact / Provenance Ledger

Artifacts carry kind, path, content hash, review status, and provenance records.
The ledger must support replay and review without trusting file names alone.

## Reviewer / Validator System

Review and validation are future layers over artifacts and runs. Phase 1
includes review status in artifact metadata but does not implement reviewers.

## Permission / Approval Policy

Risk metadata exists at the tool layer, and deny-by-default execution policy
exists at the sandbox layer. Network, subprocess, destructive writes, and
project-external file access require future explicit approval paths.

## Storage Strategy

Phase 1 defines repository traits and deterministic directory layout helpers.
Database implementations are intentionally deferred until access patterns are
clear.

## Domain Pack / Workflow Pack

Domain packs live outside the core crate. They may provide prompts, workflow
definitions, validators, and tool bindings in later phases. The initial
chemistry pack is a placeholder and is not wired into the kernel.

## First Future Vertical Workflow: chemistry.kinetics_csv

The first future vertical is a small kinetics CSV workflow. It should prove the
kernel can ingest a tiny table, generate analysis steps, produce artifacts, and
record provenance without hard-coding chemistry into `deepseek-science-core`.

## Replay / Audit Strategy

Runs emit ordered steps and core events. Prompt prefixes, model usage, tool
calls, and artifacts should be recorded in future storage backends so a run can
be inspected after execution.

## Event System

`CoreEvent` is a compact domain-neutral event enum. It is suitable for future
logs, projections, and UI updates without taking a dependency on UI code.

## Code Cleanliness Policy

Code must be small, documented, explicit, and boring. Public APIs need useful
documentation. New abstractions require real pressure, not speculative future
needs.

## Disk Safety Policy

Build output goes to `../.cache/deepseek-science-target`. Generated run,
artifact, and test output must stay in ignored temp/output directories. Scripts
that delete anything must print the target and require typed confirmation.

## Testing Strategy

Tests should be deterministic, local, small, and focused. They must not hit the
network, require API keys, depend on current time, or create uncontrolled files.

## Build Artifact Management

The workspace uses `.cargo/config.toml` to keep Cargo build artifacts outside
the source tree. `cargo clean` is not run automatically.

## Future UI Strategy

A native UI may be explored after the headless kernel proves its contracts. The
future UI must depend on the kernel, not the other way around.
