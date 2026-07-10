# Phase 4.4 Explicit Laboratory Data Conversion RFC

## Summary

Phase 4.5 should add exactly one explicit normalization command:

```sh
deepseek-science data convert \
  --input <path> \
  --output <path>
```

The first conversion contract is deliberately narrow. It accepts only one
unambiguous named finite numeric narrow table already classified through the
Phase 4.1 and Phase 4.2 inspection contracts. It may remove one supported BOM,
strictly transcode BOM-marked UTF-16LE or UTF-16BE to UTF-8, replace structural
tab delimiters with commas, and normalize record separators to LF. It does not
choose a header, remove metadata or unit rows, trim cells, quote fields, repair
shape, infer scientific meaning, or run kinetics analysis.

Every successful conversion produces one deterministic byte format: UTF-8
without a BOM, comma-delimited, LF-separated, with exactly one trailing LF,
one named header row, and a finite rectangular numeric body. Raw input remains
bounded by the existing 16 MiB inspection contract, normalized output is
bounded by a new fixed 24 MiB contract, and the output is published only
through the existing atomic `CreateNew` storage boundary.

An input already usable by the current simple numeric CSV parser is rejected as
`AlreadyCompatible` rather than copied. This keeps conversion distinct from a
general copy or line-ending utility and avoids an unnecessary disk write.

This RFC freezes design only. It does not implement `data convert`, modify the
existing inspection or analysis commands, create a new crate, or add a
dependency.

## Motivation

`data inspect` can now report strict encoding, BOM, comma/tab structure, table
shape, and generic compatibility without writing files. It intentionally does
not turn an inspected source into an analysis input. Users therefore need one
safe bridge for the small set of cases where the only differences are an
explicitly supported text encoding, a BOM, or tab separators.

The bridge must not become a general import engine. A command that silently
chooses a table region, removes instrument metadata, drops unit rows, trims
cells, or adds CSV quoting would make scientific data changes that are harder
to review than the original format differences. The smallest useful contract
is a deterministic syntax-only normalizer for tables whose structure is
already complete and unambiguous.

Separating this contract from inspection preserves three boundaries:

- inspection observes and reports without side effects;
- conversion performs one explicit, bounded, no-overwrite publication;
- `kinetics analyze` continues to require exact user-selected columns and does
  not receive hidden semantic decisions from either command.

## Current Inspection Boundary

Phase 4.1 currently exports:

- `inspect_text_encoding(&[u8])`;
- `EncodingInspection`, containing decoded `text`, `encoding`, `bom`, and
  `original_byte_len`;
- `TextEncoding::{Utf8, Utf16Le, Utf16Be}`;
- `ByteOrderMark::{None, Utf8, Utf16Le, Utf16Be}`;
- `EncodingInspectionError`;
- `MAX_INSPECTION_BYTES`, fixed at `16 * 1024 * 1024` bytes.

The decoder is strict and non-lossy. UTF-8 may have no BOM or one UTF-8 BOM;
UTF-16LE and UTF-16BE require their BOM. UTF-32, BOM-free invalid UTF-8,
invalid UTF-16, repeated or conflicting BOMs, NUL, binary evidence, and raw
input above the limit are rejected.

Phase 4.2 currently exports:

- `inspect_delimited_text(&str)`;
- `DelimitedTextInspection` and `TableRegionInspection`;
- `DelimiterFinding::{Comma, Tab, Ambiguous, Unsupported}`;
- `GenericTableShape::{NumericNarrowTable, NumericMatrix,
  MixedOrUnsupported, Empty}`;
- bounded `TableShapeReason` and `BoundedLineEvidence` findings;
- `assess_simple_csv_compatibility`;
- `SimpleCsvCompatibility::{CompatibleAsIs,
  RequiresExplicitNormalization, Incompatible}`.

`SimpleCsvCompatibility::RequiresExplicitNormalization` is a format finding,
not conversion authorization. In particular, the current generic assessment
may report normalization for a narrow table with pre-header metadata. This RFC
adds stricter conversion eligibility and rejects that case rather than removing
the metadata.

`inspect_delimited_text` also applies `MAX_INSPECTION_BYTES` to the decoded
UTF-8 byte length. Phase 4.5 must not bypass that current limit. Therefore a raw
input at or below 16 MiB can still be ineligible if UTF-16-to-UTF-8 expansion
makes the decoded `String` exceed the existing delimited-inspection limit. The
24 MiB output ceiling is a publication bound, not a promise that every raw file
at the 16 MiB ceiling can be classified.

Phase 4.3 keeps file opening and reporting in the CLI. It opens one explicit
regular file, checks metadata length, reads through a `MAX_INSPECTION_BYTES +
1` cap once, and calls the two pure common APIs once. It writes nothing.

The current downstream compatibility authority is
`parse_simple_numeric_csv(&str)`. It accepts a deliberately small UTF-8,
comma-delimited, one-header, finite numeric subset and returns a `DataTable`.
It trims cells while parsing; conversion must not use that trimming behavior to
silently change source text.

