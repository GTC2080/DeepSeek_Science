# ADR 0001: Rust-only Headless Core

## Status

Accepted.

## Context

The project needs a stable kernel before UI, provider SDK, or domain workflow
complexity enters the repository.

## Decision

Phase 1 is Rust-only and headless-first. The workspace contains Rust crates and
a minimal CLI only.

## Consequences

- No TypeScript, Node, Tauri, Electron, GPUI, egui, or Slint in Phase 1.
- Core contracts can be tested without UI state.
- Future UI shells must depend on the kernel rather than shape it.
