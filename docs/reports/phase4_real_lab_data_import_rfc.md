# Phase 4.0 Real Laboratory Data Import RFC

## Summary

Phase 4 should introduce a small, deterministic inspection boundary for real
laboratory and instrument-exported tabular data. The first user-facing command
should be:

```sh
deepseek-science data inspect --input <path>
```

The command should read one explicit file, inspect its bytes and decoded table
shape within a fixed resource limit, and report observations without changing
the file or interpreting its chemistry. It should identify a supported BOM and
encoding, assess comma or tab separation, report bounded table dimensions and
header candidates, classify the likely generic table shape, and state whether
the file is compatible with the current kinetics workflow.

Inspection comes before conversion. A later, separately reviewed command may
normalize only an unambiguous supported input:

```sh
deepseek-science data convert \
  --input <path> \
  --output <path>
```

Conversion must remain explicit, reuse the existing atomic `CreateNew` output
boundary, and never overwrite a target by default. This RFC does not implement
inspection or conversion, change `kinetics analyze`, add a crate or dependency,
or claim support for every laboratory export format.

## Motivation

v0.3 deliberately proves a narrow deterministic path: one simple UTF-8 numeric
CSV becomes an in-memory `DataTable`, the user binds exact time and
concentration columns, and chemistry performs deterministic analysis. Real
laboratory exports commonly differ before their scientific meaning is even
considered. They may contain a BOM, UTF-16 text, tabs, metadata preambles,
separate unit rows, or a wavelength-by-measurement matrix.

Sending those files directly to the current parser produces errors that do not
separate encoding, delimiter, shape, and chemistry compatibility. Broadening
the parser into a general CSV engine would also mix trust-boundary inspection
with the intentionally small numeric adapter.

The next safe step is therefore not automatic import. It is a read-only command
that tells the user what the program can observe, what remains ambiguous, and
which explicit transformation would be required before current analysis can be
used.

## Current v0.3 Input Boundary

The current v0.3 path supports:

- one input path supplied explicitly with `--input`;
- UTF-8 text read by the CLI with `std::fs::read_to_string`;
- one non-empty header row;
- comma-separated fields;
- trimmed header and numeric cells;
- finite numeric data rows only;
- equal field counts across rows;
- an in-memory numeric `DataTable` with equal-length columns;
- exact, case-sensitive time and concentration column selection supplied by
  the user;
- deterministic kinetics validation, first-order and second-order fits,
  comparison, and review;
- text or JSON stdout, plus an optional explicit deterministic JSON output
  file through atomic `CreateNew` persistence.

The current adapter intentionally does not read files itself, inspect bytes,
strip a BOM, decode UTF-16, detect delimiters, skip metadata, interpret a unit
row, support quoted or multiline fields, infer column semantics, or classify
matrices. `kinetics analyze` reads the complete file as a `String`; v0.3 does
not yet define an explicit input-size limit for that path.

Without `--output`, current kinetics analysis remains no-write. With
`--output`, only the deterministic JSON result is written; the input parsing
boundary is unchanged.

## Real Laboratory Data Examples

The inspection design must account for, without promising first-phase import
support for all of them:

- UTF-8 text without a BOM;
- UTF-8 text beginning with `EF BB BF`;
- UTF-16 little-endian text beginning with `FF FE`;
- UTF-16 big-endian text beginning with `FE FF`;
- comma-separated numeric tables;
- tab-separated numeric tables;
- metadata lines before a possible header;
- separate header and unit rows;
- a narrow table with several measurement columns;
- a wavelength matrix with one wavelength axis and multiple measurements;
- instrument labels such as `Wavelength(nm)`, `kinetics 4(Abs)`, and
  `kinetics 5(Abs)`.

For example, a decoded export might resemble:

```text
Instrument: Example Reader
Exported: 2026-07-10
Wavelength(nm)\tkinetics 4(Abs)\tkinetics 5(Abs)
200\t0.031\t0.029
201\t0.034\t0.032
```

Another export might place units on a second nonnumeric row:

