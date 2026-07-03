# Phase 2.9 Kinetics Artifact Mapping Design

## Summary

This design defines how a future `KineticsAnalysisResult` should map to generic
artifact metadata without creating files or writing storage. The boundary is:

```text
KineticsAnalysisResult -> in-memory artifact manifest proposal
```

The proposal is metadata and canonical content identity only. Persistence,
artifact files, and storage paths remain deferred.

## User Goal

After deterministic kinetics analysis, callers need a reviewable artifact-shaped
record that can later enter the generic artifact and storage layers. The record
should preserve fitted values, comparison basis, reviewer status, and provenance
without leaking chemistry logic into generic crates.

## Non-goals

- No artifact persistence.
- No generated report files.
- No storage writes or logical path allocation.
- No CSV parsing or file IO.
- No plotting.
- No model, tool, or natural-language report generation.
- No chemistry-specific `ArtifactKind` variants.
- No changes to Rust source, Cargo metadata, or crate dependencies in this
  design task.

## Existing Building Blocks

| Area | Existing type | Relevant role |
| --- | --- | --- |
| Chemistry result | `KineticsAnalysisResult` | Source structured analysis result |
| Workflow identity | `CHEMISTRY_KINETICS_CSV_WORKFLOW_ID` | Stable workflow id |
| Workflow step | `produce_analysis_result` | Likely producing step key |
| Artifacts | `ArtifactManifest` | Generic metadata record |
| Artifact kind | `ArtifactKind` | Generic artifact classification |
| Review | `ReviewStatus` | Generic review state |
| Provenance | `ProvenanceRecord` | Generic audit link metadata |
| Hashing | `hash_bytes` | BLAKE3 hash over canonical bytes |

## Mapping Boundary

The first implementation should introduce only an in-memory mapping concept:

- input: a completed `KineticsAnalysisResult`,
- output: an artifact manifest proposal,
- no file path,
- no artifact file,
- no storage write,
- no ledger append.

The proposal may later be converted into `ArtifactManifest` at a future
persistence boundary. If deterministic tests need stable equality, the proposal
should avoid allocating a random `ArtifactId`; artifact ids can be assigned by a
later artifact registry or accepted from the caller.

## Proposed Artifact Shape

Recommended proposal fields:

| Field | Source | Notes |
| --- | --- | --- |
| `kind` | fixed | Prefer `ArtifactKind::Json` |
| `title` | fixed | `"Chemistry kinetics analysis result"` |
| `content_hash` | canonical result bytes | Deterministic BLAKE3 |
| `input_hashes` | optional | Empty until input table hashing exists |
| `provenance` | workflow/run context | In-memory metadata only |
| `review_status` | `KineticsReviewStatus` | Mapped to generic `ReviewStatus` |

The proposal should not store the original `DataTable`, CSV paths, local
absolute paths, storage paths, timestamps, or random ids.

## Artifact Kind Choice

| Option | Assessment |
| --- | --- |
| `Json` | Best first choice: structured, machine-readable, can represent fits, comparison, and review findings together. |
| `Table` | Useful for fit rows, but too narrow for comparison basis and review findings. |
| `Report` | Better for future human-readable summaries, not for the deterministic MVP. |
| `Text` | Too weakly structured for replay and validation. |
| `Unknown` | Avoid when the shape is known. |

Use `ArtifactKind::Json` first. Do not add variants such as `KineticsResult`,
`KineticsReport`, or `ReactionOrder`.

## Content Hash Strategy

The content hash should be computed over a future canonical representation of
the deterministic result content. It must not use debug formatting, local paths,
timestamps, random ids, `HashMap` iteration order, or model-generated prose.

Canonical content should include, in a fixed field order:

- valid point count,
- rejected row count,
- first-order fit: slope, intercept, `rate_constant_k`, `r_squared`,
  `valid_point_count`,
- second-order fit: slope, intercept, `rate_constant_k`, `r_squared`,
  `valid_point_count`,
- comparison basis,
- preferred model under the MVP heuristic,
- review status,
- review findings in recorded order: severity, check kind, optional model kind,
  optional rejected row count, stable message.