The current publication authority is:

- `AtomicWriteRequest`;
- `WriteMode::CreateNew`;
- `AtomicWritePlan::execute(&[u8])`.

The executor requires an existing parent, creates a same-directory sibling
with create-new semantics, writes and synchronizes the complete bytes, uses a
hard-link publication step that refuses an existing final target, and removes
only its operation-owned temporary sibling.

## User Story

A user has one laboratory text export that `data inspect` identifies as a
named numeric narrow table but reports as requiring normalization solely
because of a supported BOM, BOM-marked UTF-16 encoding, or tab delimiter. The
user chooses one new output path and runs `data convert`. The command either:

- writes one deterministic simple CSV that the current parser accepts; or
- refuses the operation before publication with a concise explanation of the
  unsupported or ambiguous structure.

The user never has to trust an implicit header choice, row deletion, column
selection, unit conversion, scientific interpretation, overwrite, or hidden
project write.

## Proposed CLI

The future command surface is:

```sh
deepseek-science data convert \
  --input <path> \
  --output <path>
```

It accepts only:

- one required `--input <path>`;
- one required `--output <path>`;
- `--help`;
- `-h`.

It rejects missing values, duplicate options, unknown flags, and unexpected
positional arguments using the existing manual CLI parser style.

The first version must not expose:

- `--force` or `--overwrite`;
- `--in-place`;
- `--skip-lines`, `--header-line`, or `--unit-row`;
- `--delimiter` or `--encoding`;
- `--columns` or any column-order option;
- `--json`;
- batch, recursive, directory, watch, or background modes.

The only user-selected transformation inputs are the explicit source and new
target paths. The command derives encoding and delimiter only from the already
deterministic inspection contracts. It is never started automatically after
`data inspect`.

Help should state the 16 MiB input limit, 24 MiB output limit, exact supported
normalizations, no-overwrite behavior, existing-parent requirement, no-op
rejection, and the refusal of metadata, unit rows, whitespace repair, quoting,
matrices, and scientific interpretation.

## Supported Conversion Cases

Every row in this table is supported only when all eligibility rules in the
next section also pass:

| Source encoding and BOM | Source delimiter | Transformation |
| --- | --- | --- |
| UTF-8 with one UTF-8 BOM | comma | Remove exactly one BOM and emit UTF-8 without BOM |
| UTF-16LE with BOM | comma | Strictly decode and emit UTF-8 without BOM |
| UTF-16BE with BOM | comma | Strictly decode and emit UTF-8 without BOM |
| UTF-8 without BOM | tab | Replace structural tabs with commas |
| UTF-8 with one UTF-8 BOM | tab | Remove the BOM and replace structural tabs with commas |
| UTF-16LE or UTF-16BE with BOM | tab | Strictly decode and replace structural tabs with commas |

LF and CRLF source record terminators may be normalized to LF as part of one
of these substantive encoding, BOM, or delimiter transformations. A mixture of
LF and CRLF is acceptable when it introduces no blank record or cell content;
lone CR is not a supported record separator and is rejected as unsafe content.

The following is intentionally not a supported conversion case:

- UTF-8 without BOM, comma-delimited input that the current simple parser can
  already use, regardless of whether its accepted records use LF, CRLF, or no
  final terminator.

That source receives `AlreadyCompatible`; conversion is not a line-ending-only
or copy operation.

## Conversion Eligibility

Conversion is eligible only if all of the following are true:

1. `inspect_text_encoding` completed successfully.
2. `inspect_delimited_text` completed successfully and reports `complete`.
3. The delimiter is uniquely `DelimiterFinding::Comma` or
   `DelimiterFinding::Tab`.
4. The shape is exactly `GenericTableShape::NumericNarrowTable`.
5. One unique `TableRegionInspection` exists.
6. The region starts at physical line 1 and contains the complete nonblank
   table; no table-region choice is required.
7. `header_candidate_lines.total_count` is exactly 1.
8. The header contains the stable field count, all labels are nonempty and
   unique, and no second header or unit row exists.
9. At least one following row is fully finite numeric.
10. Every numeric body row has the stable field count.
11. `metadata_lines`, `inconsistent_width_lines`,
    `additional_content_lines`, `empty_numeric_cell_lines`, and
    `non_finite_numeric_lines` all have zero occurrences.
12. There is no nonnumeric body row, quote finding, ambiguous delimiter,
    ambiguous table region, or unsupported structural reason.
13. `blank_line_count` is zero. A final LF or CRLF terminator is not itself a
    blank physical row; an additional empty or whitespace-only record is.
14. A raw-cell safety pass over the already selected delimiter and region finds
    no leading/trailing Unicode whitespace and no unsafe output character.
15. The source requires one of the supported BOM, encoding, or delimiter
    transformations.

The raw-cell safety pass is not a second delimiter, header, or region
inspection. It uses the existing selected delimiter and complete selected
region only to ensure that exact cell text can be emitted without trimming,
quoting, or escaping.

