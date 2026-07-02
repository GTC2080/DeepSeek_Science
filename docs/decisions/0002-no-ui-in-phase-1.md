# ADR 0002: No UI in Phase 1

## Status

Accepted.

## Context

UI choices can force premature architecture decisions and heavy dependencies.

## Decision

Phase 1 ships only headless kernel crates and a CLI.

## Consequences

- Product behavior is validated through tests and CLI diagnostics.
- Native UI exploration is deferred until kernel boundaries are stable.
- UI dependencies are not allowed in the Phase 1 workspace.
