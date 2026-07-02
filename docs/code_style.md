# Code Style

Code in this repository should be clean, small, documented, and easy to review.

## Boundaries

- Keep crate boundaries explicit.
- Keep modules purposeful.
- Do not create broad `utils` modules.
- Do not leak domain-specific code into `deepseek-science-core`.
- Do not add UI code in Phase 1.

## Dependencies

- Prefer the standard library.
- Reuse existing workspace dependencies.
- Do not add a dependency to save a few lines of ordinary code.
- Heavy tools belong in future feature-gated crates or separate integrations.

## Comments and Documentation

- Use `//!` module docs for crates and important modules.
- Use `///` docs for public structs, enums, traits, and functions.
- Use inline comments only for invariants, safety assumptions, cache behavior,
  provenance, disk protection, permission boundaries, or non-obvious design.
- Do not add obvious comments.

## Simplicity

- Avoid clever abstractions without a real second use.
- Avoid speculative configuration and extension points.
- Avoid giant files.
- Avoid messy TODO blocks.
- Keep placeholder code calm, explicit, and compileable.

## Tests

- Tests must be deterministic.
- Tests must not hit the network.
- Tests must not require API keys.
- Tests must not write uncontrolled files.
- Fixtures must be tiny.