`SimpleCsvCompatibility` is used conservatively:

- `Incompatible` is rejected.
- `RequiresExplicitNormalization` remains subject to every stricter rule
  above; it is necessary but not sufficient.
- `CompatibleAsIs` becomes `AlreadyCompatible` after structural and cell-safety
  validation; it is not copied.

## Explicit Refusal Cases

The first contract refuses, without repair:

### Encoding and resource refusal

- raw input above `MAX_INSPECTION_BYTES`;
- decoded text above the existing delimited-inspection limit;
- unsupported or ambiguous encoding;
- UTF-32;
- BOM-free UTF-16;
- invalid UTF-8 or UTF-16;
- repeated, conflicting, or unsupported BOMs;
- binary or NUL evidence;
- normalized output above `24 * 1024 * 1024` bytes.

### Delimiter and shape refusal

- `DelimiterFinding::Ambiguous` or `DelimiterFinding::Unsupported`;
- semicolon, pipe, whitespace, locale, or arbitrary delimiter inference;
- `GenericTableShape::NumericMatrix`;
- `GenericTableShape::MixedOrUnsupported`;
- `GenericTableShape::Empty`;
- multiple plausible table regions;
- headerless numeric data;
- multiple headers or a separate unit row;
- metadata before the header;
- additional nonblank content after the selected table;
- inconsistent field counts;
- empty body cells;
- non-finite numeric cells;
- any nonnumeric body row;
- any blank physical row that would have to be removed.

### Cell refusal

- quoted or multiline fields;
- any cell requiring CSV quoting;
- comma in a TSV cell;
- double quote in any header or body cell;
- embedded CR, LF, or NUL in a cell;
- ASCII controls U+0001 through U+001F in a cell;
- DEL U+007F in a cell;
- leading or trailing Unicode whitespace in a cell;
- any source that would require trimming, escaping, substitution, numeric
  repair, or missing-value insertion.

For tab input, tabs are structural delimiters and cannot be cell content. For
comma input, commas are structural delimiters; quoted commas are already
unsupported, and extra unquoted commas change field width rather than becoming
cell content.

### Path and publication refusal

- lexically equal input and output paths;
- an empty output path;
- an output path without a file name;
- parent-directory traversal in the output path, matching the existing CLI
  output boundary;
- a missing or non-directory output parent;
- an output path requiring directory creation;
- an existing output target;
- a stale temporary sibling that prevents create-new execution;
- a filesystem that cannot perform the existing no-clobber atomic publication
  contract.

No refusal authorizes the command to remove a row, choose a different header,
rename a label, modify a value, overwrite a path, or retry through a less safe
write mode.

## Input Contract

Phase 4.5 should reuse the `data inspect` input path behavior:

1. Open exactly one caller-provided path.
2. Obtain metadata from the opened handle.
3. Require a regular file.
4. Reject metadata length above `MAX_INSPECTION_BYTES` before proportional raw
   allocation.
5. Read through a cap of `MAX_INSPECTION_BYTES + 1` bytes.
6. Reject an observed extra byte, including when the file grows after metadata
   inspection.
7. Pass the resulting byte buffer once to `inspect_text_encoding`.
8. Pass the decoded text once to `inspect_delimited_text`.
9. Do not reread the file.

The command must not use `read_to_string`, unbounded `read`, unbounded
`read_to_end`, memory mapping, async I/O, or a streaming-loader abstraction.
The existing private bounded-reader helper may be reused or minimally shared
inside the CLI; it must not become a general public loader.

The input path is not canonicalized for normal output, and its parent and
siblings are not scanned. The command does not lock, rename, delete, chmod, or
write the input. Symlinks follow ordinary platform opening behavior, as they
do for `data inspect`; the opened handle must still report a regular file.

## Output Contract

Every successful output has exactly this format:

- encoding: UTF-8;
- BOM: none;
- delimiter: ASCII comma;
- record separator: LF (`\n`);
- terminal newline: exactly one LF;
- quoted fields: none;
- escaped fields: none;
- multiline fields: none;
- blank lines: none;
- metadata: none;
- unit row: none;
- one named header row;
- one or more finite rectangular numeric body rows;
- original column order;
- original safe cell text.

The output is represented as one bounded in-memory UTF-8 `String` or equivalent
single byte buffer. Phase 4.5 should expose one fixed output constant following
current naming style, conceptually `MAX_NORMALIZED_OUTPUT_BYTES`, equal to:

```text
24 * 1024 * 1024
```

No output configuration, feature flag, environment variable, or CLI override
is added in v0.4.

Before persistence, the normalized `&str` is passed to
`parse_simple_numeric_csv`. Success is a defensive postcondition proving that
the exact proposed bytes satisfy the current simple parser. The returned
`DataTable` is immediately discarded and is not used for column selection,
chemistry, output reformatting, or eligibility decisions. Parser failure is an
internal normalized-output validation error and prevents all publication.

