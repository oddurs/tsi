# tsi — Architecture

How the code is put together as of v0.8. This replaces the original
pre-implementation design; the history is in git and in
[roadmap-v0.md](roadmap-v0.md).

## Two crates in one package

The package `tsiolkovsky` builds two things:

| Target | Entry point | What it is |
|--------|-------------|------------|
| Library `tsiolkovsky` | `src/lib.rs` | The physics, engines, stages and optimizers. Never prints. |
| Binary `tsi` | `src/main.rs` | A command-line tool over the library, behind the default `cli` feature. |

The binary owns everything to do with terminals: argument parsing (`clap`),
box-drawing output, progress bars, and advice about which flag to change. The
library reports data and causes. A library user who turns off the default
feature (`default-features = false`) gets no clap, anyhow or serde_json; CI
checks the dependency tree to keep it that way.

```
src/
├── lib.rs               Crate docs and the prelude
├── main.rs              The tsi binary: declares cli and output
│
├── units/               Mass, Velocity, Force, Isp, Time, Ratio newtypes
├── physics/             Rocket equation, TWR, burn time, IspModel, losses
├── engine/              Engine, Propellant, EngineDatabase (+ data/engines.toml)
├── stage/               Stage and Rocket
├── optimizer/
│   ├── mod.rs           Optimizer trait, OptimizeError, Infeasibility, Progress
│   ├── problem.rs       Problem, ProblemBuilder, Constraints
│   ├── sizing.rs        Shared stage sizing and the minimum engine count
│   ├── analytical.rs    Lagrange solution refined numerically
│   ├── brute_force.rs   Streaming grid search
│   ├── solution.rs      Solution, OptimizerKind, the serialized report
│   ├── uncertainty.rs   Uncertainty, and the crate-private sampler
│   └── monte_carlo.rs   MonteCarloRunner, MonteCarloResults
│
├── cli/                 (binary) clap arguments and the commands
└── output/              (binary) terminal and ASCII-diagram formatting
```

## Layers

Each layer uses only the ones above it.

1. **units**: plain `f64` newtypes that make it a compile error to add a
   mass to a velocity. They serialize as bare numbers; units live in field
   names (`total_mass_kg`).
2. **physics**: free functions (`delta_v`, `twr`, `burn_time`), `G0`,
   `IspModel` and the empirical loss models. `IspModel` explains why a first
   stage's Isp depends on altitude, and averages it over the ascent weighted
   by 1/m, the way the rocket equation weights it.
3. **engine**: `Engine` is validated when it is made, by `Engine::new` or on
   deserialization (serde `try_from`): positive vacuum values, sea-level
   performance no better than vacuum. `EngineDatabase::builtin()` parses the
   embedded TOML once.
4. **stage**: `Stage` (engines + propellant + structure) and `Rocket` (stages
   + payload + booster `IspModel` + surface gravity). A rocket knows the
   gravity it was sized for and quotes TWR against it.
5. **optimizer**: problems, the optimizers, and Monte Carlo analysis.

## Validity by construction

Invalid values are errors, not panics, and the checks happen at the door:

- `Engine::new`, `Stage::new`, `Rocket::new` return `Result`.
- `Problem::builder().build()` validates everything, including constraints,
  stage counts, pinned engines, and whether any engine can fly each stage.
  So a `Problem` is always valid, and optimizers never re-check it.
- Inside the crate, optimizers assemble stages with `Stage::from_parts` and
  `Rocket::from_parts`, crate-private constructors used only where the sizing
  math guarantees validity.
- The library denies `clippy::unwrap_used` and `clippy::expect_used` outside
  tests. The one `expect` (parsing the embedded database) is annotated with
  `#[expect]` and a reason, and a test proves the data is valid.

Methods that take a stage index (`Rocket::stage_delta_v` and friends) panic
past the top like slice indexing, and say so; `Rocket::stage(i)` is the
checked lookup.