```text
Time,Concentration,Temperature
s,mol/L,C
0,1.0,25
1,0.8,25
```

The first example is not automatically time/concentration data. The second is
not accepted by the current numeric CSV parser because the unit row is not
numeric. Phase 4 inspection should expose those facts; it must not silently
remove rows, rename columns, or reinterpret measurements.

## User Goals

A user should be able to:

- point to one laboratory export and learn whether it is supported text;
- see the observed BOM and likely encoding without a model call;
- see whether comma or tab is the only deterministic delimiter candidate;
- see row and column counts, candidate header rows, and numeric-cell evidence;
- distinguish a likely narrow numeric table from a likely numeric matrix;
- understand whether the file can be used by the current kinetics workflow;
- receive a concise reason when the input is ambiguous or unsupported;
- inspect without creating files, project state, artifacts, caches, or logs;
- later opt into one explicit, no-overwrite normalization when that conversion
  is fully specified and safe.

## Primary Phase 4 Direction

Phase 4 should follow an inspection-first sequence:

1. inspect a bounded byte input and identify only explicitly supported BOM and
   encoding cases;
2. decode without replacement characters or other lossy behavior;
3. inspect comma and tab candidates using a bounded deterministic rule;
4. report table structure and header candidates without rewriting input;
5. classify generic shape independently of chemistry;
6. assess current kinetics compatibility as a separate conservative result;
7. defer normalization until a dedicated conversion RFC defines exact input,
   output, row-selection, escaping, and atomic-write behavior.

No stage should call a model. The same input bytes and fixed inspection policy
must produce the same findings.

## Non-goals

Phase 4 inspection does not include:

- full CSV standards compliance;
- quoted multiline fields;
- general quoted-field parsing or emission;
- semicolon auto-detection;
- arbitrary delimiter inference;
- locale-dependent decimal parsing;
- Excel files;
- proprietary binary instrument formats;
- UTF-16 detection without a BOM;
- recursive directory import;
- watch mode, background indexing, or daemon behavior;
- automatic chemistry interpretation;
- automatic time, concentration, wavelength, unit, or other column semantic
  inference;
- automatic reaction-order inference during inspection;
- model or LLM detection of encoding, delimiter, header, shape, or columns;
- project databases, run records, or implicit artifact persistence;
- an artifact browser or UI;
- TypeScript or frontend tooling;
- Jupyter, R, PubMed, or HPC integration.

## Trust and Safety Principles

Laboratory files are untrusted input. Inspection should follow these rules:

- **Explicit source:** read exactly one caller-provided input path.
- **Bounded work:** reject input beyond the fixed v0.4 inspection policy before
  unbounded allocation or scanning.
- **Lossless decoding:** supported text must decode exactly; replacement
  characters are not an acceptable recovery path.
- **Conservative ambiguity:** report or reject ambiguity instead of guessing.
- **Deterministic rules:** byte, delimiter, header, numeric, and shape findings
  use fixed local rules only.
- **Observation is not interpretation:** labels are preserved and displayed,
  not assigned scientific meaning.
- **No side effects:** inspection writes no output or hidden state.
- **Separation of concerns:** generic inspection does not live in chemistry,
  while chemistry compatibility does not alter generic classification.
- **No silent repair:** metadata, unit rows, malformed fields, and inconsistent
  widths are not dropped to make the table appear valid.

Future model assistance may explain an already produced deterministic report,
but it is deferred, off by default, and must never replace the inspection
result.

## Proposed CLI Commands

### `data inspect`

Initial command:

```sh
deepseek-science data inspect --input <path>
```

The first command surface should accept only one required `--input` and help.
It should reject duplicate options, unexpected positional values, and unknown
flags using the existing concise CLI style.

The report should use a stable order and conceptually include:

- observed BOM: none, UTF-8, UTF-16LE, or UTF-16BE;
- likely encoding within the supported scope;
- likely delimiter: comma, tab, ambiguous, or unsupported;
- decoded physical line count and usable table-row count;
- likely column count, or a reason it cannot be established;
- one or more header candidates identified by physical line number;
- whether all, some, or none of the candidate data cells appear finite numeric;
- likely table shape;
- current kinetics compatibility and concise reasons;
- any bounded-inspection limit that prevented a complete result.

