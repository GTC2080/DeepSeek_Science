# Development

## Rust-only Policy

This repository is Rust-only in Phase 1. Do not add TypeScript, Node, Bun, npm,
Tauri, Electron, GPUI, egui, Slint, or other UI framework dependencies.

## Checks

```sh
cargo fmt --check
cargo check --workspace
cargo test --workspace --lib
cargo run -p deepseek-science-cli -- doctor
```

Useful aliases from `.cargo/config.toml`:

```sh
cargo ccore
cargo cmodel
cargo cprompt
cargo ctools
cargo tcore
cargo tmodel
cargo tprompt
cargo ttools
```

## External Target Directory

Cargo is configured to write build artifacts to:

```text
../.cache/deepseek-science-target
```

This keeps generated build output outside the source tree and reduces accidental
Git noise.

## Crate Dependencies

- `deepseek-science-core` must stay domain-neutral.
- `deepseek-science-core` must not depend on UI, model providers, or chemistry.
- Provider crates may depend on `deepseek-science-model`.
- Storage implementations may depend on core and artifact metadata.
- CLI may depend on all crates for lightweight orchestration and diagnostics.
- Heavy tools must wait behind future feature flags or separate crates.

## Generated Files

Generated files must go under ignored temp or output directories such as:

- `tmp/`
- `runs/tmp/`
- `artifacts/tmp/`
- `test-output/`

Do not write generated output into source directories, docs, crates, packs,
fixtures, or scripts.

## Destructive Scripts

Scripts must not perform uncontrolled deletion. Any script that removes files
must:

- target a known project cache or output directory,
- print the exact target path,
- reject empty, root, home, `.`, `..`, or suspicious paths,
- ask for typed confirmation,
- avoid broad wildcards.

`scripts/clean-dev.sh` is limited to the configured Cargo target cache.
