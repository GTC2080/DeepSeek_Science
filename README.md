# DeepSeek_Science

DeepSeek_Science is a Rust-only, headless-first Science Agent Kernel for a
future native scientific agent workbench.

Phase 1 focuses on the kernel foundation only: crate boundaries, replayable
core types, provider-neutral model interfaces, prompt-prefix compilation, tool
metadata, artifact provenance, storage traits, sandbox policy, and a minimal
CLI.

## Phase 1 Scope

- Rust only.
- Headless core and CLI only.
- No UI in Phase 1.
- No TypeScript in Phase 1.
- No GPUI, Tauri, Electron, egui, Slint, or browser shell in Phase 1.
- No real DeepSeek API calls yet.
- No plugin marketplace.
- No hard-coded science domain logic in the core crate.

DeepSeek is the first intended provider, but the architecture is designed
around a Hybrid Model Gateway. The first future validation workflow is expected
to be `chemistry.kinetics_csv`, while the core remains domain-neutral.

Code cleanliness and disk safety are first-class project rules. Generated
output must not churn inside source directories, and the Cargo target directory
is configured outside the repository source tree.

## Basic Commands

```sh
cargo check --workspace
cargo test --workspace --lib
cargo run -p deepseek-science-cli -- doctor
```

For crate-specific checks, see `.cargo/config.toml` aliases and
`DEVELOPMENT.md`.