## The optimizers

Both optimizers build rockets **top down**: the payload is known, so the top
stage can be sized, then the stage below carries it, and so on. The sizing
math lives in `optimizer/sizing.rs` and is shared:

- **Growth factor.** A stage with mass ratio R and structural ratio ε,
  carrying mass m, with engines of mass E, weighs (m + E)·R / (1 − ε(R − 1))
  from its engines up. That blows up at R = 1 + 1/ε: the structural ceiling.
- **Minimum engine count.** Extra engines only add dry mass, so the fewest
  that meet the TWR limit are always best, and that number has a closed form.
  Neither optimizer searches engine counts.

`AnalyticalOptimizer` starts every engine-to-stage assignment from the
classical Lagrange multiplier solution (constant structural coefficients,
equal splits for identical stages), then refines the delta-v split
numerically against the exact mass model: a coarse scan and golden-section
search along each pair of stages. It handles any stage count and engine mix.

`BruteForceOptimizer` shares none of that search logic. It walks a
logarithmic propellant grid scaled to the payload, depth-first per rayon
task (constant memory), refines around the best point, and densifies the
grid if it finds nothing. Its value is as an independent check; the property
tests hold the two within a few percent of each other.

Physical rules both optimizers apply:

- An Earth-launched first stage uses ascent-averaged Isp and sea-level
  thrust; every other stage uses vacuum values.
- On Earth, a first stage below an upper stage must deliver at least
  2,000 m/s (`Constraints::min_booster_delta_v`), so that upper stages
  modelled in vacuum really do light above the air.
- No hidden margin: rockets hit the target exactly unless
  `Constraints::with_margin` asks for more.

When nothing works, the failure is reported as an `Infeasibility`: the
constraint that binds (structural limit, engine limit, engines too heavy,
booster too small), diagnosed from the Lagrange starting point rather than
from the extremes the search visits. The CLI turns each cause into advice
about flags.

## Monte Carlo

`MonteCarloRunner` stresses one fixed design: every build gets per-stage
factors on Isp and thrust (sea-level and vacuum together) and structural
mass, and is judged against the design's target at the design's gravity.
Each sample has its own RNG seeded from (seed, index), so results are
identical for a given seed on any number of threads. `rand` types stay out
of the public API.

## Progress and output

Long runs report through the `Progress` trait (`start`, `advance`,
`finish`, all no-ops by default). The library never prints; the CLI's
`StderrProgress` draws under a lock so the percentage only moves forward.

`Solution` and `MonteCarloResults` implement `serde::Serialize` as the
versioned report documented in [../commands.md](../commands.md)
(`JSON_SCHEMA_VERSION`). The report types themselves are crate-private; the
JSON contract is the schema, not Rust structs. `tsi optimize --output json`
adds `design_margin_percent`, which belongs to the problem.

## Public API shape

- Structs with invariants have private fields and accessors.
- Error enums, `Propellant`, `IspModel` and `OptimizerKind` are
  `#[non_exhaustive]`.
- `Constraints` setters take `impl Into<Ratio>`, so `with_margin(0.02)`
  works.
- `tsiolkovsky::prelude` holds the everyday imports.

## Where the rules are enforced

| Rule | Enforced by |
|------|-------------|
| The library has no CLI dependencies | CI `library` job (`cargo tree`) |
| Formatting, lints, warnings | CI `fmt`, `clippy` (both feature sets), `RUSTFLAGS=-D warnings` |
| MSRV 1.87 | CI `msrv` job; `rust-version` in Cargo.toml |
| Docs build and have no broken links | CI `docs` job, `RUSTDOCFLAGS=-D warnings` |
| JSON output doesn't drift | `insta` snapshots in `tests/cli.rs` |
| Examples keep working | CI `examples` job |
| Roadmap is valid and rendered | CI `roadmap` job (`cairn check`, `cairn render`) |
