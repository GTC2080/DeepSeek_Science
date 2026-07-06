# v0.2 Roadmap RFC

## Summary

After the `v0.1.0` CLI MVP tag, the next release should improve the existing
headless user path before adding model/API integration.

Recommended primary direction for v0.2:

> Focus on CLI usability and structured output first.

The primary v0.2 deliverable is deterministic structured JSON output for the
existing `deepseek-science kinetics analyze` command. Other CLI usability work
is secondary and should not block v0.2.

This keeps the project deterministic, disk-safe, and reviewable while making the
current `kinetics analyze` workflow more useful for real data and downstream
automation.

## Current v0.1.0 Scope

`v0.1.0` provides one user-facing CLI MVP:

```sh
deepseek-science kinetics analyze \
  --input <path> \
  --time-column <column> \
  --concentration-column <column>
```

Implemented behavior:

- reads exactly one user-provided CSV file in the CLI layer,
- parses a tiny UTF-8 comma-separated numeric CSV subset into `DataTable`,
- validates explicit time and concentration columns,
- computes deterministic first-order and second-order linearized fits,
- compares them with the MVP finite `r_squared` heuristic,
- runs deterministic reviewer checks,
- prints a concise plain-text summary,
- writes no output files.

## What v0.1.0 Deliberately Excludes

- DeepSeek/API calls.
- Model-generated explanations.
- Tool execution.
- Artifact persistence.
- Storage/project workspace records.
- JSON output.
- Plotting.
- Full CSV dialect support.
- Automatic column detection.
- Units parsing.
- UI or TypeScript.
- Notebook, Jupyter, R, PubMed, or HPC integrations.

## Primary v0.2 Deliverable

The primary v0.2 deliverable is deterministic structured JSON output for the
existing `deepseek-science kinetics analyze` command.

First target interface:

```sh
deepseek-science kinetics analyze ... --json
```

JSON output requirements:

- Success JSON is written to stdout.
- Errors remain on stderr.
- Include a stable schema version, for example `kinetics.analysis.v1`.
- Include only deterministic values from the existing analysis pipeline.
- Include no timestamps, random IDs, local absolute paths, storage paths, or
  model-generated prose.
- Never emit `NaN` or infinity.
- Keep wording scientifically cautious.
- Label the preferred model only as the MVP finite `r_squared` heuristic result.

Other CLI usability improvements are secondary and should not block v0.2.

## v0.2 Direction Comparison

| Direction | Value | Risk | Recommendation |
| --- | --- | --- | --- |
| CLI usability and structured output | Makes current workflow usable, testable, and scriptable | Low | Primary v0.2 focus; JSON output is blocking, other CLI polish is secondary |
| Artifact persistence | Starts durable audit trail | Medium: storage contracts and disk writes need care | Defer until output contract is stable |
| DeepSeek API integration | Adds explanatory layer | High: secrets, network, cost, privacy, non-determinism | Defer or keep design-only |
| More chemistry workflows | Expands domain coverage | Medium: broadens scope before first workflow is polished | Defer until kinetics UX is solid |

## v0.2 Goals

- Keep the existing deterministic kinetics pipeline.
- Add deterministic structured JSON output as the blocking v0.2 deliverable.
- Improve command-line usability without adding a CLI framework unless clearly
  justified.
- Add structured output suitable for scripts and future artifact mapping.
- Make failure messages easier to act on.
- Preserve disk safety: no implicit output files or storage writes.
- Keep scientific wording cautious and non-definitive.

## v0.2 Non-goals

- No DeepSeek network client as the primary release goal.
- No API key loading by default.
- No artifact persistence or storage writes by default.
- No UI, TypeScript, web server, notebook, or plotting implementation.
- No full CSV engine.
- No broad chemistry workflow expansion.
- No dependency additions unless standard library and current dependencies are
  clearly insufficient.

## Proposed Features

### 1. Structured Output

Add an explicit structured-output mode, likely:

```sh
deepseek-science kinetics analyze ... --json
```

Initial JSON should mirror existing deterministic data only:

- input path as provided,
- selected column names,
- valid/rejected row counts,
- first-order fit result,
- second-order fit result,
- comparison basis,
- preferred model by MVP heuristic,
- review status,
- review findings.

