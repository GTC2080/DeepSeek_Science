# Phase 5.4 End-to-End Kinetics SVG Visualization Audit

## Audit Scope

This audit evaluates the committed-on-`main` Phase 5 visualization implementation
through the following committed baselines:

- Phase 5.0 RFC: `8ed2a77c887e0ca0d9cb123117dd0780ed9c31c6`;
- Phase 5.1 plot-data contract:
  `54c266c4e574c932ef87c8cd0006a84cb4f86438`;
- Phase 5.2 SVG renderer:
  `f5a22f3fab4d4e13d9842ba6b37a284ae226c6f5`;
- Phase 5.3 CLI integration:
  `0c10c05bc0aea61d8f977b8b0a3ba7c823fd3dca`.

Before audit asset creation, `HEAD`, `main`, and `origin/main` were aligned at
`0c10c05bc0aea61d8f977b8b0a3ba7c823fd3dca`; the worktree was clean and no
tag pointed at `HEAD`.

The audited runtime was:

- OS/kernel: Darwin 25.5.0, arm64 (`RELEASE_ARM64_T6050`);
- Rust: `rustc 1.96.1 (31fca3adb 2026-06-26)`;
- Rust host: `aarch64-apple-darwin`;
- LLVM: `22.1.2`;
- Cargo: `cargo 1.96.1 (356927216 2026-06-26)`.

This audit does not include Phase 5.5, a version change, a tag, a GitHub
Release, a release build, browser rendering, or raster comparison.

## Source and Contract Inspection

The inspected CLI path is one linear orchestration flow:

1. dispatch `kinetics plot` and parse the four required values;
2. reject lexical input/output equality and a non-SVG target extension;
3. open one regular input and use the existing 16 MiB bounded reader;
4. decode strict UTF-8 without BOM repair;
5. call the existing simple numeric CSV parser;
6. construct exact `KineticsColumns` and `ValidatedKineticsInput` values;
7. call `KineticsAnalysisResult::analyze` once;
8. call `KineticsPlotData::from_analysis` once;
9. call `render_kinetics_svg` once;
10. validate the SVG byte boundary before output planning;
11. plan one `AtomicWriteRequest` with `WriteMode::CreateNew`;
12. call `AtomicWritePlan::execute` once;
13. return `kinetics plot complete\n` only after successful publication.

No independent fit or prediction exists in the CLI or renderer. Chemistry owns
the accepted observations, existing fit metadata, 128 candidate predictions
per model, contiguous curve segments, comparison preference, deterministic
review summary, counts, and bounded warnings. The renderer consumes those
values without parsing CSV, validating kinetics, refitting, repredicting, or
sorting accepted observations.

The CLI postcondition checks the fixed root line, UTF-8/no-BOM/LF contract,
single final LF, closing element, and 4 MiB maximum. Publication reuses the
existing storage implementation; the CLI contains no direct final-target
writer, parent creation, overwrite fallback, or rollback deletion.

Storage execution opens a deterministic sibling with create-new semantics,
syncs it, publishes through a hard link that refuses an existing target, and
removes its operation-owned sibling. Plot-specific error mapping preserves a
stable existing-target error, reports a missing or invalid parent, and treats
other publication failures conservatively without exposing the sibling path.

The Phase 5.3 plot process tests resolve `CARGO_TARGET_TMPDIR`, create all test
assets under that external root, and include the real exact-limit and
limit-plus-one command paths. The existing `kinetics analyze` path still uses
its prior unbounded `fs::read_to_string` behavior; the plot-specific 16 MiB
boundary did not silently alter analysis.

## Synthetic Data

The audit created one non-private, synthetic, LF-only simple numeric CSV under
the configured external Cargo target temp root. It was not copied into the
repository and was deleted after evidence collection.

The synthetic table used exact columns `time_s` and
`concentration_mol_l`, nine strictly positive accepted observations, one zero
concentration row, and a non-ascending caller-row time order (`30` before
`15`). Its concentration series approximated first-order decay.

Input evidence:

- byte length: `130`;
- physical lines: `11`;
- final byte: LF (`0x0a`);
- CR bytes: none;
- SHA-256:
  `85ac6f132f59f4e04abc7395abc0f91a7e645faad515c6d4993b705ffd10101f`;
- accepted observations: `9`;
- rejected rows: `1`.

No fixture, snapshot, golden SVG, private experiment, or user-uploaded data was
created or used.

## Commands Run

Repository, identity, and environment checks:

- `gh --version`;
- `gh auth status`;
- `git config user.name`;
- `git config user.email`;
- `git status --short`;
- `git branch --show-current`;
- `git pull --ff-only origin main`;
- `git log --oneline --decorate -n 12`;
- `git rev-parse HEAD`;
- `git rev-parse origin/main`;
- `git tag --points-at HEAD`;
- `rustc -vV`;
- `cargo -V`;
- `uname -a`.

