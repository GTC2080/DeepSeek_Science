# Phase 5.0 Deterministic Kinetics SVG Visualization RFC

## Summary

Phase 5 should add one explicit, deterministic visualization command:

```text
deepseek-science kinetics plot \
  --input <path> \
  --time-column <column> \
  --concentration-column <column> \
  --output <path.svg>
```

The command reads one existing simple numeric CSV, runs the existing kinetics
validation and analysis exactly once, prepares one chemistry-owned immutable
plot-data value, renders one fixed 960 by 640 SVG in memory, and publishes one
explicit target through the existing atomic `CreateNew` storage boundary. It
does not overwrite, create a parent directory, emit a second persistent output,
call a model, use the network, start background work, or introduce UI or browser
behavior.

The first visualization contains accepted concentration-versus-time
observations, a first-order predicted curve, a second-order predicted curve, a
fixed legend, labels derived only from the selected columns, cautious fit and
review summaries, and bounded fixed warnings. The renderer never parses CSV,
validates chemistry rows, or computes an independent fit.

This RFC is design-only. It changes no Rust source, test, manifest, lockfile,
README, fixture, version, tag, release, or generated SVG.

## Motivation

The v0.4 command can produce deterministic text and JSON analysis, but users
must mentally translate the fitted linearized models back into the original
concentration-time domain. One small, static SVG makes the accepted observations
and both candidate curves inspectable without adding a UI, interactive charting,
or a general report system.

The original concentration-time domain is the appropriate first view because it
shows the measured values in their selected columns. A transformed `ln(C)` or
`1/C` panel is useful for specialist diagnosis but is less direct and is not
required to establish the first visualization boundary.

The design must be frozen before implementation because rendering adds new
contracts around scientific wording, curve reconstruction, numeric formatting,
untrusted XML text, byte identity, resource use, and no-overwrite publication.

## Current v0.4 Kinetics Boundary

The released baseline is annotated tag `v0.4.0` at
`6996f404386abeed84514c1cce8ea32b4a413181`. The tag, `main`, `HEAD`, and
`origin/main` are aligned at that commit. A GitHub Release was intentionally not
created.

The current implementation establishes these relevant contracts:

- `parse_simple_numeric_csv` parses a narrow in-memory UTF-8, comma-delimited,
  named numeric table. Quoted and multiline fields are unsupported, and all
  parsed table values must be finite.
- `KineticsColumns` preserves exact, case-sensitive caller-selected time and
  concentration names.
- `ValidatedKineticsInput` retains accepted `KineticsPoint` values in caller row
  order and exposes them through `valid_points()`.
- The only current row-level kinetics rejection is a zero or negative
  concentration. Rejected row indices and the rejected count remain available;
  missing or invalid numeric CSV cells fail parsing rather than becoming
  rejected kinetics points.
- `KineticsFitResult` already exposes `slope`, `intercept`, `rate_constant_k`,
  `r_squared`, and `valid_point_count` for each model.
- `KineticsAnalysisResult` retains both fit results, the finite-`r_squared` MVP
  preference, the comparison basis, counts, and deterministic review. It does
  not own the accepted point vector.
- The current analysis JSON is manually mapped as `kinetics.analysis.v1`; adding
  plotting must not change that schema.
- Explicit JSON output already uses `AtomicWriteRequest`,
  `WriteMode::CreateNew`, `AtomicWritePlan::execute`, an existing parent, and
  no-overwrite publication.

The current `kinetics analyze` input read is not bounded to 16 MiB: it calls
`fs::read_to_string` directly. The 16 MiB bounded reader is currently used by
the data inspection/conversion path. Phase 5 plotting therefore cannot claim it
is reusing an existing bounded kinetics reader.

## User Story

As a user with one simple numeric kinetics CSV, I can explicitly select the time
and concentration columns and request one new SVG file. If the input analysis
and rendering succeed and the target is new, the file contains the exact
accepted observations and deterministic representations of both existing fits.
If any pre-publication step fails, no output path is touched. If the target
already exists, its bytes remain unchanged.

Completion is visible as one successful process exit and one explicit SVG
target. No JSON sidecar, report directory, cache, artifact record, project
state, or implicit output is created.