The initial output should be human-readable. A versioned JSON inspection schema
is deferred until the fields and semantics have implementation evidence.

Inspection must not:

- modify or normalize the input;
- create a converted output;
- call DeepSeek or another model;
- infer reaction order;
- silently choose chemistry columns;
- run kinetics analysis;
- write project state, run records, artifacts, logs, caches, or temporary files.

### Future `data convert`

A later command may be introduced only after its separate RFC is accepted:

```sh
deepseek-science data convert \
  --input <path> \
  --output <path>
```

It should normalize only a supported, unambiguous source to the deliberately
narrow UTF-8 comma-separated form. The conversion RFC must decide which
encoding, delimiter, header, or row-selection decisions require explicit
options. Ambiguous inputs must be refused rather than resolved heuristically.

### Existing `kinetics analyze`

The responsibilities and arguments of `kinetics analyze` remain unchanged.
It continues to require exact time and concentration column names and the
current simple numeric CSV input. Inspection does not feed hidden selections
or state into a later analysis command.

## Encoding Inspection

The minimal first encoding scope should be:

| Observed prefix | Encoding result | First-phase behavior |
| --- | --- | --- |
| `EF BB BF` | UTF-8 with BOM | Strip one BOM for inspection and report it |
| `FF FE` | UTF-16LE with BOM | Decode little-endian code units exactly |
| `FE FF` | UTF-16BE with BOM | Decode big-endian code units exactly |
| no BOM + valid UTF-8 | UTF-8 without BOM | Decode exactly |
| no BOM + invalid UTF-8 | ambiguous/unsupported | Do not guess UTF-16 |

Known unsupported BOMs, including UTF-32 signatures, should be rejected as an
unsupported encoding rather than misclassified as UTF-16. BOM-free byte input
with NUL patterns or other binary evidence must not be promoted to text merely
because a heuristic appears plausible.

The standard library is sufficient for the first scope:

- `str::from_utf8` or `String::from_utf8` validates UTF-8;
- `u16::from_le_bytes` and `u16::from_be_bytes` construct UTF-16 code units;
- `char::decode_utf16` rejects unpaired surrogates when handled strictly;
- an odd UTF-16 payload length is an error.

Errors should carry a byte offset. Where a valid decoded prefix makes it
practical, they should also carry decoded line and column context. The CLI
should not promise a line/column position when decoding failed before that
position can be established reliably.

## BOM Handling

BOM handling is a byte-boundary concern and must occur before delimiter or
header inspection.

- Report the exact recognized BOM kind.
- Remove exactly one recognized BOM from the decoded content supplied to table
  inspection.
- Do not include a UTF-8 BOM in the first header cell.
- Require a BOM for UTF-16 in the first implementation.
- Reject conflicting, truncated, repeated, or unsupported BOM sequences when
  they cannot be interpreted unambiguously.
- Do not rewrite the source or claim that BOM removal has converted the file.
- Preserve byte offsets relative to the original input when reporting errors.

## Delimiter Inspection

The first delimiter scope should contain exactly two candidates:

- comma;
- tab.

Inspection should evaluate both candidates over the decoded, bounded input
using a fixed rule. A candidate is plausible only when a stable multi-field
width is present across a usable row region. Empty lines and pre-header
metadata may be reported, but must not be silently folded into the table.
Consistent evidence for only one candidate selects it. Evidence for both or
neither produces an ambiguous or unsupported result.

The algorithm must be deterministic and bounded by the same input policy. It
must not try semicolons, inspect locale settings, learn arbitrary delimiters,
or repeatedly rescan without a fixed bound. Quotes must be detected before
simple splitting. Any input requiring RFC 4180 quoting, escaped delimiters, or
multiline fields is unsupported in this phase.

## Table Shape Inspection

