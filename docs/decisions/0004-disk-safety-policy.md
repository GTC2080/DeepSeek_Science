# ADR 0004: Disk Safety Policy

## Status

Accepted.

## Context

Scientific agent workflows can generate large artifacts, logs, caches, and
intermediate files. Uncontrolled writes make repositories noisy and expensive to
maintain.

## Decision

Build artifacts live outside the source tree. Generated output must go to
ignored temp or output directories. Deletion scripts must require explicit typed
confirmation and target only known safe cache locations.

## Consequences

- `.cargo/config.toml` sets an external target directory.
- `.gitignore` reserves temp and artifact output locations.
- `scripts/clean-dev.sh` refuses suspicious delete targets and asks for
  `DELETE` before removing the Cargo target cache.