Source inspection used bounded `rg`, `sed`, and `nl` reads over the Phase 5
RFC, CLI dispatch/orchestration, chemistry plot-data, SVG renderer, storage
atomic implementation, and plot process tests.

Cargo validation commands:

- `cargo fmt --check`;
- `cargo check -p deepseek-science-cli`;
- `cargo test -p deepseek-science-chemistry --lib`;
- `cargo test -p deepseek-science-storage --lib`;
- `cargo test -p deepseek-science-cli`.

The actual binary was the debug Mach-O arm64 executable at
`/Users/taomu/开发/.cache/deepseek-science-target/debug/deepseek-science`.
It was `1,972,432` bytes with SHA-256
`92fad4f735ef1c30480e89bdda21be27c4fab81f92fe464e27b13a0267180aae`.

The external audit root was
`/Users/taomu/开发/.cache/deepseek-science-target/tmp/phase5-4-audit-0c10c05`.
It was confirmed absent before creation and empty immediately after creation.

Actual CLI audit invocations used that binary with these command forms:

- `kinetics plot --input <synthetic.csv> --time-column time_s
  --concentration-column concentration_mol_l --output <audit-a.svg>`;
- the same command with fresh target `<audit-b.SVG>`;
- the same command with existing sentinel target `<existing.svg>`;
- the same command with missing-parent target
  `<missing-parent/output.svg>`;
- the same command with wrong-extension target `<wrong.png>`;
- the same command with the input path also supplied as output;
- `kinetics analyze` in text mode;
- `kinetics analyze --json`;
- `kinetics analyze --output <analysis.json>` twice;
- `data inspect --input <synthetic.csv>`;
- `data convert --input <compatibility.tsv> --output <converted.csv>`.

`wc`, `shasum -a 256`, `cmp -s`, `iconv`, `head`, `tail`, `od`, `grep`,
`sed`, `awk`, and `find` provided byte, hash, UTF-8, structure, content,
inventory, and cleanup evidence. Audit cleanup used one exact `rm -f` argument
per recorded file, verified an empty directory, and then used exact `rmdir`.
No glob cleanup or recursive deletion was used.

## Results

### End-to-End Publication and Determinism

Both fresh plot invocations exited `0`, wrote exact stdout
`kinetics plot complete\n`, and wrote zero stderr bytes. Each invocation
created exactly its requested SVG and no JSON sidecar or other CLI-owned
persistent output.

`audit-a.svg` and `audit-b.SVG` were each `11,796` bytes. `cmp -s` returned
`0`, and both files had SHA-256
`8c0d0e84e17b7df37084bd79882c28614dbf4e69813c853ca80d26f871755729`.
The input remained 130 bytes with its original SHA-256 after both renders and
after all refusal and compatibility paths.

### SVG Byte, Structure, Accessibility, and Scientific Content

The generated SVG passed all audited byte and structure checks:

- exact RFC root first line;
- `width="960"`, `height="640"`, and `viewBox="0 0 960 640"`;
- fixed title and bounded description;
- `role="img"` and `aria-labelledby="plot-title plot-desc"`;
- valid UTF-8;
- no BOM or CR byte;
- final line `</svg>`;
- final bytes `</svg>\n` with exactly one trailing LF;
- size below the fixed 4 MiB maximum.

The element whitelist contained exactly `svg`, `title`, `desc`, `g`, `line`,
`polyline`, `circle`, `text`, and `rect`. The SVG contained nine accepted
observation circles, one first-order polyline, and one second-order polyline.
The rejected row did not produce an observation circle.

Visible content included `observed data`, `first-order fit`, `second-order
fit`, `MVP heuristic preference:`, `deterministic review status: passed with
warnings`, `accepted observations: 9`, `rejected rows: 1`, and
`visualization warnings: 1`. The sole warning was `rejected rows not
displayed` at the first fixed warning position, so the bounded warning order
was preserved. No model curve warning was expected or emitted.

The SVG did not contain any definitive claim such as confirmed reaction order,
proven model, correct model, validated mechanism, best model, or true order.

### SVG Safety

The SVG contained no audit, input, output, or temporary path; timestamp; UUID;
generator metadata; data URI; XML declaration; comment; script; style; image;
`foreignObject`; event-handler attribute; `href`; `xlink`; or CSS `url(...)`.

The only URI was the required root namespace identifier
`http://www.w3.org/2000/svg`; there was no external resource reference and no
HTTPS URI.

### No-Overwrite and Missing-Parent Behavior