## Proposed CLI

The first command accepts exactly:

```text
--input <path>
--time-column <column>
--concentration-column <column>
--output <path.svg>
--help
-h
```

`--input`, `--time-column`, `--concentration-column`, and `--output` are all
required for execution. Each may appear once. Help succeeds only when `--help`
or `-h` is the sole plot argument, matching the existing subcommand pattern.
Unknown options, duplicate options, missing values, positional values, and
missing required options are user errors with plot-specific usage.

The command is separate from `kinetics analyze`; `--plot` is not added to the
existing command. This gives each command one persistent-output responsibility,
avoids partial success between JSON and SVG, preserves the existing analysis
surface, keeps publication to one atomic operation, and permits plot-specific
help and errors while still calling the same deterministic chemistry analysis.

The first version has no `--format`, `--png`, `--pdf`, `--html`, `--theme`,
`--color`, `--width`, `--height`, `--title`, `--force`, `--overwrite`, or
`--json`. It has no interactive or multiple-output mode.

After and only after successful publication, stdout is exactly:

```text
kinetics plot complete
```

with one trailing LF. User and publication errors use stderr and leave stdout
empty. The success line intentionally contains no input path, temporary path,
byte count, or scientific claim.

## Why SVG

SVG is the smallest format that satisfies this phase:

- it is a deterministic UTF-8 text document;
- points, lines, labels, legend, and accessible description need no rasterizer;
- it can be produced with Rust `String` and fixed XML escaping;
- it stays standalone without a browser runtime, external images, font files,
  or stylesheet;
- exact bytes and structure can be tested without screenshots;
- the existing storage layer can publish it as opaque bytes.

PNG and PDF require rendering or document dependencies and introduce
font/raster/platform variability. HTML would broaden the security and output
surface and invite JavaScript or multiple assets. None is justified for the
first chart.

## Visualization Contract

The SVG contains one concentration-versus-time plot in the original domain.
It renders:

1. first-order and second-order model curves behind the observations;
2. every accepted observation as one point, in accepted caller-row order;
3. exactly six x grid/tick positions and six y grid/tick positions;
4. fixed axis labels derived only from the two selected column names;
5. a fixed legend in this order:
   - `observed data`;
   - `first-order fit`;
   - `second-order fit`;
6. a fixed fit/review summary;
7. zero to three fixed visualization warnings.

Rejected observations are not plotted. Model lines never replace, move,
smooth, aggregate, or downsample accepted points.

The fixed canvas and layout are:

- width: `960`;
- height: `640`;
- viewBox: `0 0 960 640`;
- plot panel: left `96`, top `72`, width `624`, height `360`;
- x-axis label: centered at x `408`, baseline y `476`;
- y-axis label: centered at x `32`, y `252`, rotated `-90` degrees;
- legend and fit summary region: x `752` through `928`, y `72` through `432`;
- warning/footer region: x `96` through `928`, baselines y `536`, `568`, and
  `600`.

The fixed title is `Kinetics concentration versus time`. There is no
caller-provided title.

The fixed palette is:

- background: `#ffffff`;
- primary text and observed points: `#111827`;
- axes: `#334155`;
- grid: `#cbd5e1`;
- first-order line: `#005ea8`, width `2.5`, solid;
- second-order line: `#a23b00`, width `2.5`, dash pattern `8 5`;
- observed point radius: `3.5`.

The renderer uses only fixed attributes and a generic
`system-ui, sans-serif` font stack. It embeds no font and does not depend on
font metrics for data coordinates. Model identity is conveyed by legend text
and solid versus dashed stroke, not color alone. Observations are circles and
are rendered last within the data panel so they remain visible over a curve.

## Plot Data Contract

The preferred Phase 5.1 addition is one chemistry-owned immutable
`KineticsPlotData` value with private fields and read-only accessors. Its
checked constructor receives:

- `&ValidatedKineticsInput`;
- `&KineticsColumns`;
- `&KineticsAnalysisResult`.

CLI orchestration must maintain the caller invariant that all three values come
from the same parse, validation, and analysis flow.

