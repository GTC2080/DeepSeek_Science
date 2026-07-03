# Phase 2.1 Kinetics CSV Design

## Summary

`chemistry.kinetics_csv` is the first Phase 2 scientific workflow design. The
first implementation should be deterministic, in-memory, reviewable, and
replay-friendly. It should analyze a small numeric `DataTable`, fit simple
linearized kinetic models, and return structured results plus reviewer warnings.

The name `kinetics_csv` refers to the expected future user-facing data source.
The initial workflow API boundary is an in-memory `DataTable`; CSV parsing is a
later adapter and must not be part of the first implementation.

## User Goal

Given time-series concentration data, the user wants a small, reproducible
kinetics analysis that can:

- validate the input table,
- fit first-order and second-order linearized models,
- compare those fits with a deterministic MVP heuristic,
- report fitted rate constants and warnings,
- leave enough structure for future artifact, provenance, and review systems.

## Minimal Deliverable

- A deterministic workflow function or module that accepts a `DataTable`.
- Exact conceptual inputs: `time` and `concentration`.
- First-order and second-order linearized regression only.
- A structured in-memory result containing per-model fit results, comparison
  summary, rejected row indices, and reviewer warnings.
- No file reads, file writes, persistence, model calls, tool execution, plotting,
  or CSV parsing.

## Non-goals

- CSV parsing or CSV fixtures.
- Nonlinear fitting.
- Confidence intervals.
- AIC or other rigorous model-selection criteria.
- Residual diagnostics.
- Weighted fitting.
- Uncertainty analysis.
- Plotting.
- Storage persistence.
- Real artifact file creation.
- Model or DeepSeek calls.
- Tool execution.
- UI or TypeScript.
- New dependencies.

## Existing Phase 1 Building Blocks

| Area | Phase 1 building block | Phase 2.1 use |
| --- | --- | --- |
| Table input | `DataTable`, `DataColumn` | In-memory workflow boundary |
| Numeric fit | `simple_linear_regression` | Linearized model fitting |
| Workflow shape | `WorkflowPlan`, `WorkflowStepPlan` | Future plan description only |
| Artifacts | `ArtifactManifest`, `ArtifactKind`, `ReviewStatus` | Future metadata mapping |
| Provenance | `ProvenanceRecord`, `hash_bytes` | Future audit trail |
| Tool boundary | `ToolRegistry`, permissions, risk levels | Future adapters only |
| Storage | storage layout and write plans | Deferred; no writes in MVP |
| Prompt/model | cache-aware model request and prompt contracts | Future explanation layer only |

## Input Data Contract

The first implementation accepts an already-validated `DataTable`.

- The workflow must not read CSV files.
- The workflow must not parse CSV text.
- The workflow must not infer files from paths.
- Column length validation and finite numeric values are handled by
  `DataTable` and `DataColumn`.
- Units are not typed yet. Unit information, if present, is only a column naming
  convention such as a caller-supplied label or future adapter mapping.

## Expected Columns

| Concept | Required | Notes |
| --- | --- | --- |
| `time` | Yes | Independent variable for both fits |
| `concentration` | Yes | Dependent variable before transformation |

The first implementation should use explicit logical column names selected by
the caller or adapter. A future CSV adapter may map source names such as
`time_s` or `concentration_mol_l` into these logical concepts before calling the
workflow.

## Validation Rules

- Missing `time` column: fail with a structured workflow validation error.
- Missing `concentration` column: fail with a structured workflow validation
  error.
- Mismatched column lengths: already rejected by `DataTable`.
- Non-finite values: already rejected by `DataColumn` and `DataTable`.
- Non-positive concentration values: reject those rows for both first-order and
  second-order transforms.
- Rejected row indices are stored internally as 0-based indices into the input
  `DataTable`.
- Future user-facing reports may convert rejected row indices into
  human-readable row numbers.
- If fewer than two valid paired rows remain after rejection, the fit must fail
  with a structured validation error.
- Zero time variance must fail with a structured validation error.
- Any invalid regression input from `simple_linear_regression` must be surfaced
  clearly rather than hidden.

## Kinetic Models

| Model | Transform | Rate constant |
| --- | --- | --- |
| First-order | `ln(concentration)` vs `time` | `k = -slope` |
| Second-order | `1 / concentration` vs `time` | `k = slope` |

Only these two deterministic linearized models are in scope for the first MVP.

## Linearization Strategy

1. Read `time` and `concentration` values from the input `DataTable`.
2. Reject rows where `concentration <= 0.0`.
3. Build paired valid arrays:
   - `x = time`
   - first-order `y = ln(concentration)`
   - second-order `y = 1.0 / concentration`
4. Run `simple_linear_regression(x, y)` independently for each model.
5. Derive `k` from the model-specific slope convention.
6. Preserve rejected row indices and validation warnings in the result.

The transform stage is chemistry workflow logic and must not be placed in
generic kernel crates.

## Model Comparison Strategy