After strict decoding and delimiter selection, inspection should describe
physical structure without constructing the current `DataTable` prematurely.
The structural report should include:

- total decoded lines within the accepted file;
- blank and nonblank line counts;
- the candidate tabular region;
- field count per usable row and the stable candidate width;
- inconsistent-width line numbers;
- candidate header line numbers;
- possible pre-header metadata line numbers;
- numeric-cell counts based on finite `f64` parsing;
- rows that appear to be unit/header rows rather than numeric data;
- complete versus partial inspection status.

A conservative header candidate is a nonnumeric row adjacent to a stable
rectangular numeric region. Multiple candidates should remain multiple; the
inspector must not choose one merely to make a later parser succeed. Labels
such as `Wavelength(nm)` and `kinetics 4(Abs)` are reported verbatim. Their
words and units do not establish chemistry roles.

If metadata or a unit row prevents an unambiguous single-header numeric table,
inspection should still report the observed structure where possible, but it
must not claim that current parsing or future conversion is safe.

## Narrow Table vs Matrix Classification

The generic shape result should stay deliberately small:

- `NumericNarrowTable`: a stable named-column region with a finite numeric
  body that does not satisfy the conservative matrix rule;
- `NumericMatrix`: a stable finite numeric rectangle with a leading axis-like
  column position and multiple peer measurement columns indicated by repeated
  syntactic header structure;
- `MixedOrUnsupported`: mixed numeric/text body, inconsistent widths,
  unsupported quoting, ambiguous header/table region, or other unsupported
  structure;
- `Empty`: no usable nonblank table rows.

These are layout classifications, not scientific data types. The matrix rule
must use only deterministic structure and header syntax; it must not contain
chemistry vocabulary or infer what an axis measures. If the rule cannot
distinguish a matrix confidently, the result should be
`MixedOrUnsupported` rather than a guessed narrow table.

The classifier should also return evidence and reasons, not only an enum. That
evidence may include stable dimensions, numeric ratios, header candidates, and
the repeated sibling-label pattern. It must not return inferred time,
concentration, absorbance, or wavelength bindings.

## Compatibility With Kinetics Analysis

Kinetics compatibility is a separate assessment over generic inspection
findings:

- `compatible`: the file already satisfies the current UTF-8, comma, one-header,
  finite numeric-table boundary and presents one unambiguous pair-shaped narrow
  table; the user must still provide both exact column names;
- `potentially compatible after explicit column selection`: a current-parser-
  compatible narrow numeric table has more than two candidate numeric columns,
  so the user may explicitly select two, but inspection makes no semantic
  recommendation;
- `incompatible with current kinetics workflow`: encoding, delimiter,
  metadata/unit rows, quoting, inconsistent shape, nonnumeric cells, empty
  content, or a matrix prevents safe use as-is.

An additional reason may state that an otherwise numeric UTF-16 or TSV input
could be reassessed only after a future explicit conversion. That does not make
the current file compatible.

Every `NumericMatrix`, including a wavelength matrix, should be incompatible
with the current kinetics workflow. The inspector must never reinterpret
`Wavelength(nm)` as time, or an absorbance measurement as concentration. Any
future domain adapter for spectroscopic kinetics requires its own explicit
scientific contract.

## Explicit Conversion Boundary

Conversion is not part of the first inspection implementation. Its later RFC
should preserve these minimum rules:

- require one explicit input and one explicit output path;
- reject identical input and output targets;
- accept only encodings and delimiters supported by deterministic inspection;
- refuse ambiguous encoding, delimiter, header, metadata, or table-region
  decisions;
- produce normalized UTF-8 comma-separated text only for a narrow, explicitly
  supported table subset;
- preserve labels and numeric text rather than changing units or scientific
  meaning;
- never select chemistry columns or run analysis;
- never overwrite an existing target by default;
- require the output parent directory to exist;
- reuse the existing storage `AtomicWriteRequest`, `WriteMode::CreateNew`, and
  atomic executor rather than duplicating write mechanics in CLI;
