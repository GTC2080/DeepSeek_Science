# Roadmap

## Phase 1: Headless Kernel

- Rust workspace.
- Domain-neutral core entities.
- Provider-neutral model gateway.
- Prompt prefix compiler.
- Tool registry and permission metadata.
- Artifact manifests and provenance.
- Storage traits.
- Sandbox policy.
- Minimal CLI.
- Documentation and disk safety policy.

## Phase 2: Chemistry Kinetics Workflow MVP

- Wire `chemistry.kinetics_csv` as the first vertical workflow.
- Parse tiny CSV inputs with controlled fixtures.
- Produce reviewed artifacts with provenance.
- Keep chemistry logic outside the core crate.

## Phase 3: Hybrid Model Providers and Multimodal Evidence Pipeline

- Add real provider adapters.
- Add provider-side cache accounting.
- Add local and remote model routing.
- Add multimodal artifact references and validation.

## Phase 4: Native UI Spike

- Explore a native workbench shell after the kernel is proven.
- Keep UI crates separate from core.
- Preserve headless CLI workflows.

## Phase 5: Multi-domain Expansion

- Add additional domain packs for physics, materials science, engineering,
  mathematics, bioinformatics, and other scientific workflows.