For the Phase 2 MVP, compare first-order and second-order fits by finite
`r_squared` only. This is a deterministic heuristic for a small first workflow,
not a final rigorous statistical model-selection method.

The workflow must avoid overstating the selected model as scientifically
definitive. The result should say the selected model has the higher finite
`r_squared` among the two MVP linearized fits.

Deferred:

- AIC.
- Confidence intervals.
- Residual diagnostics.
- Weighted fitting.
- Nonlinear fitting.
- Uncertainty analysis.
- Plotting.

## Output Artifacts

The first implementation should return in-memory structured output only.
Conceptual outputs:

- result table or structured result with model name, slope, intercept, `k`, and
  `r_squared`,
- model comparison summary,
- rejected row report,
- reviewer warnings.

Future artifact mapping may represent these as:

- `ArtifactKind::Table` for structured fit results,
- `ArtifactKind::Report` or `ArtifactKind::Json` for comparison summaries,
- `ReviewStatus::Passed`, `PassedWithWarnings`, or `Failed` after deterministic
  validation.

No real artifact persistence is required for the first implementation.

## Review / Validator Rules

The deterministic reviewer should check:

- each reported `k` is derived from the corresponding regression slope,
- every reported `r_squared` is finite,
- model transform assumptions were applied only to valid concentration rows,
- rejected row indices and counts are reported,
- invalid inputs are visible in structured errors or warnings,
- the comparison summary does not claim scientific certainty beyond the MVP
  heuristic.

Reviewer logic must not call a model.

## Workflow Plan Steps

A future `WorkflowPlan` can describe the workflow with stable, domain-neutral
step kinds:

| Step key | Kind | Purpose |
| --- | --- | --- |
| `inspect_input` | `InspectInput` | Confirm required table columns and shape |
| `validate_rows` | `Custom` | Apply chemistry-specific row validation |
| `fit_models` | `Custom` | Run deterministic linearized fits |
| `compare_models` | `Review` | Select by finite `r_squared` heuristic |
| `produce_results` | `ProduceArtifact` | Return structured in-memory outputs |
| `complete` | `Complete` | End workflow |

The plan describes intent only. It must not execute tools, call models, or write
files.

## Crate Boundary Plan

Future chemistry-specific implementation should live outside generic crates:

- preferred: a dedicated domain crate such as `deepseek-science-chemistry`,
- acceptable for an early step: a clearly separated domain workflow module if a
  new crate is not yet justified.

Explicit prohibitions:

- no chemistry-specific logic in `deepseek-science-core`,
- no chemistry-specific workflow logic in `deepseek-science-common`,
- no CSV parsing in `deepseek-science-common`,
- no model-specific logic in the deterministic workflow core.

`deepseek-science-common` may remain limited to truly generic reusable numeric
helpers, such as regression or table primitives, when they are useful outside
chemistry.

## Disk Safety Considerations

The first implementation is memory-only.

- No file reads.
- No CSV file IO.
- No generated files.
- No storage persistence.
- No artifact file writes.
- No uncontrolled temp files.
- No watch process or background loop.

Future CSV adapters must pass through explicit file, storage, tool, and sandbox
review before they read or write project data.

## Model / DeepSeek Usage

DeepSeek or other model calls are not required for the first deterministic
implementation.

Future model usage may summarize or explain already-reviewed deterministic
results. If added later, it must remain outside the deterministic workflow core
and use existing cache-aware prompt/model contracts, including stable prefix
identity and privacy policy.

## Testing Plan

Use tiny in-memory `DataTable` fixtures only.

Minimum test scenarios:

- valid first-order-like table returns finite first-order fit,
- valid second-order-like table returns finite second-order fit,
- missing `time` column fails,
- missing `concentration` column fails,
- non-positive concentration rows are rejected for both transforms,
- fewer than two valid rows after rejection fails,
- zero time variance fails,
- reviewer rejects non-finite `r_squared` or inconsistent `k`.

Do not add CSV fixtures unless a later CSV adapter phase explicitly introduces
CSV parsing.

## Implementation Milestones

1. Define domain-level request/result/error shapes for the in-memory workflow.
2. Implement column lookup and row validation over `DataTable`.
3. Implement first-order and second-order transforms.
4. Reuse `simple_linear_regression` for both fits.
5. Build deterministic comparison and reviewer warnings.
6. Add tiny in-memory tests.
7. Map results to future artifact metadata only after persistence requirements
   are explicit.
8. Add CSV parsing as a separate adapter milestone, not as part of the first
   workflow core.

## Open Questions

- Should the first Rust implementation use a new `deepseek-science-chemistry`
  crate immediately, or start as a clearly isolated domain module until a second
  chemistry workflow exists?
- Should logical column names be passed explicitly by the caller, or should the
  first API require exact `time` and `concentration` names?
- What structured error enum should the chemistry workflow expose, and should it
  wrap `CommonError` directly or translate it into domain validation errors?
- When persistence is added later, which artifact shape should be canonical:
  table, JSON, report, or a bundle of all three?