Before real hashing is implemented, the mapping phase should define exact
canonical serialization. If current chemistry types are not serializable, either
add a tiny canonical projection type in the chemistry crate later or write a
small deterministic serializer there. Do not hash `Debug` output.

## Provenance Strategy

The provenance record should be generic and path-free.

Recommended fields for the MVP:

- workflow id: `chemistry.kinetics_csv`,
- optional run id when an `AgentRun` is available,
- optional workflow step key: `produce_analysis_result`,
- source data boundary: `DataTable`,
- tool call id: `None`,
- model call id: `None`,
- prompt prefix hash: `None`,
- source artifact ids: empty until upstream artifacts exist.

Current `ProvenanceRecord` does not have dedicated workflow fields. The first
mapping can use a stable note such as:

```text
workflow=chemistry.kinetics_csv;step=produce_analysis_result;source=DataTable
```

If stronger structured provenance is needed later, add only domain-neutral
fields to the artifact crate. Do not add chemistry-specific provenance fields.

## Review Status Mapping

| Chemistry status | Generic artifact status |
| --- | --- |
| `KineticsReviewStatus::Passed` | `ReviewStatus::Passed` |
| `KineticsReviewStatus::PassedWithWarnings` | `ReviewStatus::PassedWithWarnings` |
| `KineticsReviewStatus::Failed` | `ReviewStatus::Failed` |
| no review available | `ReviewStatus::NotReviewed` |

`KineticsAnalysisResult` should normally include a deterministic review, so
`NotReviewed` is only for partial or future non-analysis proposals.

## Crate Boundary Plan

- `deepseek-science-chemistry` owns the chemistry-specific mapping from
  `KineticsAnalysisResult`.
- `deepseek-science-chemistry` may depend on `deepseek-science-artifacts` in a
  later implementation phase for a thin mapping function.
- `deepseek-science-artifacts` must not depend on chemistry.
- `deepseek-science-core` must remain chemistry-neutral.
- Do not move kinetics mapping logic into the generic artifact crate.
- Do not add chemistry-specific artifact kinds or provenance concepts.

## Disk Safety Considerations

The mapping is in-memory only.

- No storage writes.
- No artifact files.
- No generated reports.
- No CSV or path reads.
- No local absolute paths in metadata.
- No temporary directories.
- Persistence is deferred to a storage phase.

Future persistence must pass only logical paths through storage safety
contracts before any write is planned.

## Testing Strategy

Future tests should be tiny and in-memory:

- build a small `DataTable`,
- validate and analyze kinetics input,
- create the in-memory manifest proposal,
- assert `ArtifactKind::Json`,
- assert deterministic content hash stability,
- assert no paths, timestamps, or random ids are included in hashed content,
- assert review status mapping,
- assert provenance records workflow id and `DataTable` source boundary,
- avoid file IO, temp dirs, network, API keys, models, and tools.

If the implementation later converts to `ArtifactManifest`, tests should avoid
including random `ArtifactId` in content-hash assertions.

## Deferred Work

- Actual artifact persistence.
- JSONL ledger records.
- Storage writes and logical path allocation.
- CLI export.
- Human-readable report generation.
- Plotting and figure artifacts.
- Model-generated explanations.
- CSV parsing.
- Input table hashing.
- Canonical serialization implementation if not included in the next coding
  phase.

## Recommended Implementation Milestones

1. Add a chemistry-side in-memory proposal type and review-status mapping.
2. Add deterministic canonical content projection and hash tests.
3. Add optional provenance inputs for run id and workflow step key.
4. Convert the proposal to generic `ArtifactManifest` only when artifact id
   assignment and persistence boundaries are clear.
5. Add storage integration in a later phase using logical paths and disk-safety
   contracts.

## Open Questions

- Should artifact id assignment happen in an artifact registry, storage layer,
  or caller-provided boundary?
- Should `ProvenanceRecord` gain domain-neutral workflow id and workflow step
  fields instead of encoding them in `note`?
- Should input `DataTable` hashing become part of `deepseek-science-common` or
  remain a workflow-specific canonicalization step?
- Should the first persisted representation be a single JSON artifact or split
  later into a fit table plus a summary artifact?
