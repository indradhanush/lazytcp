# lazytcp coverage

`lazytcp` uses Rust upstream source-based coverage tooling:

- `rustc -C instrument-coverage`
- `llvm-profdata` and `llvm-cov` from rustup `llvm-tools`

## Prerequisite

Install Rust LLVM tools once:

```bash
rustup component add llvm-tools
```

Ensure `jq` is available for parsing JSON summary output:

```bash
jq --version
```

## Run coverage

From the repository root:

```bash
make coverage
```

This runs instrumented tests and writes reports under `target/coverage/`:

- `summary.txt`: terminal summary from `llvm-cov report`
- `summary.json`: machine-readable summary from `llvm-cov export --summary-only`
- `total.txt`: single repo-wide line coverage percentage (without `%`)
- `lcov.info`: LCOV export for CI/reporting systems
- `html/index.html`: browseable HTML coverage report

## Notes

- The workflow ignores standard library and crates.io paths with `--ignore-filename-regex`.
- Coverage scope is deterministic unit/contract test surfaces; interactive TUI behavior is not directly exercised by automation.