It owns only the values required for deterministic rendering:

- the exact selected time and concentration names;
- accepted `(time, concentration)` values in accepted caller-row order;
- accepted and rejected counts;
- first-order slope, intercept, `r_squared`, and deterministic sampled curve;
- second-order slope, intercept, `r_squared`, and deterministic sampled curve;
- the existing MVP preference and comparison basis;
- deterministic review status and review finding count;
- a bounded ordered list of visualization-warning enums.

The constructor performs every structural consistency check available from the
current types: accepted count, rejected count, and both fits'
`valid_point_count` must agree with the corresponding validated-input and
analysis counts. These checks reject obviously inconsistent combinations, but
they do not prove data provenance or object identity when unrelated values have
the same counts. `KineticsColumns` likewise carries exact names, not source
identity. CLI orchestration is responsible for passing the validated input,
columns, and analysis created by one flow. Phase 5.1 adds no data hash,
fingerprint, provenance identifier, or refit and does not expand the existing
analysis or JSON contract.

The accepted points are not added to `KineticsAnalysisResult`. That option would
grow every analysis result and public equality contract for a visualization
used by one command. CLI reparsing or revalidation is rejected because it would
duplicate the chemistry row rules and could drift from analysis. A dedicated
plot-data constructor is the smallest boundary that preserves the current
analysis and JSON contracts.

Model prediction and curve segmentation are chemistry-specific and happen
while constructing `KineticsPlotData`. The SVG renderer receives completed
observation and curve data; it performs coordinate mapping and XML formatting
only.

## Model Curve Contract

The current fit values are sufficient to reconstruct both curves; no new fit
field and no independent regression is required.

For time `t`:

```text
first-order:  C(t) = exp(intercept + slope * t)
second-order: C(t) = 1 / (intercept + slope * t)
```

Each model is evaluated at exactly 128 candidate time positions over the
accepted observation domain. With accepted minimum `t_min`, maximum `t_max`,
and nonzero span, candidate index `i` from `0` through `127` is:

```text
t(0)   = t_min
t(127) = t_max
t(i)   = t_min + (t_max - t_min) * i / 127
```

The two endpoints are assigned directly rather than relying on the interpolated
expression. Sampling is ascending by time regardless of input row order. There
is no extrapolation, adaptive sampling, randomness, smoothing, or configurable
sample count.

For first order, a candidate is retained only when the exponent expression and
predicted concentration are finite. A finite underflowed zero is retained; it
is not replaced or raised to an arbitrary minimum.

For second order, a candidate is retained only when the denominator is finite
and strictly greater than zero and the reciprocal is finite. Zero or negative
denominators are omitted. No invalid value is clamped.

An omitted candidate ends the current contiguous curve segment. The renderer
never connects points across an invalid sample. Segments containing fewer than
two points are not drawn. If fewer than two retained points exist for the whole
model, that model line is omitted. If a model has any omitted candidates, a
fixed model-specific warning is emitted; it distinguishes `partially omitted`
from `omitted: fewer than two finite predictions`.

An invalid individual model curve is non-fatal. Accepted observations and any
other valid model remain in a completed SVG. The existing MVP preference is
still reported exactly as analysis produced it, even if that preferred curve is
omitted; the warning makes the display limitation explicit. Visualization
warnings do not mutate chemistry review status.

## Axis and Range Policy

All axis arithmetic is checked for finite results. Failure to construct a
finite axis from the accepted observations is a plot-ineligible user error and
prevents publication.

For x:

1. find the finite minimum and maximum accepted time;
2. if they are equal, fail with `accepted time values do not span a renderable range`;
3. compute `span = max - min`; a non-finite or non-positive span is fatal;
4. compute `padding = span * 0.05`;
5. use `min - padding` and `max + padding` as displayed bounds;
6. require both padded bounds and their span to remain finite and ordered.

For y, include every accepted concentration and every retained point from every
displayed model segment. A model whose otherwise finite values make the padded
y range non-representable is omitted with a fixed warning rather than making
accepted observations disappear.