The existing target sentinel was 40 bytes with SHA-256
`eac976318b88169c8b0e2456abc615485022d9e798311abddb75d73c5896bbb2`.
The plot invocation exited `1`, wrote empty stdout, emitted the stable
existing-target error, and left both sentinel length and SHA-256 unchanged.
The input hash also remained unchanged.

The missing-parent invocation exited `1`, wrote empty stdout, and emitted the
stable missing-or-invalid-parent error. Neither the parent nor requested SVG
was created. Wrong-extension and lexical-equality invocations also exited
nonzero with empty stdout and their stable stage-specific errors. None of the
failure outputs contained `kinetics plot complete`.

Success, existing-target refusal, and missing-parent refusal left no
operation-owned temporary sibling, hidden output, lock, journal, cache, or
sidecar. Before final cleanup, the audit directory contained exactly the
recorded audit-owned inputs, requested outputs, sentinel, compatibility
outputs, and captured stdout/stderr files, with no subdirectory.

The passing Phase 5.3 process suite additionally exercised strict invalid
UTF-8 rejection through the actual process boundary.

### Compatibility

Actual binary compatibility invocations passed for:

- `kinetics analyze` text mode;
- `kinetics analyze --json`;
- explicit kinetics JSON output;
- `data inspect`;
- `data convert`.

Analysis JSON retained `schema_version: kinetics.analysis.v1` and command
`kinetics.analyze`. It contained no observations, visualization warnings,
SVG, or polyline fields. Explicit JSON output was 992 bytes with SHA-256
`9a3d28cf622d277beedd27fd615da8a6b58239e845cd026624b7d0d9b108c449`.
A second explicit write exited nonzero with empty stdout, and the JSON length
and hash remained unchanged, confirming the existing CreateNew behavior.

`data inspect` retained its inspection report, and `data convert` produced the
expected UTF-8 comma/LF CSV. Plotting created no JSON sidecar and did not alter
the independent analysis command or `kinetics.analysis.v1` schema.

### Resource Boundaries

Source and passing test evidence confirm:

- plot input maximum: `16 * 1024 * 1024` bytes;
- the reader observes at most the limit plus one detection byte;
- renderer output maximum: `4 * 1024 * 1024` bytes;
- model candidates: exactly 128 per model;
- visualization warnings: at most three, in fixed order;
- persistent output per plot invocation: one requested SVG;
- plot analysis calls: one;
- plot publication execute calls: one;
- no parent creation, overwrite, cache, worker, watcher, or background work.

The passing `kinetics_plot_smoke` process test
`exact_limit_is_read_and_limit_plus_one_is_rejected` exercised the real
exact-limit and over-limit CLI paths. This audit therefore did not create a
second 16 MiB file merely to duplicate that evidence.

## Determinism Boundary

For the same 130 input bytes, exact selected column names, one debug binary,
one program build, and this Darwin arm64 runtime platform, repeated rendering
to two different fresh targets was byte-identical.

This is a same-build, same-platform determinism conclusion. It is not a claim
that every CPU, architecture, operating system, Rust version, or standard
library produces identical transcendental results. In particular, the current
standard-library `f64::exp` boundary does not establish a formal
cross-architecture bit-for-bit guarantee. The RFC's bounded numeric
serialization and per-release-target audit remain the applicable policy.

## Disk Safety

Cargo target output remained under the configured external root
`/Users/taomu/开发/.cache/deepseek-science-target`. The source worktree never
contained the synthetic CSV, generated SVG, sentinel, captured output,
compatibility input, JSON, or converted CSV.

The audit created and then precisely removed 31 external, audit-owned files:

- `synthetic.csv`, `audit-a.svg`, `audit-b.SVG`, `existing.svg`;
- `compatibility.tsv`, `analysis.json`, `converted.csv`;
- 24 explicit stdout/stderr capture files for success, refusal, and
  compatibility invocations.

The audit directory contained zero unexpected subdirectories or hidden,
temporary, lock, journal, or cache entries. Each recorded file was passed as an
exact argument to `rm -f`; the directory was verified empty and removed with
exact `rmdir`.

No `cargo clean`, target deletion, recursive deletion, release build, script,
browser, rasterizer, fixture generation, snapshot generation, database, log,
watcher, daemon, model call, or background task was used. No audited CLI
invocation performed network activity; the required Git/GitHub identity,
repository synchronization, and report publication operations were the only
network control-plane activity. No non-audit external file was modified or
removed.

## Conclusion

Phase 5.4 audit passed. No functional, deterministic-within-scope,
compatibility, storage-safety, resource-boundary, SVG byte-contract, or
scientific-wording failure was found.

Phase 5 visualization implementation is ready for Phase 5.5 version alignment
and release audit. This conclusion does not state or imply that v0.5.0 has
already been released.
