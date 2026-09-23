# tsi — Testing

How tsi is tested as of v0.8, and what each kind of test is for. This
replaces the original pre-implementation test plan.

The principle: **physics must be right, and the tests should be able to
tell.** In v0.6 the "optimal" optimizer was 11% heavier than a grid search,
and nothing noticed, because every test checked an example against itself.
Most of what follows exists so that can't happen again.

## At a glance (343 tests)

| Kind | Count | Where | What it catches |
|------|-------|-------|-----------------|
| Library unit tests | 181 | `src/**` `#[cfg(test)]` | Each module's arithmetic and edge cases |
| Binary unit tests | 11 | `src/cli`, `src/output` | Advice text, progress drawing, diagrams |
| CLI tests | 81 | `tests/cli.rs` | Every flag end to end, errors, JSON snapshots |
| Property tests | 16 | `tests/properties.rs` | Invariants over random inputs |
| Validation tests | 18 | `tests/validation.rs` | Agreement with real rockets |
| Doc tests | 36 | rustdoc examples | Every example in the docs compiles and runs |

Plus five runnable examples (`examples/`) that CI executes, and criterion
benchmarks (`benches/`).

```bash
cargo test                            # everything
cargo test --no-default-features      # the library alone (CLI tests need the cli feature)
cargo test --test properties          # one suite
INSTA_UPDATE=always cargo test --test cli snapshot   # re-record JSON snapshots
```

## Unit tests

Inline in each module. They pin down arithmetic (unit conversions, the
rocket equation both ways), validation (every constructor's error cases),
and the building blocks of the optimizers:

- **Sizing** (`optimizer/sizing.rs`): sizing a stage for a delta-v and then
  building it must give back exactly that delta-v; the engine count is
  minimal (one fewer fails TWR); the structural and engine limits are
  detected.
- **Analytical optimizer**: the Lagrange split is equal for identical stages
  and favours the higher-Isp stage; with a feather-light engine in vacuum the
  optimizer reproduces the equal split exactly; pinned engines stay pinned;
  failures name the stage that binds.
- **Monte Carlo**: a zero-margin design succeeds about half the time; 3%
  margin succeeds more than 95% of the time; the same seed gives identical
  results on one thread and many; lunar designs are judged at lunar gravity.
- **IspModel**: the constant matches a numerical integration of the model in
  its docs, and the range quoted in the docs holds.

## CLI tests

`tests/cli.rs` runs the real `tsi` binary (`assert_cmd`). They cover every
command and flag, including the ones that used to be silently ignored
(`--stage1-engine`, `--gravity`, `--max-stages`), error messages that must
name the flag at fault, and reproducibility with `--seed`.

Two **snapshot tests** (`insta`) pin the JSON document, schema version 1.
Numbers are rounded to six significant figures and timing and iteration
counts are redacted, so snapshots survive last-digit floating-point
differences between platforms. Snapshots live in `tests/snapshots/`; after
an intended change to the output, re-record them and review the diff.

## Property tests

`tests/properties.rs` uses `proptest` to state invariants rather than
examples.

Physics and units:
- mass addition commutes and inverts; conversions round-trip
- delta-v is positive for any mass ratio above 1, zero at 1, and rises with
  both mass ratio and Isp
- `required_mass_ratio` inverts `delta_v`

Optimizers:
- **every solution** reaches its target and meets every TWR limit and the
  engine cap (random engines, payloads, targets, structural ratios, 2-3 stages)
- a heavier payload never needs a lighter rocket
- more delta-v always costs mass, so payload fraction strictly falls
- **analytical agrees with brute force**: never more than 1% heavier, and
  brute force never more than 5% heavier than analytical. The two share the
  sizing model but none of the search logic, so agreement is real evidence

Robustness:
- arbitrary `f64` payloads and targets, NaN and infinities included, are
  either rejected by the builder or produce a finite rocket
- every public constructor, fed arbitrary `f64`s, returns `Ok` or `Err` and
  never panics

A unit-level property in `optimizer/uncertainty.rs` checks that a perturbed
engine is never better at sea level than in vacuum, for any uncertainty.

Case counts are modest so `cargo test` stays quick. When changing an
optimizer, raise them temporarily (edit `with_cases`) and run in release mode.

## Validation tests

`tests/validation.rs` compares against published data. Each test names its
source in a comment.

- **Single stages**: Saturn V S-IC and S-II, Falcon 9 stages, the Shuttle SRB,
  Super Heavy, with delta-v within published ranges.
- **Stacks**: Falcon 9's stages stacked with payload give about 9,300 m/s.
  (v0.6's test summed isolated stages and asserted more than 18 km/s.)
- **Redesign Falcon 9**: given its engines, payload and delta-v, the optimizer
  designs a rocket within 5% of the real 571.5 t, with nine Merlins and one
  MVac.
- **Saturn V**: the optimizer designs a rocket about 30% *lighter* than the
  real one. The test asserts this and its comment explains why: ideal
  staging theory ignores gravity losses, so it overloads the low-thrust
  hydrogen stages. This is a known limit of the model, tracked as #58, and
  the test documents it rather than hiding it behind a loose tolerance.

## Examples as tests

Every program in `examples/` runs in CI against the library alone. They are
written as stories (why Falcon 9 has nine engines; what Raptors would do for
Saturn V), and they compute their claims from the library rather than
stating them, so a physics change that breaks a story breaks the build or
visibly changes the output.

## Benchmarks

`cargo bench --no-default-features` runs criterion benchmarks of both
optimizers and a 10,000-build Monte Carlo run. Baselines are recorded in
cairn item #34. They are not run in CI.

## What CI runs

On every push and pull request: formatting, clippy on both feature sets,
tests on Linux, macOS and Windows, the library alone, docs with warnings
denied, the 1.87 MSRV, a dependency-tree check that the library carries no
CLI crates, every example, and `cairn check`. See
[architecture.md](architecture.md#where-the-rules-are-enforced).

## Writing a new test

- **A bug**: write the failing test first, in the narrowest suite that shows
  it. Say in a comment what used to happen.
- **An invariant** ("always", "never", "monotone"): a property test.
- **A claim about a real rocket**: a validation test with its source, and a
  tolerance you can defend before you see the result.
- **An output format**: a CLI test, or a snapshot if it's JSON.