## Cell Preservation Rules

Conversion preserves scientific text and performs no semantic
reinterpretation.

Header cells:

- preserve the exact decoded Unicode scalar sequence between structural
  delimiters;
- preserve case, punctuation, units, and column order;
- receive no Unicode normalization, renaming, vocabulary mapping, unit parsing,
  or scientific role assignment.

Numeric body cells:

- preserve their exact decoded lexical text;
- use finite `f64` parsing only as an eligibility check;
- are not serialized from the parsed `f64` value;
- retain precision, exponent notation, leading plus or minus signs, letter
  case in exponent markers, and any other accepted lexical choice;
- are never reordered, rescaled, unit-converted, rounded, or substituted.

For example, `+01.00e-03`, `-0`, and `1E+2` remain those exact strings if the
current finite numeric check accepts them.

The current inspection and simple parser trim cells for validation. Conversion
must not treat that as permission to trim output. For every selected source
cell, the raw decoded field must equal its Unicode-trimmed form. A difference
is a `StructuralConversionIneligible` or `UnsafeCellContent` error with
one-based line and column context where practical.

Whitespace normalization is deferred. Adding it later would require an
explicit contract describing which Unicode characters are removed and how the
change is disclosed.

## Line-ending Contract

The normalizer treats LF and CRLF as supported source record terminators and
always emits LF. It does not preserve source line-ending style.

The output algorithm emits each selected row once, joins adjacent rows with one
LF, and appends exactly one final LF. Therefore the output contains neither a
missing final newline nor a double terminal newline.

An LF/CRLF mixture is accepted only when existing inspection still identifies
one complete table and no blank row. Lone CR is rejected because it is not a
supported source record terminator and cannot be preserved safely inside a
cell.

Line-ending normalization is incidental to a supported BOM, encoding, or tab
conversion. UTF-8, no-BOM, comma input that is otherwise directly usable is
`AlreadyCompatible`, even if it uses CRLF or omits the terminal newline. The
first version does not offer line-ending-only conversion.

## No-op Conversion Policy

Freeze the following behavior:

An input that passes the strict structural and cell-safety checks and is
already UTF-8, has no BOM, uses comma delimiters, and is accepted by
`parse_simple_numeric_csv` is rejected with `AlreadyCompatible`.

The message should be equivalent to:

```text
input already matches the current simple numeric CSV format and can be used directly
```

The command exits nonzero, writes empty success stdout, and does not validate or
touch the output parent or target beyond the earlier lexical input/output
inequality check.

Cell-safety validation precedes `AlreadyCompatible`. A directly parseable file
whose cells depend on the current parser's trimming behavior is refused as
unsafe for this conversion contract rather than endorsed as canonical.

This policy:

- avoids an unnecessary write;
- avoids presenting a copy or line-ending utility as scientific conversion;
- keeps the first implementation focused on real BOM, encoding, and delimiter
  normalization;
- removes any need for `--force`, overwrite, or in-place behavior.

## Path and Publication Semantics

The command requires one explicit output path. It performs an early lexical
comparison of the caller-supplied `Path` values and rejects equality without
canonicalization or filesystem identity lookup. This is an early usability and
input-protection safeguard, not a complete alias detector.

Different spellings, symlinks, or hard-linked aliases are not resolved by that
check. If the target path already exists, the storage executor's no-clobber
publication remains authoritative and rejects it safely.

After all input, normalization, postcondition, and size checks pass, the CLI:

1. validates the output path using the existing explicit-output style;
2. maps its existing parent to `StorageRoot` and its file name to an
   `AtomicWriteRequest` logical target;
3. selects `WriteMode::CreateNew` explicitly;
4. obtains an `AtomicWritePlan`;
5. calls `AtomicWritePlan::execute(normalized.as_bytes())` once.

It must not call `Path::exists()` for a check-then-write target decision. It
must not fall back from `CreateNew` to `ReplaceExisting`, rename replacement,
direct `fs::write`, or pre-delete behavior.

The output parent must already exist. No directory is created. The existing
executor may create one deterministic same-directory temporary sibling with
create-new semantics, publish the requested final target without clobbering,
and remove only the temporary sibling created by that invocation. A stale
temporary sibling is not operation-owned and is never removed.

Success exists only when `AtomicWritePlan::execute` returns `Ok(())`. The
current executor publishes before removing its temporary hard link. A rare
cleanup failure after successful publication can therefore return an error
while the requested final target already exists. Phase 4.5 must map that case
without exposing the temporary path and tell the user that the requested target
may have been published. It must not delete the final target in an attempted
rollback. This limitation does not permit overwrite and does not modify any
pre-existing target.

## Failure Ordering

Freeze this order:

1. Parse CLI arguments.
2. Reject lexical input/output equality.
3. Open and bounded-read one regular input file.
4. Strictly decode once and inspect delimited structure once.
5. Determine conversion eligibility from the existing findings plus the raw
   cell-safety pass.