- create no project state, artifact manifest, run record, cache, or log.

TSV-to-CSV conversion must reject cells containing commas, quotes, embedded
newlines, or other content that would require quoting under the deliberately
narrow output format. It must not emit an invalid simple CSV or quietly add a
general quoting engine. Removing metadata or choosing a header row is a data
transformation and therefore requires an explicit, separately specified user
choice; inspection findings alone are not permission to discard rows.

The normalized bytes should be fully prepared and size-checked before the
single atomic publication request, unless the future storage contract gains a
reviewed bounded streaming interface. An output-expansion limit is required
because UTF-16-to-UTF-8 and delimiter normalization do not have identical byte
sizes.

## Crate Boundary Plan

Generic inspection must remain outside chemistry, and filesystem reading must
remain in the CLI. The main placement options are:

| Option | Advantages | Risks | Recommendation |
| --- | --- | --- | --- |
| Extend `deepseek-science-common` with separate inspection modules | Reuses the existing pure, file-IO-free numeric CSV/table boundary; no new crate or dependency | `common` could become a junk drawer if import policy grows | Preferred for Phase 4.1-4.3 while APIs remain small and cohesive |
| Add `deepseek-science-data` or `deepseek-science-import` | Gives explicit ownership to strict byte/text decoding and deterministic delimited-table inspection | Adds a workspace crate and dependency edge before implementation pressure proves it | Use only if both responsibilities become stable, substantial public boundaries |
| Put all inspection logic in CLI | Keeps the initial call site local | Couples pure rules to argument/output code, reduces unit-test reuse, and invites chemistry-specific branching | Do not use for the pure inspection algorithms |

The preferred Ponytail path is:

1. leave `parse_simple_numeric_csv` unchanged so it remains the current narrow
   adapter;
2. add, in a future implementation, separately named pure modules for strict
   byte/text inspection and delimited-table shape inspection inside
   `deepseek-science-common` if they remain small;
3. keep bounded file opening/reading, CLI argument parsing, report formatting,
   and future conversion orchestration in `deepseek-science-cli`;
4. keep kinetics validation and analysis in
   `deepseek-science-chemistry`, unchanged;
5. reuse `deepseek-science-storage` only for future explicit conversion output.

A dedicated data crate becomes justified only when it owns at least two
coherent responsibilities: strict lossless encoding/BOM decoding and reusable
deterministic tabular inspection/classification. It should then remain pure and
file-IO-free. A crate created only to hold one parser or one enum would be
speculative and should not be added.

Neither `deepseek-science-core` nor chemistry should gain encoding, delimiter,
matrix, or import-policy variants.

## Error Model

Future implementation should use structured internal categories and map them
to concise actionable CLI messages:

- `InputReadFailed`: the explicit file could not be opened or read;
- `InspectionLimitExceeded`: the file exceeded the fixed v0.4 policy or grew
  beyond it while being read;
- `UnsupportedOrAmbiguousEncoding`: no supported unambiguous encoding exists;
- `InvalidUtf8`: UTF-8 validation failed;
- `InvalidUtf16`: odd bytes or invalid surrogate structure was found;
- `UnsupportedBinaryInput`: bytes indicate binary or an unsupported BOM;
- `NoUsableRows`: no table region can be inspected;
- `InconsistentFieldCount`: the candidate table has conflicting widths;
- `UnsupportedQuotedOrMultilineInput`: correct parsing would require the
  deferred CSV features;
- `DelimiterAmbiguity`: comma versus tab cannot be selected deterministically;
- `TableShapeUnsupported`: the structure is mixed or cannot be classified
  safely;
- `KineticsWorkflowIncompatible`: generic inspection succeeded but the current
  kinetics contract is not met.

Read and decoding failures are fatal and should produce nonzero exit status,
empty stdout, and a concise stderr message. Successfully inspected but empty,
ambiguous, unsupported, or kinetics-incompatible structure is useful inspection
information: it should appear in the deterministic report with an unsupported
status and reason, rather than being disguised as a successful import. The CLI
exit-status convention for those nonfatal findings should be frozen alongside
the exact output contract in Phase 4.3.