If all included y values are nonnegative, the lower bound is exactly zero and
the upper bound is `max + max * 0.05`. If that maximum is zero, the upper bound
is the fixed minimum padding `1e-12`. If any included y value is negative, use
the finite actual minimum and maximum and add 5% of their span to both sides.
For a zero negative-domain span at value `v`, use
`max(abs(v) * 0.05, 1e-12)` on both sides. All resulting bounds must be finite
and ordered.

Exactly six ticks are used on each axis. Tick index `i` from `0` through `5` is
linear interpolation over the final padded bounds; endpoints are assigned
directly. Ticks are not made “nice,” rounded to units, or selected by terminal,
locale, font, or hidden scientific heuristics.

Coordinate mapping is linear:

```text
x = 96 + (time - x_min) / (x_max - x_min) * 624
y = 72 + 360 - (concentration - y_min) / (y_max - y_min) * 360
```

Every mapped coordinate must be finite. No log axis or inferred unit is used.

## Missing and Rejected Data

The simple CSV parser does not define a missing-value syntax. Empty cells,
invalid floats, non-finite values, inconsistent rows, quoted fields, or other
unsupported CSV structure fail before kinetics validation and produce no SVG.

After parsing, the existing chemistry validation accepts finite time values and
strictly positive concentrations. Zero and negative concentrations are rejected
exactly as they are today. The plot shows only `valid_points()` and displays the
exact rejected-row count. It does not expose rejected row indices, values, raw
rows, or arbitrary input content.

No row is repaired, interpolated, imputed, clamped, sorted out of the accepted
observation list, or silently removed by the renderer. Duplicate time values
remain separate observed points. If all accepted times are identical, analysis
or the explicit plot-domain check fails without output.

## Scientific Wording and Labels

The SVG must not claim a confirmed reaction order, proven model, correct model,
validated mechanism, or final scientific selection.

The fixed visible labels are:

- `observed data`;
- `first-order fit`;
- `second-order fit`;
- `first-order r_squared: <value>`;
- `second-order r_squared: <value>`;
- `MVP heuristic preference: first-order` or
  `MVP heuristic preference: second-order`;
- `deterministic review status: passed`, `passed with warnings`, or `failed`;
- `accepted observations: <count>`;
- `rejected rows: <count>`;
- `visualization warnings: <count>`.

The current analysis always selects first or second order, with an exact tie
preferring first order. The renderer does not invent a `none` preference. If a
future analysis contract can explicitly return no preference, the fixed wording
may extend to `MVP heuristic preference: none` without implying a model choice.

Only fixed visualization warnings are shown. Existing arbitrary review-finding
messages are not copied into the SVG. The review status and finding count remain
visible without expanding the untrusted or layout surface.

## SVG Byte Contract

Every successful SVG is:

- valid UTF-8;
- without a BOM;
- standalone;
- without an XML declaration;
- LF-only;
- terminated by exactly one LF;
- emitted in fixed element order and manually fixed attribute order;
- free of timestamps, random identifiers, UUIDs, absolute paths, temporary
  paths, environment comments, and generator metadata.