6. Return `AlreadyCompatible` if applicable.
7. Compute normalized length with checked arithmetic and build the complete
   normalized in-memory UTF-8 buffer only within the 24 MiB bound.
8. Validate the normalized `&str` with `parse_simple_numeric_csv` and verify the
   final byte count.
9. Validate and construct the atomic `CreateNew` write plan.
10. Execute atomic persistence once.
11. Emit the success summary to stdout.

Checked projected-length arithmetic may reject an oversized result before
allocating the full output buffer. The completed buffer is checked again before
any write plan is executed.

No output parent, temporary sibling, or final target is touched before all
input, structure, cell, normalization, parser-postcondition, and output-size
validation has passed.

For every failure:

- success stdout remains empty;
- stderr contains one concise actionable error;
- input bytes and permissions remain unchanged;
- a pre-existing output target remains unchanged;
- no broader cleanup is attempted.

Before publication, failure leaves no final output. During publication, the
existing executor cleans only a temporary file it created. If cleanup itself
fails after publication, the requested final target may exist as described in
the preceding section, and the command still emits no success stdout.

## Success and Error Output

Successful stdout is deterministic, human-readable, and line-oriented. It is
emitted only after atomic persistence returns success and ends with exactly one
trailing newline.

Recommended stable order:

```text
conversion_status: complete
source_encoding: utf-16le
source_bom: utf-16le
source_delimiter: tab
output_encoding: utf-8
output_bom: none
output_delimiter: comma
line_endings: lf
field_count: 3
data_rows: 8
input_bytes: 256
output_bytes: 192
```

Success output does not include timestamps, random IDs, temp paths, derived
absolute paths, header or body cell contents, model prose, chemistry column
roles, or scientific compatibility claims. Phase 4.5 has no `--json` mode.

Errors write empty success stdout and concise stderr. Structured internal
categories should be equivalent to:

- invalid arguments;
- input read failure;
- input not a regular file;
- input limit exceeded;
- encoding failure;
- structural conversion ineligible;
- already compatible;
- unsafe cell content;
- output limit exceeded;
- identical input/output lexical paths;
- invalid output path;
- missing output parent;
- target already exists;
- atomic publication failure;
- internal normalized-output validation failure.

Errors may include the exact caller-provided input or output path and safe
one-based line/column or original-input byte offsets. They must not print raw
input bytes, decoded body rows, all headers, normalized output contents,
operation-owned temporary paths, backtraces, or internal enum debug dumps.

## Crate Boundary Plan

Phase 4.5 ownership should be:

- `deepseek-science-cli`: manual argument parsing, help, lexical path equality,
  one bounded file read, orchestration, error mapping, success formatting, and
  one atomic persistence request;
- `deepseek-science-common::encoding`: unchanged strict decoding;
- `deepseek-science-common::delimited`: unchanged deterministic inspection and
  generic shape evidence;
- `deepseek-science-common::csv`: unchanged current-parser postcondition;
- at most one focused pure common module, preferably `normalize.rs`, for strict
  eligibility refinement, raw-cell safety, deterministic output construction,
  and the 24 MiB limit;
- `deepseek-science-storage`: unchanged opaque-byte atomic publication through
  `AtomicWriteRequest`, `WriteMode::CreateNew`, and
  `AtomicWritePlan::execute`;
- `deepseek-science-chemistry`: unchanged and unused by conversion.

The common normalizer is justified by two coherent responsibilities that must
remain file-IO-free and chemistry-neutral: validating that an inspected table
can be represented by the narrow output grammar, and producing that one
deterministic representation. It is not a codec registry, parser framework,
conversion service, or format plugin system.

No new crate is justified. `deepseek-science-common` may be reconsidered after
v0.4 only if the import surface grows materially beyond strict encoding,
delimited inspection, and this one narrow normalizer. Phase 4.5 must not move
the existing modules into a new data/import crate.

## Pure Normalization Contract

The public pure surface should remain equivalent in responsibility to one
entry point:

```text
normalize_delimited_text(
    encoding: &EncodingInspection,
    inspection: &DelimitedTextInspection,
) -> Result<String, NormalizationError>
```

Exact Rust naming may follow existing style, but the public responsibility must
not expand. A plain owned `String` is sufficient; the CLI already obtains
field count, data-row count, source encoding, BOM, delimiter, and input byte
count from existing findings.

The function should:

1. consume no filesystem, environment, model, storage, or chemistry state;
2. accept only the existing successful encoding and table findings;
3. reject ineligible shape/evidence and no-op input;
4. select the delimiter only from `DelimiterFinding::Comma` or `Tab`;
5. walk the complete selected source rows once using that delimiter;
6. validate exact cells without trimming or semantic interpretation;
7. preserve cell text and column order;
8. compute output length with checked arithmetic;
9. build one UTF-8 comma/LF `String` with exactly one terminal LF;
10. enforce the fixed 24 MiB maximum.