Messages may include the user-provided path, byte offset, and practical
line/column context. They should not print raw file contents, debug dumps,
backtraces, unrelated local paths, or lossy-decoded text.

## Disk Safety

`data inspect` must:

- read exactly one explicit file;
- write no files;
- create no converted output;
- create no temporary file or directory;
- create no cache, log, project record, run record, artifact, or workspace;
- avoid the system temporary directory;
- perform no recursive scan, directory import, watch mode, or background work.

The v0.4 implementation should use a fixed, documented, conservative maximum
input size selected after reviewing a small project-controlled inventory of
real export sizes. This RFC intentionally does not invent a numeric ceiling
without repository or user requirements. The fixed limit should be low enough
for bounded in-memory decoding and shape inspection, checked before allocation
where metadata is available, and enforced again while reading by allowing at
most the limit plus one detection byte. File metadata alone is not sufficient
because a file can change during reading.

The first release should not expose a configurable limit. A future explicit
configuration may be considered only when real files demonstrate that one
fixed bound is insufficient. Files above the bound should fail with an
actionable message; they should never trigger partial unmarked classification.

Future conversion may create one bounded sibling temporary file and one final
output only through the existing atomic `CreateNew` boundary. It must not
create parent directories, overwrite a target, delete an input, scan siblings,
or clean unrelated paths. Input and normalized output sizes both require fixed
bounds.

## Dependency Policy

Default: no new dependencies.

The Rust standard library is sufficient for the first design:

- prefix comparisons for BOM inspection;
- built-in UTF-8 validation;
- endian-aware `u16` construction and strict UTF-16 decoding;
- `str::lines` or an equivalent explicit line iterator;
- comma/tab splitting after quoted/multiline rejection;
- finite `f64` parsing for numeric evidence;
- bounded vectors and counters for shape inspection;
- existing storage types for later atomic `CreateNew` publication.

Do not add encoding-detection libraries, broad CSV frameworks, an async
runtime, database, logging framework, UI framework, TypeScript, or frontend
tooling. Do not add semicolon/locale packages or a model dependency.

Any dependency proposal must be a separate narrow justification showing a
supported input that the standard library cannot handle safely, the exact
transitive/build/disk cost, and why a few clear lines cannot provide the needed
contract. Convenience or speculative format expansion is not sufficient.

## Testing Strategy

Future tests should be deterministic, tiny, and separated by responsibility.

Pure unit tests should prefer inline byte arrays for:

- UTF-8 without BOM;
- UTF-8 with BOM;
- UTF-16LE with BOM;
- UTF-16BE with BOM;
- odd-length UTF-16 and unpaired surrogate rejection;
- BOM-free invalid UTF-8 that must not be guessed as UTF-16;
- unsupported or ambiguous binary input;
- comma-separated numeric text;
- tab-separated numeric text;
- delimiter ambiguity;
- inconsistent field counts;
- unsupported quoted and multiline structures;
- metadata/header and unit-row candidates;
- wavelength matrix classification;
- generic classification remaining chemistry-neutral;
- conservative kinetics compatibility.

Process-level CLI smoke tests may use tiny committed byte fixtures only when
encoding bytes cannot be expressed clearly through the existing harness.
Fixtures must be project-controlled and contain no private laboratory data.
No large fixture is justified.

CLI tests must prove:

- one explicit input is read;
- default inspection writes no files;
- no system temporary directory is used;
- no cache, log, project state, artifact, or output appears;
- repeated inspection produces identical findings;
- supported encodings and comma/tab cases report the expected observations;
- wavelength matrices are never treated as time/concentration input;
- unsupported and ambiguous input receives concise actionable output;
- existing `kinetics analyze` behavior is unchanged.

Filesystem tests should use the existing Cargo-controlled test output boundary,
such as `CARGO_TARGET_TMPDIR`, with exact per-test paths and exact cleanup only.
They must not use `std::env::temp_dir`, `remove_dir_all`, recursive cleanup, or
large generated content.