The exact first line is:

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 960 640" width="960" height="640" role="img" aria-labelledby="plot-title plot-desc" font-family="system-ui, sans-serif">
```

The document then emits, in order: fixed `<title>` and `<desc>` elements, white
background, plot background and grid, axes and tick labels, first-order
segments, second-order segments, observed circles, axis labels, legend,
fit/review summary, warnings, and closing `</svg>`. Within repeated groups,
ticks are ascending, curve segments are model then time order, observations are
accepted row order, and warnings are rejected-row, first-order, then
second-order order.

The renderer may emit only `<svg>`, `<title>`, `<desc>`, `<g>`, `<line>`,
`<polyline>`, `<circle>`, `<text>`, and `<rect>`. It does not emit scripts,
stylesheets, images, `foreignObject`, font files, external URLs, data URIs,
animation, filters, or comments.

One shared finite-number formatter is used for coordinates, ticks, and visible
fit metrics:

- reject non-finite input;
- normalize numeric and rounded negative zero to `0`;
- use fixed notation for zero and magnitudes in `[1e-4, 1e6)`;
- use scientific notation for nonzero magnitudes below `1e-4` or at least
  `1e6`;
- round to at most six fractional digits;
- trim trailing fractional zeroes and a trailing decimal point;
- use lowercase `e`, an explicit exponent sign, and no exponent leading zeroes;
- never use locale grouping or a locale decimal separator.

Thus representative spellings are `0`, `1.25`, `-0.0001`, `1e-5`, and
`1e+6`. XML coordinates never carry more than six fractional digits.

## Determinism

For the same input bytes, exact selected columns, program build, and supported
runtime platform, repeated rendering must produce byte-identical SVG. The
contract fixes sampling count and order, ranges, tick count, number formatting,
layout, colors, line styles, labels, warning order, element order, attributes,
line endings, and final newline.

The implementation must use no current time, random source, process id, UUID,
hash-derived id, terminal measurement, locale, environment path, filesystem
enumeration order, model output, network response, or font measurement.

The current fitting and standard-library `f64::exp` boundary does not establish
a formal cross-architecture bit-for-bit transcendental guarantee. Six-digit
serialization deliberately bounds visible precision, and Phase 5.4 must test
repeated byte identity on each supported release target. This RFC does not
justify a new deterministic-math dependency solely to strengthen an unproven
cross-platform edge case.

## Accessibility

The root has `role="img"` and fixed `aria-labelledby` references. The document
contains:

- `<title id="plot-title">Kinetics concentration versus time</title>`;
- `<desc id="plot-desc">` containing the bounded selected labels, accepted and
  rejected counts, both `r_squared` values, the cautious MVP preference, review
  status, and visualization-warning count.

The visible legend repeats the model names. Observed points, a solid
first-order line, and a dashed second-order line provide shape and pattern
distinctions independent of color. Text and strokes use high contrast on white.
No interaction, focus behavior, hover content, or color-only status is
required.

The first SVG is not a replacement for a machine-readable data table and does
not embed every observation in prose. Existing `kinetics analyze --json`
remains the separate structured analysis path; `kinetics plot` does not create a
JSON sidecar.

## Untrusted Text Safety

Column names and paths are untrusted. Only the selected column names may enter
the SVG, and only as bounded text content. Input and output paths, raw rows,
rejected values, parser fragments, arbitrary review messages, and temporary
paths are excluded.

Each selected column label is processed in this exact order:

1. validate Unicode scalar values;
2. reject XML 1.0-forbidden characters, all Unicode control characters, and
   U+2028/U+2029 as a user error rather than replacing them;
3. if longer than 48 Unicode scalar values, retain the first 47 and append one
   U+2026 ellipsis, for a maximum of 48 displayed scalars;
4. XML-escape `&`, `<`, `>`, `"`, and `'` as `&amp;`, `&lt;`, `&gt;`,
   `&quot;`, and `&apos;`.

The cap is measured before entity expansion and never splits UTF-8. The same
escaped bounded text is used in the axis label and accessible description. It
is never interpreted as markup or inserted into an attribute that can create a
URL, event handler, script, or external reference.

## Output Path and Atomic Publication

`--output` is mandatory. Its final extension must be valid UTF-8 and equal to
`svg` under ASCII case-insensitive comparison. `.svg` and `.SVG` are accepted;
missing or different extensions are fatal. The command does not infer, append,
or rewrite an extension.

The user-supplied input and output paths must not be lexically equal under the
existing `Path` comparison. No canonicalization, symlink-containment, or alias
equality claim is added in v1.

Publication reuses only:

- `AtomicWriteRequest`;
- `WriteMode::CreateNew`;
- `AtomicWritePlan::execute`.

The CLI resolves the explicit parent as the storage root and the final file name
as the logical target, as the existing explicit-output path does. Planning is
filesystem-free. Execution requires the parent directory to exist, creates one
deterministic create-new temporary sibling, syncs it, hard-links the new final
target without replacing, and removes the sibling. The command never calls
`std::fs::write`, implements a second atomic writer, pre-deletes a target,
creates a directory, or falls back to overwrite.

Existing-target and missing-parent errors are mapped to specific stable user
messages. Other publication errors use a conservative message that the
requested target may exist and should be inspected before retrying; the
operation-owned temporary sibling path is not exposed. The CLI does not delete
a possibly published final target as rollback.