It must not rediscover encoding or delimiter, rerank regions, call
`data inspect` as a subprocess, invoke `parse_simple_numeric_csv` as the primary
eligibility authority, call chemistry, select columns, or independently build
a `DataTable`.

After normalization, the CLI or common call site performs the single defensive
`parse_simple_numeric_csv` postcondition. The parser necessarily returns a
temporary `DataTable`; that value is immediately discarded. No other
`DataTable` construction or use is permitted in the conversion path. Reusing
the existing parser is smaller and safer than introducing a second simple-CSV
validator solely to avoid that temporary value.

The normalizer needs only a small structured error surface for `AlreadyCompatible`,
structural ineligibility, unsafe cell content with bounded location context,
output-limit failure, checked-arithmetic failure, and an impossible internal
invariant. File, path, and publication errors remain CLI/storage concerns.

## Resource and Memory Bounds

The v0.4 conversion path is fully bounded:

- raw input buffer: at most `MAX_INSPECTION_BYTES + 1` observed bytes, with
  anything above 16 MiB rejected;
- decoded `EncodingInspection.text`: bounded by strict decoding and the
  existing 16 MiB decoded-text inspection check;
- inspection evidence: bounded by the input and existing diagnostic caps;
- normalized output `String`: at most `24 * 1024 * 1024` bytes;
- parser postcondition: one temporary `DataTable`, indirectly bounded by the
  normalized byte ceiling and dropped immediately;
- CLI report and error state: small fixed fields without body-cell dumps.

The output bound is 24 MiB because a 16 MiB UTF-16 input contains at most half
as many code units, and a BMP code unit can expand to at most three UTF-8 bytes.
This gives a conservative 3:2 byte ratio. Surrogate pairs expand less, tab to
comma is byte-neutral, BOM removal shrinks, and CRLF to LF does not expand.
The terminal LF remains within the fixed ceiling for the raw-input bound.

The implementation uses checked addition while projecting delimiter and LF
bytes, reserves only after the projected length passes, and confirms actual
length before persistence. The fixed limit is not configurable in v0.4.

This is a bounded-memory design, not an exact process-RSS claim. Allocator
overhead and the temporary parser result vary. No cache, spool file, memory
mapping, streaming conversion, streaming publication, async runtime, or
background worker is added.

## Disk Safety

One successful explicit conversion may:

- read exactly one explicit regular input file;
- create at most one bounded same-directory operation-owned temporary output;
- publish exactly one explicitly named final target;
- remove only the operation-owned temporary sibling as part of atomic cleanup.

The final output and temporary payload are each bounded by 24 MiB. The command
must not:

- modify, truncate, rename, chmod, or delete the input;
- overwrite, pre-delete, rename, or alter an existing target;
- remove a stale temporary file it did not create;
- create an output parent directory;
- inspect or clean sibling files;
- use the system temporary directory;
- create a project, workspace, artifact manifest, run record, cache, log, or
  database entry;
- start a background writer, watcher, indexer, or daemon;
- perform recursive or broad cleanup.

All input and normalized bytes are validated before the atomic plan is
executed. A failed eligibility or normalization check creates no file. The
existing storage executor is the only allowed filesystem-write boundary.

## Security and Untrusted Input

Input bytes, decoded labels, numeric text, and user paths are untrusted.
Phase 4.5 must:

- retain strict non-lossy decoding and original-input byte offsets;
- reject binary, NUL, unsupported BOM, invalid Unicode, and ambiguous text;
- reject all output characters that would require quoting, escaping, control
  interpretation, or terminal-safe display;
- reject Unicode surrounding whitespace rather than silently changing it;
- use checked arithmetic for projected output length;
- keep target existence authority in the atomic create-new executor;
- avoid canonicalization claims or symlink containment claims;
- avoid printing source rows, raw bytes, normalized content, or temp paths;
- avoid model, network, tool, shell, or subprocess execution;
- avoid scientific claims based on syntactic inspection.

Normal printable Unicode headers remain allowed in the output file. Success
stdout contains only fixed machine labels and counts, so untrusted header text
cannot inject terminal fields.

The input is not locked. Concurrent modification can cause a read error or
produce a strict report for the bounded bytes actually observed. The output may
be created concurrently by another process; `CreateNew` remains the no-clobber
authority.

## Dependency Policy

Default and recommendation: no new dependency.

Use only:

- existing `inspect_text_encoding` and Rust standard-library Unicode handling;
- existing `inspect_delimited_text` and compatibility types;
- standard `str` iteration, delimiter splitting, checked arithmetic, and
  bounded `String` construction;
- existing `parse_simple_numeric_csv` for the postcondition;
- existing `AtomicWriteRequest`, `WriteMode::CreateNew`, and
  `AtomicWritePlan::execute` for publication.

Do not add an encoding detector, broad CSV framework, quoting library, async
runtime, database, logging framework, CLI framework, temporary-file library,
UI framework, JavaScript, TypeScript, Node, or model dependency.

Any future dependency proposal requires a separate narrow RFC showing a
supported case that the standard library and current contracts cannot handle.
Convenience, general CSV support, or speculative extensibility is insufficient.

