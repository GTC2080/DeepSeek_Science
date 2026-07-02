# Phase 1 Foundation Audit

## Summary

The Phase 1 foundation matches the intended Rust-only, headless-first kernel
shape. The workspace contains the expected 10 crates, the core crate remains
domain-neutral, and no UI, TypeScript, networking, database, or provider API
implementation was found.

One cleanup was applied: unused workspace dependency declarations for `anyhow`
and `time` were removed from the root `Cargo.toml`. They were not used by any
crate and were not present in the active dependency tree.

## Commands Run

- `git status --short`
- `git branch --show-current`
- `git remote -v`
- `find . -maxdepth 3 -type d | sort`
- `find crates -maxdepth 2 -name Cargo.toml -print | sort`
- `cargo tree --workspace`
- `cargo tree --workspace -i wasm-bindgen`
- `cargo tree --workspace -i js-sys`
- `RUSTDOCFLAGS="-D missing_docs" cargo doc --workspace --no-deps`
- `rg` scans for UI, TypeScript, networking, database, TODO/FIXME, and unsafe patterns
- `bash -n scripts/doctor.sh`
- `bash -n scripts/clean-dev.sh`

Final validation commands were also run after the cleanup:

- `cargo fmt --check`
- `cargo check --workspace`
- `cargo test --workspace --lib`
- `cargo run -p deepseek-science-cli -- doctor`

The Git commands reported that this directory is not currently a Git repository.

## Crate Boundary Status

- `deepseek-science-core`: contains only domain-neutral IDs, project, thread,
  run, step, event, and error types. No chemistry implementation, UI
  dependency, model provider dependency, prompt compiler dependency, storage
  implementation, or sandbox dependency was found.
- `deepseek-science-model`: remains provider-neutral. It owns descriptors,
  requests, responses, usage accounting, capabilities, routing, cache policy,
  and privacy policy.
- `deepseek-science-model-deepseek`: contains only DeepSeek placeholder
  descriptors and mock pricing logic. It performs no network calls and reads no
  API keys.
- `deepseek-science-prompt`: owns deterministic prompt prefix compilation and
  prefix hashing only.
- `deepseek-science-tools`: owns generic tool definitions, calls, results,
  permissions, and registry behavior only.
- `deepseek-science-common`: contains small domain-neutral utilities only:
  mean, simple linear regression, table shape, and placeholder unit labels.
- `deepseek-science-artifacts`: owns artifact kind, manifest, hash,
  provenance, and review status metadata only.
- `deepseek-science-storage`: owns deterministic layout helpers and repository
  traits only. No SQLite or database implementation exists.
- `deepseek-science-sandbox`: owns deny-by-default policy and runner interface
  types only. It does not execute arbitrary commands.
- `deepseek-science-cli`: is the only crate with direct terminal output.

## Dependency Status

Active dependencies remain minimal:

- `serde`
- `serde_json`
- `thiserror`
- `uuid`
- `blake3`

No `reqwest`, `tokio`, `sqlx`, `rusqlite`, `clap`, GUI crate, UI framework, or
TypeScript/Node/Bun tooling appears in the active workspace dependency tree.

`Cargo.lock` contains target-specific WebAssembly packages through transitive
metadata, but `cargo tree --workspace -i wasm-bindgen` and
`cargo tree --workspace -i js-sys` show they are not active host dependencies.

## Disk Safety Status

- `.cargo/config.toml` sets `target-dir = "../.cache/deepseek-science-target"`.
- `.gitignore` ignores build output, cache, temp, run, artifact, and test-output
  directories.
- `scripts/clean-dev.sh` targets only the configured Cargo target cache, prints
  the path, rejects suspicious paths, asks for exact `DELETE`, and uses no broad
  wildcard.
- `scripts/clean-dev.sh` was inspected and syntax-checked only. It was not
  executed.
- Tests are deterministic and do not write uncontrolled files.
- The kinetics CSV fixture is tiny: 71 bytes.
- No watch loop, repeated write/delete loop, `cargo clean`, network script, or
  API call path was found.

## Code Hygiene Status

- No library `unwrap()` or `expect()` calls were found.
- No `TODO` or `FIXME` markers were found in `crates/`.
- No broad `utils` module exists.
- `RUSTDOCFLAGS="-D missing_docs" cargo doc --workspace --no-deps` passed.
- Public APIs have documentation comments.
- Comments are mostly boundary, caching, provenance, disk-safety, or permission
  oriented and do not appear noisy.

## Prompt Cache Status

`deepseek-science-prompt` has tests proving:

- Stable sections with different user requests produce the same `prefix_hash`.
- Changing a stable section produces a different `prefix_hash`.

The prompt compiler keeps stable prefix and variable tail separate.

## Usage / Cost Status

`NormalizedUsage` includes:

- `input_tokens`
- `output_tokens`
- `cache_hit_tokens`
- `cache_miss_tokens`
- `total_tokens`
- `estimated_cost_usd`
- `cache_hit_rate()`

The cache hit-rate calculation is covered by a unit test.

## Artifact Manifest Status

Artifact manifests are serializable, hash-aware, provenance-oriented, and not
tied to chemistry or UI concepts. Hash stability and JSON serialization are
covered by unit tests.

## Issues Found

- Root `Cargo.toml` had unused workspace dependency declarations for `anyhow`
  and `time`.
- The directory is not currently a Git repository, so Git diff/status evidence
  cannot be produced.

## Minimal Fixes Applied

- Removed unused `anyhow` and `time` declarations from root
  `workspace.dependencies`.

## Remaining Risks

- `uuid` brings target-specific WebAssembly metadata into `Cargo.lock`, though
  it is not an active host dependency. This is acceptable for now because
  stable unique IDs are part of the core model and `uuid` was an intended
  lightweight dependency.
- No full integration tests exist yet. This is acceptable for Phase 1 because
  the project currently exposes skeleton interfaces and small deterministic
  unit tests.

## Recommended Next Development Milestone

Start Phase 2 with the smallest headless `chemistry.kinetics_csv` vertical
slice: parse the tiny CSV fixture, keep chemistry logic outside
`deepseek-science-core`, produce an artifact manifest with provenance, and add
one deterministic CLI or library validation path.
