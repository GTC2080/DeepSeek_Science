# Contributing

Thank you for helping improve DeepSeek_Science. This project is currently in
Phase 1, so contributions should keep the kernel small, Rust-first,
headless-first, and easy to audit.

## Phase 1 Boundaries

Phase 1 accepts work related to the headless Rust kernel, CLI, provider-neutral
model types, DeepSeek placeholders, prompt prefix compilation, tool metadata,
artifact provenance, storage traits, sandbox policies, and small pure-Rust
scientific utilities.

Phase 1 does not include UI frameworks, TypeScript, Node, Bun, web server
frameworks, real DeepSeek API calls, database implementations, Python tool
execution, or a full plugin marketplace.

## Commit Format

Use short, conventional commit messages:

```text
<type>(<scope>): <summary>
```

The scope is optional when the change is repository-wide:

```text
docs: add contribution templates
```

Recommended types:

| Type | Use for |
| --- | --- |
| `feat` | User-visible or public API behavior |
| `fix` | Bug fixes |
| `docs` | Documentation-only changes |
| `test` | Tests and fixtures |
| `refactor` | Internal restructuring without behavior changes |
| `perf` | Performance improvements |
| `chore` | Maintenance changes |
| `ci` | Continuous integration configuration |
| `build` | Build configuration |

Recommended scopes:

| Scope | Area |
| --- | --- |
| `core` | `deepseek-science-core` |
| `model` | `deepseek-science-model` |
| `deepseek` | `deepseek-science-model-deepseek` |
| `prompt` | `deepseek-science-prompt` |
| `tools` | `deepseek-science-tools` |
| `common` | `deepseek-science-common` |
| `artifacts` | `deepseek-science-artifacts` |
| `storage` | `deepseek-science-storage` |
| `sandbox` | `deepseek-science-sandbox` |
| `cli` | `deepseek-science-cli` |
| `docs` | Documentation |
| `packs` | Domain or workflow pack placeholders |

Examples:

```text
docs: add issue templates
fix(prompt): keep user request out of prefix hash
feat(cli): add doctor summary line
chore(repo): update ignore rules
```

Keep commits focused. Do not combine unrelated refactors, dependency changes,
or generated output with functional changes.

## Pull Request Format

Use the repository pull request template. A good PR should explain:

- What changed.
- Why the change is needed.
- Which crates or docs are affected.
- Which Phase 1 boundaries were considered.
- Which validation commands were run.
- Whether any generated files, high-write commands, or deletion happened.

Small documentation-only PRs do not need cargo validation by default.

## Issue Format

Use the bug report or feature request template when possible.

For bugs, include:

- A clear summary.
- Exact reproduction steps.
- Expected and actual behavior.
- Relevant environment details.
- Logs or screenshots with secrets removed.

For feature requests, include:

- The user goal.
- The smallest useful behavior.
- Non-goals.
- Expected crate or API impact.
- Disk, build, dependency, and validation impact.

## Validation

Choose the smallest meaningful validation for the change.

Recommended commands:

```sh
cargo fmt --check
cargo check -p <changed-crate>
cargo test -p <changed-crate> --lib
```

For cross-crate public API changes, run the workspace checks once:

```sh
cargo fmt --check
cargo check --workspace
cargo test --workspace --lib
```

For documentation-only changes, prefer:

```sh
git diff --check
git diff --stat
git status --short
```

## Disk Safety

Disk safety is a first-class rule for this repository.

Do not run `cargo clean`, watch processes, repeated full-workspace validation,
coverage generation, profiling, benchmark output, or broad deletion commands
unless the maintainer explicitly requests it.

Do not commit generated build output, caches, logs, local environment files, or
large fixtures.