## Testing Strategy

Phase 4.5 pure tests should use inline strings and byte arrays and cover:

- UTF-8 BOM plus comma normalization;
- UTF-16LE BOM plus comma normalization;
- UTF-16BE BOM plus comma normalization;
- UTF-8 no-BOM tab normalization;
- UTF-8 BOM tab normalization;
- UTF-16LE and UTF-16BE tab normalization;
- LF and CRLF normalization to LF;
- exactly one terminal LF from sources with and without a final terminator;
- preservation of numeric lexical forms such as signs, precision, and exponent
  notation;
- preservation of safe printable Unicode headers;
- `AlreadyCompatible` rejection;
- metadata, unit-row, matrix, headerless, empty, mixed, ambiguous,
  inconsistent-width, empty-cell, and non-finite rejection;
- quoted or multiline rejection;
- comma in a TSV cell;
- double quote, control, DEL, lone CR, and NUL rejection;
- leading or trailing Unicode whitespace rejection;
- output-limit failure through a private small-limit helper rather than a large
  allocation;
- deterministic repeated normalization;
- normalized output accepted by `parse_simple_numeric_csv`;
- parser-postcondition failure mapped as an internal conversion validation
  error.

No filesystem fixture is needed for pure normalization tests.

Future CLI process tests should create only tiny files beneath
`env!("CARGO_TARGET_TMPDIR")`, using PID plus a bounded atomic counter for one
exact test-owned directory per test. They should cover:

- successful UTF-8 BOM conversion;
- successful UTF-16LE conversion;
- successful UTF-16BE conversion;
- successful UTF-8 and UTF-16 TSV conversion;
- exact normalized output bytes;
- success stdout emitted only after the final target exists;
- existing-target rejection with sentinel preservation;
- missing-parent rejection without directory creation;
- identical lexical input/output rejection;
- already-compatible rejection without output creation;
- matrix, metadata, unit-row, blank-row, whitespace, quoted, and invalid
  encoding rejection;
- output-limit behavior through a private bounded unit seam, not a large
  process fixture;
- input bytes remaining byte-identical after success and failure;
- directory contents containing only the input and expected final target after
  success;
- no extra file after every failure;
- repeated conversions to different fresh targets producing byte-identical
  output;
- no use of `std::env::temp_dir` or system temporary paths;
- exact input/output cleanup and exact empty-directory removal only;
- no `remove_dir_all`, recursive cleanup, or large fixtures.

Existing storage integration tests remain the authority for exact opaque bytes,
missing parents, existing-target sentinel preservation, stale temp refusal,
temporary cleanup, and bounded two-writer `CreateNew` concurrency. Phase 4.5
should reuse that boundary rather than duplicate storage races in every CLI
test.

No real private laboratory file should be committed. Any future instrument
validation belongs to Phase 4.6 and should use reviewed, tiny, synthetic or
properly controlled project data.

## Compatibility

- `data inspect --input <path>` remains read-only and unchanged.
- `kinetics analyze` arguments, text output, JSON output, exact column
  selection, and explicit JSON `--output` behavior remain unchanged.
- Conversion never invokes kinetics, chooses chemistry columns, or claims
  direct UTF-16 kinetics support.
- `parse_simple_numeric_csv` remains unchanged and remains the current narrow
  downstream parser.
- The conversion postcondition does not change `DataTable` or its public API.
- Storage atomic planning and execution remain domain-neutral and unchanged.
- No JSON schema, artifact manifest, run record, project state, or database
  semantic is introduced.
- `version` and `doctor` behavior are unaffected.
- No new crate, dependency, feature flag, configuration, or environment
  variable is introduced.

The normalized file may later be passed explicitly to `kinetics analyze` by a
user who also supplies exact time and concentration column names. Successful
conversion proves only current-parser syntax compatibility, not scientific
suitability.

## Non-goals

The first conversion contract explicitly defers:

- conversion of `NumericMatrix` data;
- metadata skipping or preamble removal;
- unit-row removal;
- header, unit, row, or table-region selection flags;
- column selection or reordering;
- unit conversion;
- numeric parsing followed by reformatting;
- whitespace trimming or normalization;
- quoted CSV emission or parsing;
- escaped fields or multiline fields;
- semicolon, pipe, locale-aware, or arbitrary delimiters;
- Excel and proprietary binary instrument formats;
- UTF-16 without a BOM;
- statistical encoding detection;
- line-ending-only conversion;
- in-place conversion;
- overwrite, force, or replacement modes;
- batch, recursive, or directory conversion;
- conversion followed by automatic kinetics analysis;
- automatic time, concentration, wavelength, absorbance, unit, or reaction
  role inference;
- model or LLM assistance;
- project persistence, artifacts, run records, cache, or logging;
- UI, TypeScript, Jupyter, R, PubMed, or HPC integration.

## Phase 4.5 Implementation Plan