## Phase Breakdown

### Phase 4.1: byte encoding/BOM inspection contract

- Freeze supported BOM and encoding results.
- Define strict UTF-8 and BOM-required UTF-16 decoding.
- Define binary/ambiguity rejection and byte-offset errors.
- Select and document the fixed conservative inspection limit.
- Use inline-byte tests; do not add CLI or conversion.

### Phase 4.2: decoded text delimiter and table-shape inspection

- Inspect comma and tab only.
- Define header candidates, numeric evidence, stable width, and shape reasons.
- Add the four generic classifications.
- Add a separate conservative kinetics compatibility assessment.
- Do not read files, write files, or add chemistry semantics.

### Phase 4.3: `data inspect` CLI

- Add one explicit input command.
- Perform bounded file reading in CLI and call pure inspection APIs.
- Print deterministic findings.
- Freeze exit behavior for supported and unsupported findings.
- Prove that inspection creates no files or hidden state.

### Phase 4.4: explicit UTF-16/TSV to normalized CSV conversion RFC

- Specify exact supported conversions and required explicit decisions.
- Define metadata/header/unit-row behavior and refusal cases.
- Define normalized byte format and output-expansion bounds.
- Reconfirm no-overwrite, existing-parent, and no-chemistry semantics.
- Do not implement conversion in this phase.

### Phase 4.5: conversion implementation using atomic CreateNew

- Implement only the accepted narrow conversion contract.
- Reuse the storage atomic executor.
- Write one explicit output and no other persistent state.
- Add bounded failure and no-clobber tests.

### Phase 4.6: real laboratory fixture validation and v0.4 audit

- Validate against a small, reviewed, non-private fixture set.
- Confirm encoding, delimiter, shape, compatibility, resource, and disk-safety
  claims.
- Audit dependencies and confirm `kinetics analyze` compatibility.
- Do not broaden formats during the audit.

No phase in this RFC is implemented by this design task.

## Deferred Work

- Full CSV/RFC 4180 support.
- Quoted delimiters and multiline fields.
- Semicolon and arbitrary delimiters.
- Locale-aware numbers and decimal commas.
- UTF-16 without BOM and statistical encoding detection.
- Excel and proprietary binary instrument formats.
- Recursive or batch directory import.
- Background indexing, watchers, caches, or import databases.
- Automatic metadata removal or unit conversion.
- Automatic column semantic inference.
- Spectroscopy-specific or other domain adapters.
- Automatic chemistry interpretation or model selection.
- Model-generated detection; optional result explanation remains off by default.
- Project database, persisted import records, and artifact browser.
- UI and TypeScript.
- Jupyter, R, PubMed, and HPC integration.

## Open Questions

- What fixed conservative input-size ceiling is justified by the first reviewed
  inventory of real, non-private instrument exports?
- Should Phase 4.3 return success for a complete inspection whose table shape
  is unsupported, or reserve a distinct nonzero exit code for unsupported
  findings?
- What minimal deterministic syntax rule is sufficiently conservative for
  distinguishing peer measurement columns from an ordinary multi-column
  narrow table?
- Should header candidates report only line numbers and verbatim labels, or
  also bounded numeric-cell evidence for the following region?
- Which metadata/header/unit-row choices must become mandatory explicit flags
  in the Phase 4.4 conversion RFC?
- Should future normalized CSV preserve original line-ending style or define
  one stable line ending for byte determinism?
- What exact output-size expansion bound is required before UTF-16/TSV
  conversion can be implemented safely?
- Should a future inspection JSON schema be added only after the human-readable
  contract has real fixture evidence?

## Recommended Next Step

Review and accept this RFC, then begin Phase 4.1 only: define a pure, strict,
bounded byte encoding/BOM inspection contract with inline tests. Keep file IO
in the CLI, leave `parse_simple_numeric_csv` and `kinetics analyze` unchanged,
add no dependency or new crate, and do not begin delimiter inspection or
conversion until the byte boundary and fixed resource policy are accepted.