## Failure Ordering

The command uses this exact order:

1. identify exact help or parse all required arguments;
2. reject lexical input/output equality;
3. enforce the case-insensitive `.svg` output extension;
4. open one regular input and read at most 16 MiB plus one detection byte;
5. strictly decode the bytes as UTF-8 and parse the existing simple CSV;
6. construct exact `KineticsColumns` and `ValidatedKineticsInput`;
7. run `KineticsAnalysisResult::analyze` once;
8. construct chemistry-owned `KineticsPlotData`, including fixed samples and
   warnings, without refitting;
9. render the full SVG into a size-checked in-memory `String`;
10. verify UTF-8, the 4 MiB maximum, no BOM, and exactly one trailing LF;
11. construct the atomic `CreateNew` plan;
12. call `AtomicWritePlan::execute` exactly once;
13. emit the fixed success stdout.

No output plan is executed and no output path is touched before all analysis,
plot-data, rendering, and byte checks pass. No success stdout is prepared as
process output before persistence succeeds.

## Crate Boundary Plan

Three placements were considered:

1. `deepseek-science-common`: rejected for v1 because the model equations,
   preference wording, review summary, and fixed chart are kinetics-specific.
   Putting them in common would turn a small scientific utility crate into a
   charting boundary before a second use exists.
2. `deepseek-science-chemistry`: preferred. It already owns accepted kinetics
   points, model kinds, fits, comparison, and review. One focused module can own
   `KineticsPlotData`, sampling, axis calculation, XML safety, and the first
   fixed renderer without pretending to be a general framework.
3. `deepseek-science-cli`: rejected for scientific preparation. It would make
   the CLI duplicate equations and validation rules. The CLI should only
   orchestrate and publish.

The preferred implementation keeps one focused renderer module in the existing
chemistry crate rather than creating a visualization crate. Chemistry performs
no file IO and receives no paths. The CLI owns argument parsing, bounded file
reading, exact-column binding, orchestration, suffix validation, error/success
formatting, and atomic publication. Storage remains an opaque-byte publication
boundary and requires no SVG knowledge.

A future second genuinely different scientific visualization can provide the
second caller and concrete pressure needed to extract XML escaping, axis, or SVG
primitives. No generic renderer, chart trait, registry, factory, template
system, or visualization crate is created for this one chart.

## Resource Limits

The plot command has these fixed non-configurable limits:

- input: 16 MiB, detected by reading at most the limit plus one byte;
- model candidates: exactly 128 per model, 256 total;
- displayed column labels: 48 Unicode scalar values each after truncation;
- visualization warnings: at most three fixed warning entries;
- successful SVG: at most 4 MiB.

The input must be one regular file. Accepted points are bounded by the input
size and are all retained; v1 does not downsample observations. If rendering
all accepted points would exceed 4 MiB, plotting fails with no publication.

The renderer uses checked append/projection logic so the `String` never grows
beyond the 4 MiB limit merely to discover the error. A final exact byte-length
check remains mandatory before output planning. A private small-limit seam
tests the limit branch without allocating a 4 MiB test document.

There is no streaming renderer, memory mapping, spool file, cache, async
runtime, worker, daemon, progress file, coverage data, or background task.

Because current `kinetics analyze` is unbounded, v1 applies the 16 MiB limit to
the new `kinetics plot` command only. It must not silently change analyze while
implementing an SVG feature. A separate focused compatibility decision should
later decide whether to move analyze onto the same bounded reader, with its own
help, README, and over-limit process tests.

## Dependency Policy

The preferred v0.5 implementation adds no dependency. Fixed SVG generation is
adequately and safely covered by:

- standard-library `String` construction with checked appends;
- a small explicit XML text escaper;
- checked finite coordinate and number formatting;
- existing kinetics types and standard `f64` operations;
- existing atomic storage publication.

Do not add a plotting library, XML DOM library, browser engine, headless
Chromium, image renderer, font library, JavaScript, TypeScript, UI framework,
or serialization layer for SVG. Any future dependency proposal requires a
separate justification showing why the standard-library fixed renderer is
insufficient; it is not part of the preferred Phase 5 plan.