1. Add one focused pure normalization module in
   `deepseek-science-common`, one public entry point, a fixed 24 MiB constant,
   small structured errors, and inline unit tests.
2. Reuse `EncodingInspection`, `DelimitedTextInspection`,
   `DelimiterFinding`, `GenericTableShape`, `TableRegionInspection`, and
   `SimpleCsvCompatibility`; do not change Phase 4.1 or Phase 4.2 detection.
3. Extend the existing manual CLI parser with `data convert`, exactly two path
   arguments, and fixed help text. Do not add `clap` or refactor unrelated
   commands.
4. Reuse the existing private bounded reader for one input read. Do not create
   a service layer or loader abstraction.
5. Run the pure normalizer, perform the `parse_simple_numeric_csv`
   postcondition, and only then build one `AtomicWriteRequest` with
   `WriteMode::CreateNew` and execute it once.
6. Add tiny inline common tests and one focused CLI process test file beneath
   `CARGO_TARGET_TMPDIR`, with exact cleanup and no private fixtures.
7. Update README and CLI help only in Phase 4.5, after the command exists, to
   document the exact implemented limits and refusal rules.
8. Run targeted common/CLI/storage compatibility validation and one workspace
   validation pass, then audit the diff for no new dependency, crate, semantic
   inference, or disk side effect.

Expected production modification scope is one common module, one common export,
the existing CLI library, and the later README update. Storage, chemistry,
existing inspection modules, the simple CSV parser, Cargo manifests, and lock
files should not need modification unless implementation evidence proves a
genuine contract defect and a separate review approves it.

## Open Questions

No semantic question remains open for conversion v1. This RFC resolves the
previous Phase 4.0 questions as follows:

- output line endings are fixed LF;
- there is exactly one terminal LF;
- directly compatible sources are rejected as `AlreadyCompatible`;
- there are no metadata, unit-row, header, or row-selection flags;
- cells are never trimmed;
- normalized output is capped at exactly 24 MiB;
- at most one focused pure common normalizer is added;
- `parse_simple_numeric_csv` is used once as an in-memory postcondition;
- there is no conversion JSON output.

Implementation must still verify two existing-boundary details without
changing this contract:

- the exact CLI mapping for rare storage cleanup failure after publication must
  hide temporary paths and explain that the requested target may exist;
- platforms without the storage executor's hard-link publication capability
  must fail safely rather than gain a non-atomic fallback.

Exact private helper names and exact wording of concise errors may follow
current code style. They must not change eligibility, byte format, limits,
failure order, or write semantics.

## Acceptance Criteria

- Given one explicit eligible UTF-8-BOM comma table, when conversion succeeds,
  then one UTF-8 no-BOM comma/LF target with exactly one trailing LF is created.
- Given one eligible BOM-marked UTF-16LE or UTF-16BE table, when conversion
  succeeds, then strict decoding is preserved and no replacement character is
  introduced.
- Given one eligible tab table, when conversion succeeds, then only structural
  tabs become commas and exact safe cell text is preserved.
- Given a valid numeric lexical cell, when output is generated, then precision,
  sign, exponent notation, and column order remain unchanged.
- Given UTF-8 no-BOM comma input already accepted by the current parser, when
  conversion is requested, then `AlreadyCompatible` is returned and no output
  is touched.
- Given metadata, a unit row, multiple headers, a matrix, headerless data,
  ambiguity, blank rows, quotes, whitespace-bearing cells, unsafe controls, or
  inconsistent shape, when conversion is requested, then the input is refused
  without repair or output creation.
- Given raw input above 16 MiB or decoded input beyond the existing inspection
  limit, when conversion is requested, then it is rejected before publication.
- Given projected or completed normalized output above 24 MiB, when conversion
  is requested, then it is rejected before write-plan execution.
- Given lexically equal input and output paths, when arguments are processed,
  then the command fails before opening or modifying an output.
- Given a missing output parent, when publication is reached, then no directory
  or target is created.
- Given an existing output target, when publication is attempted, then
  `CreateNew` rejects it and its bytes remain unchanged.
- Given valid normalized bytes, when the parser postcondition fails, then the
  command reports an internal validation error and writes nothing.
- Given atomic persistence failure, then success stdout is empty, input is
  unchanged, and no pre-existing target is modified.
- Given atomic persistence success, then the stable success summary is emitted
  only afterward and ends with exactly one newline.
- Given any conversion, then no chemistry analysis, model, tool, storage record,
  artifact, project state, cache, log, or background work is created.
- The implementation introduces no new crate, dependency, configurable limit,
  quoting engine, codec registry, or general conversion framework.

## Recommended Next Step

Review and accept this RFC. Then implement Phase 4.5 as the smallest common and
CLI change that satisfies this exact contract: one pure normalizer, one manual
`data convert --input --output` route, one bounded read, one parser
postcondition, and one atomic `CreateNew` publication. Do not broaden supported
formats during implementation; defer real instrument validation and any format
expansion to the separate Phase 4.6 audit.
