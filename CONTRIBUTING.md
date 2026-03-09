# Contributing to Uxie

This project values correctness for ROM data workflows first, then ergonomics and performance.

## Development Setup

1. Install current stable Rust.
2. Clone the repository recursively:

```shell
git clone --recursive <repo-url>
```

If you already cloned without submodules, run:

```shell
git submodule update --init --recursive
```

3. Build once:

```shell
cargo build
```

The build links against the `nitroarc` shared FFI library, so you need a working C toolchain in addition to Rust.

## Local Validation Commands

Run these before opening a PR:

```shell
cargo test --all-targets
cargo clippy --all-targets
```

Optional strict pass (recommended for touched code paths):

```shell
cargo clippy --all-targets -- -D warnings
```

Optional mutation testing pass for script resolution/provider logic:

```shell
cargo mutants -f src/script_file/script_resolution.rs -f src/provider.rs --cap-lints true
```

If `cargo-mutants` is not installed:

```shell
cargo install cargo-mutants
```

## Integration Fixture Environments

Some tests are real-fixture integration tests and are intentionally marked `#[ignore]`.
They exercise real DSPRE/decomp/HGSS binary data.

### Required environment variables

- `UXIE_TEST_PLATINUM_DECOMP_PATH`
- `UXIE_TEST_PLATINUM_DSPRE_PATH`
- `UXIE_TEST_HGSS_DECOMP_PATH`
- `UXIE_TEST_HGSS_DSPRE_PATH`

### Expected fixture types

- `UXIE_TEST_PLATINUM_DECOMP_PATH`: pokeplatinum decomp root (`include/constants`, `generated`, `res/...`).
- `UXIE_TEST_PLATINUM_DSPRE_PATH`: DSPRE unpacked project root (contains `arm9.bin` or `unpacked/arm9.bin`, plus data/unpacked assets).
- `UXIE_TEST_HGSS_DECOMP_PATH`: pokeheartgold decomp root.
- `UXIE_TEST_HGSS_DSPRE_PATH`: HGSS DSPRE unpacked project root.
  - arm9 is auto-resolved from common layouts: `arm9.bin`, `unpacked/arm9.bin`, `arm9/arm9.bin`.

### Run ignored integration tests

Bash example:

```shell
export UXIE_TEST_PLATINUM_DECOMP_PATH=~/dev/pokeplatinum
export UXIE_TEST_PLATINUM_DSPRE_PATH=~/Desktop/pt_DSPRE_contents
export UXIE_TEST_HGSS_DECOMP_PATH=~/dev/pokeheartgold
export UXIE_TEST_HGSS_DSPRE_PATH=~/Desktop/hg_DSPRE_contents
cargo test --all-targets -- --ignored
```

PowerShell example (equivalent):

```powershell
$env:UXIE_TEST_PLATINUM_DECOMP_PATH="C:\dev\pokeplatinum"
$env:UXIE_TEST_PLATINUM_DSPRE_PATH="C:\path\to\platinum_dspre_project"
$env:UXIE_TEST_HGSS_DECOMP_PATH="C:\dev\pokeheartgold"
$env:UXIE_TEST_HGSS_DSPRE_PATH="C:\path\to\hgss_dspre_project"
cargo test --all-targets -- --ignored
```

If env vars are missing, tests should skip cleanly instead of failing.

## Testing Expectations

- Every testing module should have synthetic/unit coverage.
- Where real fixtures are applicable, include at least one env-gated real-file integration test.
- Avoid hardcoded machine-specific paths in tests.
- Prefer adding property tests for binary roundtrip and invariant-heavy code.

## Code and Error-Handling Guidelines

- Prefer explicit errors over silent fallback in compiler-facing or data-loading paths.
- Keep changes scoped and self-contained.
- Preserve cross-family behavior (DP, Pt, HGSS) unless a change is intentionally family-specific.

## Documentation Requirements

Update docs in the same PR when behavior changes:

- `README.md` for user-facing behavior, CLI semantics, setup, or testing workflow changes.
- If dependency, packaging, or distribution behavior changes, document submodule/setup expectations and any third-party license implications in the relevant contributor or release docs.

## Third-Party Dependency Note

`uxie` keeps `nitroarc` as a git submodule at the repository root in `nitroarc/`.

- `uxie` itself is MIT-licensed.
- The `nitroarc` dependency is LGPL-3.0-or-later.
- The project links against the `nitroarc` shared FFI library.
- If you change how `nitroarc` is built, linked, bundled, or distributed, make sure the resulting distribution remains compliant with the LGPL obligations for that dependency.
- At minimum, keep the dependency notices accurate and avoid removing or obscuring the upstream license files shipped with the submodule.