## Testing Strategy

Phase 5.1 and 5.2 should use pure chemistry unit tests for:

- exact accepted plot points and caller-row order;
- rejected-row count preservation without raw rejected values;
- rejection of structurally inconsistent accepted, rejected, or fit-point
  counts without claiming source authentication;
- exactly 128 candidates and both exact time endpoints for a valid model;
- first-order prediction by `exp(intercept + slope * t)`;
- second-order prediction by `1 / (intercept + slope * t)`;
- invalid second-order denominator omission;
- segment breaks across invalid samples;
- partial-curve warning and fewer-than-two-points line omission;
- model omission without changing the existing heuristic preference;
- fixed x and y range calculations;
- equal-time plot-ineligible handling;
- zero-span y handling and `1e-12` minimum padding;
- exactly six deterministic ticks per axis;
- fixed/scientific numeric formatting, exponent normalization, and negative
  zero normalization;
- XML escaping of all five special characters;
- rejection of control and XML-invalid label characters;
- 48-scalar deterministic label truncation;
- fixed dimensions, viewBox, layout coordinates, colors, and stroke patterns;
- exactly one trailing LF, no BOM, and valid UTF-8;
- absence of timestamp, random id, UUID, input path, output path, temporary
  path, external URL, script, image, stylesheet, and `foreignObject` fields;
- repeated rendering byte identity;
- the 4 MiB branch through a private small-limit seam;
- cautious scientific wording and absence of definitive model claims.

Phase 5.3 should add CLI process tests entirely beneath
`CARGO_TARGET_TMPDIR` for:

- successful creation of one fresh `.svg` target;
- case-insensitive `.SVG` acceptance and missing/different suffix refusal;
- valid standalone root, title, description, fixed dimensions, and closing
  structure;
- existing-target sentinel preservation;
- missing-parent refusal without creating the parent;
- invalid CSV or analysis failure creating no SVG;
- invalid time or concentration column creating no SVG;
- untrusted label escaping and control-character refusal;
- input bytes remaining unchanged;
- publication failure leaving success stdout empty;
- success stdout appearing only on successful publication;
- no temporary sibling after successful publication or ordinary refusal;
- two distinct fresh targets receiving byte-identical SVG;
- `data inspect`, `data convert`, and `kinetics analyze` behavior remaining
  unchanged, including JSON and explicit JSON-output tests.

Tests should use structural and exact-byte assertions. The first release needs
no screenshot, rasterization, browser, or visual-regression baseline.

## Compatibility

The new command is additive. It preserves:

- `data inspect` syntax and behavior;
- `data convert` syntax and behavior;
- `kinetics analyze` arguments, text output, JSON stdout, and explicit JSON
  output-file behavior;
- the `kinetics.analysis.v1` schema and field wording;
- current fit equations, rate constants, finite-`r_squared` preference, tie
  behavior, review status, and scientific caution;
- storage `CreateNew` semantics and error safety;
- the current CLI versioning policy.

No accepted points are added to existing JSON. Plot warnings do not alter
analysis results or review findings. No artifact manifest, project record, run
record, cache, or hidden storage state is created for an explicit SVG target.

The plot-specific 16 MiB bound is new and documented. Current analyze remains
unchanged in this feature to avoid an unrelated silent compatibility change.
Version alignment to `0.5.0` occurs only after the functionality and final
audit pass.

No model call, tool execution, network, UI, browser, JavaScript, TypeScript, or
provider behavior is introduced.

## Non-goals

The following are explicitly deferred:

- PNG or PDF export;
- HTML reports;
- interactive zoom, pan, hover, or selection;
- multiple themes, custom colors, or custom dimensions;
- user titles;
- multi-dataset overlays;
- residual plots;
- confidence intervals;
- uncertainty propagation;
- error bars;
- transformed `ln(C)` or `1/C` panels;
- chart templates;
- a general visualization framework or crate;
- model-generated chart explanations;
- automatic report generation;
- UI;
- automatic column or unit inference;
- observation downsampling;
- output overwrite or force behavior;
- multiple plot files or JSON sidecars.

