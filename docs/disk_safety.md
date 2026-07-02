# Disk Safety

Disk safety is a project rule, not an afterthought.

## Build Output

Cargo is configured in `.cargo/config.toml` to write build artifacts outside the
source tree:

```text
../.cache/deepseek-science-target
```

This keeps source directories clean and prevents large generated build output
from appearing in normal project listings.

## Allowed Generated Output

Generated files may go only into ignored, controlled locations:

- `tmp/`
- `runs/tmp/`
- `artifacts/tmp/`
- `test-output/`
- the configured Cargo target directory

## Disallowed Generated Output

Generated files must not be written into:

- `crates/`
- `docs/`
- `packs/`
- `tests/fixtures/`
- `scripts/`
- repository root documentation files

## Deletion Rules

No script may perform uncontrolled deletion. Deletion scripts must print the
target path, reject suspicious paths, require typed confirmation, and avoid
broad wildcards.

## Long-running Tools

Do not run watcher loops, hot reload servers, infinite background processes, or
repeated build/test loops by default.

## Tests

Tests must not hit the network, require current credentials, create large
fixtures, or write uncontrolled files.
