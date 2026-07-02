# Architecture

DeepSeek_Science starts as a Rust core with a headless CLI. The kernel is split
into small crates so future UI, provider, and domain code can be added without
polluting core contracts.

The core crate defines projects, threads, runs, steps, states, events, and
typed identifiers. It contains no chemistry, UI, or model-provider code.

The model gateway crate defines provider-neutral messages, descriptors, usage,
privacy policy, cache policy, capabilities, and routing decisions. Provider
crates adapt concrete providers into that neutral shape. The DeepSeek crate is
currently only a descriptor and mock-pricing placeholder.

The prompt crate compiles stable prompt sections into a deterministic prefix and
hashes that prefix. Variable user requests are appended separately so cache
keys remain stable.

The tools crate records tool definitions, JSON schemas, call payloads, result
payloads, and risk metadata. The sandbox crate defines deny-by-default policy
for future execution.

The artifacts crate records manifests, content hashes, review status, and
provenance. The storage crate defines deterministic layout helpers and
repository traits without selecting a database.

Domain packs live under `packs/`. They provide future prompts and workflow
configuration while keeping domain-specific code out of the kernel. A future UI
shell can consume the same core events, storage records, and artifacts without
becoming part of the core.
