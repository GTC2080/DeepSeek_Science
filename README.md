<p align="center">
  <img src="docs/assets/DeepSeek_Science.svg?v=20260702-3" alt="DeepSeek Science logo" width="260">
</p>

<h1 align="center">DeepSeek_Science</h1>

<p align="center">
  A Rust-only, headless-first Science Agent Kernel for future scientific agent workbenches.
</p>

---

## Overview

DeepSeek_Science is an early-stage kernel for building replayable, auditable,
and extensible scientific agent workflows. The project is currently focused on
the Phase 1 foundation: clean Rust crate boundaries, provider-neutral model
types, prompt-prefix caching primitives, tool protocol metadata, artifact
provenance, storage interfaces, sandbox policy, and a minimal CLI.

DeepSeek is the first intended provider family, but the model layer is designed
as a Hybrid Model Gateway. No real provider API calls are implemented yet.

## Current Status

Phase 1 is intentionally headless and lightweight.

Included now:

- Rust workspace with explicit crate boundaries.
- Domain-neutral core entities for projects, threads, runs, steps, and events.
- Provider-neutral model gateway request/response and usage types.
- DeepSeek descriptor and mock pricing placeholders.
- Prompt Prefix Compiler with stable-prefix hashing.
- Generic tool registry and permission metadata.
- Artifact manifest, hashing, review, and provenance types.
- Storage layout helpers and repository traits.
- Sandbox policy and runner interfaces.
- Minimal `deepseek-science` CLI.

Not included in Phase 1:

- UI or native workbench shell.
- TypeScript, Node, Tauri, Electron, GPUI, egui, or Slint.
- Real DeepSeek API calls.
- Database implementation.
- Python tool execution.
- Full plugin marketplace.
- Full chemistry workflow implementation.

## Workspace

| Crate | Role |
| --- | --- |
| `deepseek-science-core` | Domain-neutral IDs, projects, threads, runs, steps, states, and events. |
| `deepseek-science-model` | Provider-neutral model gateway types and usage accounting. |
| `deepseek-science-model-deepseek` | DeepSeek descriptors and mock pricing placeholders. |
| `deepseek-science-prompt` | Prompt prefix compilation and stable-prefix hashing. |
| `deepseek-science-tools` | Generic tool protocol, registry, risk, and permission metadata. |
| `deepseek-science-common` | Small pure-Rust scientific utilities. |
| `deepseek-science-artifacts` | Artifact manifests, refs, hashes, review status, and provenance. |
| `deepseek-science-storage` | Storage layout helpers and repository traits. |
| `deepseek-science-sandbox` | Sandbox policy and future runner interfaces. |
| `deepseek-science-cli` | Minimal headless CLI entry point. |

## Quick Start

```sh
cargo check --workspace
cargo test --workspace --lib
cargo run -p deepseek-science-cli -- doctor
```

Crate-specific check and test aliases are defined in `.cargo/config.toml`.

## Disk Safety

Build output is configured outside the source tree:

```text
../.cache/deepseek-science-target
```

The repository is designed to avoid uncontrolled disk churn. Generated run
output, artifact output, logs, coverage, profiling output, local agent rules,
and environment files are ignored by Git.

## Project Direction

The first future validation workflow is expected to be a small
`chemistry.kinetics_csv` vertical slice. Chemistry-specific logic must stay out
of `deepseek-science-core`; domain workflows should be built around the kernel
interfaces instead of changing the kernel into a chemistry-specific system.

Longer term, the project aims to support scientific workflows across chemistry,
physics, materials science, engineering, mathematics, bioinformatics, and other
domains.