## Phase 5 Breakdown

### Phase 5.1: Kinetics plot-data contract

- add the checked chemistry-owned `KineticsPlotData` boundary;
- expose exact accepted points, existing fit/review metadata, deterministic
  predictions, segments, and bounded warnings;
- add no SVG formatting, CLI behavior, or file IO.

### Phase 5.2: Deterministic SVG renderer

- implement the fixed chart, XML safety, layout, numeric formatting, axis, and
  byte contracts in the existing chemistry crate;
- use pure inline unit tests;
- add no CLI or publication behavior.

### Phase 5.3: `kinetics plot` CLI

- add exact arguments and help;
- use 16 MiB bounded simple-CSV input;
- orchestrate existing validation and analysis, plot-data construction, and
  rendering;
- require one explicit `.svg` output;
- publish once with atomic `CreateNew`;
- update README and add process tests in that implementation phase.

### Phase 5.4: End-to-end visualization audit

- exercise realistic synthetic kinetics data from an audit-owned controlled
  path;
- verify repeated deterministic bytes, warnings, disk safety, no overwrite,
  no leftover sibling, and existing-command compatibility;
- create no private laboratory fixture.

### Phase 5.5: Version alignment and v0.5 release audit

- align the CLI to `0.5.0` only after all feature checks pass;
- perform the final release audit;
- create an annotated `v0.5.0` tag in a separate authorized task;
- do not create a GitHub Release unless separately requested.

## Open Questions

There is no blocking design question for Phase 5.1.

Two non-blocking follow-ups are deliberately outside the first implementation:

- whether a separately reviewed compatibility change should bound existing
  `kinetics analyze` at 16 MiB as well as plot;
- whether supported-target audits demonstrate sufficient cross-platform byte
  identity to strengthen the current same-build/runtime determinism claim.

Neither follow-up changes the fixed v1 command, scientific formulas, SVG layout,
byte contract, or no-overwrite semantics. Generic visualization extraction is
not an open question: it is deferred until a second real visualization creates
the need.

## Acceptance Criteria

- One explicit `kinetics plot` command accepts only the four required values and
  help flags defined by this RFC.
- The command uses the exact points accepted by `ValidatedKineticsInput` and the
  fit/review values from one existing `KineticsAnalysisResult`.
- CLI orchestration passes `ValidatedKineticsInput`, `KineticsColumns`, and
  `KineticsAnalysisResult` from one parse/validation/analysis flow; constructor
  count checks reject obvious mismatches but do not authenticate provenance or
  identity.
- The plot-data constructor performs no independent fit and the renderer
  performs no CSV parsing or chemistry validation.
- One successful command publishes exactly one deterministic standalone SVG.
- All accepted observations and every renderable portion of both candidate
  model curves are represented in the original concentration-time domain.
- Invalid individual curves are omitted or segmented with fixed warnings;
  invalid accepted observation domains fail without publication.
- Wording remains cautious and identifies the preference as an MVP heuristic.
- Column labels are bounded, XML-safe, and cannot inject markup, attributes,
  scripts, external references, or layout controls.
- Sampling, axis bounds, six ticks, number spelling, layout, element order, and
  styling follow the fixed contracts above.
- SVG bytes are UTF-8, BOM-free, LF-only, exactly one trailing LF, and contain
  no time, random id, UUID, environment path, or external resource.
- Input is bounded to 16 MiB, successful SVG is bounded to 4 MiB, and both
  model candidate counts are fixed at 128.
- Publication uses `AtomicWriteRequest`, `WriteMode::CreateNew`, and
  `AtomicWritePlan::execute`; existing targets are never overwritten and
  parents are never created.
- No plotting dependency, new crate, general chart framework, UI, browser,
  JavaScript, TypeScript, model, network, tool execution, or background behavior
  is added.
- Existing inspect, convert, analyze, kinetics JSON, storage, and versioning
  contracts remain unchanged.

## Recommended Next Step

Review and accept this RFC. Then implement only Phase 5.1 in a separate task:
add the minimal chemistry-owned plot-data contract and pure tests, with no SVG,
CLI, file IO, dependency, version, tag, or release change.