Do not include model-generated prose, timestamps, random IDs, local absolute
derived paths, or storage paths.

### 2. CLI Usability

Possible small improvements:

- `deepseek-science kinetics analyze --help`.
- clearer usage text for missing or unknown arguments,
- stable error categories in stderr,
- optional `--format text|json` only if simpler than a standalone `--json`,
- documented exit-code behavior.

Keep the parser small. Do not add `clap` unless the command surface becomes
large enough to justify it.

These are secondary polish items. They should not block v0.2 if deterministic
JSON output lands cleanly first.

### 3. CSV Input Ergonomics

Real lab data may arrive as UTF-16, TSV, or instrument-specific exports. v0.2
can either:

- document a conversion path to the v0.1 CSV subset, or
- design a small import adapter phase.

Implementation should avoid growing the generic parser into a broad CSV/TSV
engine without real tests and a clear boundary.

These real lab data adapters are not part of the primary v0.2 deliverable unless
deterministic JSON output lands cleanly first.

### 4. Review Visibility

Improve visibility of deterministic reviewer findings:

- rejected row count,
- rejected row indices,
- warning status,
- comparison-basis caution.

Keep this deterministic and non-prose-heavy.

## Disk Safety Considerations

v0.2 should preserve the v0.1 disk profile:

- CLI reads one explicit user-provided input file.
- No output file is written unless a future flag explicitly requests it.
- No storage records are created by default.
- No logs, caches, temp directories, generated reports, or artifact files.
- Cargo output remains outside the source tree.
- Tests should use tiny fixtures and no temp directories unless a future output
  feature explicitly requires them.

If an output-file flag is later proposed, it should receive a separate design
review with path-safety rules.

## Dependency Policy

Default: no new dependencies.

Preferred order:

1. standard library,
2. existing workspace crates,
3. existing dependencies,
4. a new dependency only after a narrow RFC explains why it is necessary.

Do not add:

- `clap` for the current small command surface,
- `reqwest` or async runtime for v0.2 CLI usability,
- database/storage dependencies,
- UI or web dependencies,
- broad CSV dependencies unless full dialect support becomes an explicit goal.

## Testing Plan

Add focused tests around user-visible CLI behavior:

- text output remains stable and cautious,
- JSON output is valid and deterministic,
- JSON output contains no timestamps, random IDs, storage paths, or model output,
- invalid CSV remains a user error,
- missing selected columns remain user errors,
- help/usage output is stable,
- no file output is created,
- tests use tiny project-controlled fixtures.

For structured output, prefer deterministic assertions on parsed fields rather
than full snapshot blobs.

## Why DeepSeek API Should Remain Deferred or Limited

DeepSeek integration should not be the primary v0.2 goal because it introduces:

- network behavior,
- API keys and secret handling,
- cost and rate-limit concerns,
- non-deterministic output,
- privacy review requirements,
- cache-policy and provenance questions.

The current workflow already proves useful deterministic computation. v0.2
should make that deterministic result easier to consume before adding model
explanations.

If any model work appears in v0.2, it should be limited to design documents or a
disabled/off-by-default boundary. It should not be required for `kinetics
analyze` success.

## Suggested Phase Breakdown

### Phase 2.21: CLI Structured Output Design

- Define text vs JSON behavior.
- Define stable JSON fields.
- Define non-goals and disk-safety rules.

### Phase 2.22: CLI JSON Output Contract

- Implement deterministic `--json` output.
- Reuse existing analysis result.
- Add process-level smoke tests.

### Phase 2.23: CLI Help and Error UX

- Improve help/usage output.
- Keep manual parser unless complexity justifies a dependency.
- Add failure-path process tests.

### Phase 2.24: Real Lab Data Adapter Design

- Decide whether UTF-16/TSV/instrument exports are documented conversion steps
  or first-class adapters.
- Keep chemistry analysis separate from import parsing.

### Phase 2.25: Artifact Persistence Design Revisit

- Revisit persistence after structured output stabilizes.
- Keep storage writes explicitly opt-in and path-safe.

## Recommended Next Step

Start v0.2 with CLI structured output design, then implement JSON output only
after the schema and disk-safety constraints are agreed.
