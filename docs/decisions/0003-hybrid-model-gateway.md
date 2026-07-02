# ADR 0003: Hybrid Model Gateway

## Status

Accepted.

## Context

DeepSeek is the first intended provider, but scientific workflows may need
local models, remote models, multimodal models, and provider-specific cache
accounting.

## Decision

The model layer defines provider-neutral descriptors, requests, responses,
capabilities, routing decisions, privacy policy, cache policy, and normalized
usage. Provider crates adapt concrete providers into this shape.

## Consequences

- `deepseek-science-core` has no provider dependency.
- DeepSeek integration can evolve without changing core run entities.
- Cache accounting is normalized for future audit and cost analysis.
