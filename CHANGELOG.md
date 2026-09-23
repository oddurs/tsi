# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **A new look for every command.** Output is laid out by one renderer from
  a small document model: a title line, aligned fields, tables, and charts.
  `tsi optimize` shows a stage table, bars for how delta-v and mass are
  shared between stages, and a compact rocket diagram with aligned labels;
  `--show-losses` shows bars; Monte Carlo shows a histogram with the target
  marked and, when confidence is low, the `--margin` that would fix it.
- **Colour**, only in terminals: `--color auto|always|never`, and `NO_COLOR`
  is respected. (#41)
- `--ascii` for plain ASCII output, symbols included.
- `tsi calculate --output json`.
- Every JSON document carries `schema_version` and `command`.
- Snapshot tests of each command's pretty output. (#59)

### Changed

- **Breaking (JSON):** `tsi engines --output json` prints
  `{"schema_version", "command", "engines": [...]}` instead of a bare array.
- `tsi engines --output` is `pretty` or `json`; `table` still works as an
  alias for `pretty`.
- The pretty output of every command is new; scripts should use `--output
  json` or `-o compact`, whose formats are stable.

## [0.8.0] - 2026-09-23 — Stacking

tsi is now a library with a command-line tool on top, rather than the
other way round. The crate is called **`tsiolkovsky`** (`tsi` was taken on
crates.io); the command it installs is still `tsi`. Almost every breaking
change planned before 1.0 lands here, so that 0.9 and 1.0 can be additive.

The `tsi` command's behaviour and output are unchanged, apart from new
fields in its JSON.

### Migrating from 0.7

| 0.7 | 0.8 |
|-----|-----|
| `use tsi::...` | `use tsiolkovsky::...`, or `use tsiolkovsky::prelude::*` |
| `Problem::new(payload, dv, engines, constraints).with_stage_count(2)` | `Problem::builder().payload(..).target(..).engines(..).constraints(..).stages(2).build()?` |
| `.with_pinned_engine(i, e)` | `.pin(i, e)` on the builder |
| `Constraints::new(twr, upper_twr, stages, ratio)` | `Constraints::default().with_min_liftoff_twr(..).with_min_stage_twr(..)...` |
| `problem.constraints.margin = ..` | build a new problem, or `problem.to_builder()` |
| `solution.rocket`, `.margin`, `.iterations` | `solution.rocket()`, `.margin()`, `.iterations()` |
| `solution.optimizer_name` | `solution.optimizer()` (an `OptimizerKind`) |
| `solution.margin_percent(target)` | `solution.margin_percent()` |
| `engine.name`, `engine.propellant` | `engine.name()`, `engine.propellant()` |
| `Engine::new(..)`, `Stage::new(..)`, `Rocket::new(..)` | the same, returning `Result` |
| `EngineDatabase::default()` | `EngineDatabase::builtin()` (a shared reference, parsed once) |
| `OptimizeError::Infeasible { reason }` | `OptimizeError::Infeasible(Infeasibility)` |
| `BruteForceOptimizer::default().with_progress(true)` | `.with_progress(your_observer)`; the default is silent |
| `MonteCarloRunner::run_design` → results | → `Result<MonteCarloResults, UncertaintyError>` |
| `Uncertainty { isp_percent: .., .. }` | `Uncertainty::default().with_isp_percent(..)`, or `low()` / `high()` |
| `ParameterSampler` | no longer public |
| `tsi::cli`, `tsi::output` | part of the binary, not the library |

### Added

- **Library without the CLI**: `default-features = false` drops clap,
  anyhow and serde_json. (#24)
- `tsiolkovsky::prelude`, and crate-level documentation that starts with the
  rocket equation. (#29)
- `Problem::builder()` validates everything on `build()`, so a `Problem` is
  always valid; `Problem::to_builder()` makes variations. (#29)
- **Per-stage structural ratios**: `Constraints::with_structural_ratios`,
  and `--structural-ratio 0.04,0.06` on the command line. The definition
  (structure excluding engines, over propellant) is documented against the
  textbook structural coefficient, with a table of real stages;
  `Stage::structural_ratio()` and `Stage::structural_coefficient()` give
  both. (#30)
- `Progress`: an observer for long optimizations and Monte Carlo runs. The
  library never prints. (#25)
- `Infeasibility`: the constraint that binds, as data. The CLI turns it into
  advice about flags. (#26)
- `EngineError`, `StageError`, `RocketError`, `DatabaseError`,
  `UncertaintyError`. Every error enum is `#[non_exhaustive]`. (#26, #28)
- Serde: units serialize as plain numbers; `Engine`, `Propellant` and
  `IspModel` serialize and deserialize (engines are validated on the way
  in); `Solution` and `MonteCarloResults` serialize as the documented JSON
  report. `tsi optimize --output json` is built from them and carries
  `"schema_version": 1`, plus new `structural_mass_kg` and
  `surface_gravity_mps2` fields. (#31)
- `physics::losses::ascent_losses(&Rocket)` and `leo_orbital_velocity()`. (#32)
- Examples that tell stories: `falcon9` (why nine engines),
  `hydrogen_upper_stage`, `saturn_v_with_raptors`, `monte_carlo` (how much
  margin is enough), and `quickstart`. (#33)
- Criterion benchmarks for both optimizers and Monte Carlo. (#34)
- CI tests the library alone, checks it has no CLI dependencies, and runs
  every example.

### Changed

- **Breaking:** the crate is renamed `tsiolkovsky`. (#23)
- **Breaking:** private fields and accessors throughout the optimizer
  module, `Engine`, and `Uncertainty`; see the migration table. (#28, #29)
- **Breaking:** constructors validate and return `Result`: engines with sea-
  level performance better than vacuum, non-positive masses, or zero
  engines are rejected by name, as are malformed engine files. (#27, #28)
- **Breaking:** loss estimates are `Velocity`, not `f64`. (#32)
- `Propellant` and `IspModel` are `#[non_exhaustive]`.
- The library denies `clippy::unwrap_used` and `clippy::expect_used`.
  Invalid values are errors, not panics. Methods that take a stage index
  (`Rocket::stage_delta_v` and friends) still panic past the top, as slice
  indexing does, and say so; `Rocket::stage(i)` returns an `Option`.

### Fixed

- Brute-force designs now carry their surface gravity, so their reported
  TWR is right off Earth (a regression in 0.7's review fixes).

### Removed

- `Uncertainty::new` and `BruteForceOptimizer::with_vacuum_preference`,
  deprecated in 0.7.

## [0.7.0] - 2026-09-22 — Static fire

Every number tsi prints was checked against an independent method or a real
rocket, and several were wrong. This release fixes them. Designs are lighter
and more honest, so expect different numbers from the same commands.

### Fixed

- **The analytical optimizer was not optimal.** For the README example it
  designed a 205 t rocket where a grid search found 183 t. It now starts from
  the Lagrange multiplier staging solution and refines it against the exact
  mass model, including engine mass, which the classical equal split ignores.
  Engine counts come from a closed-form TWR bound, so the unbounded
  engine-count loops are gone. (#9)
- **A hidden 2% delta-v margin** was added to every analytical design.
  Margin is now an explicit constraint, default zero. (#8)
- **First stages used vacuum Isp**, overstating booster delta-v by 5-10%. They
  now use Isp averaged over the ascent, weighted by 1/mass as the rocket
  equation weights it (305 s for a Merlin-1D, against 311 s in vacuum); upper stages still use vacuum. See `IspModel`. (#10)
- **Monte Carlo analysis re-optimized every sample**, so it measured whether
  *some* rocket could be found rather than whether *this* one survives, and it
  always reported 100%. It now builds the chosen design with correlated
  per-stage errors. A zero-margin design succeeds about half the time. (#11)
- **Brute force missed small rockets** (fixed 10 t propellant floor), silently
  kept only three engines, under-counted its iterations, and held its whole
  search space in memory (about 1 GB for three stages). It now scales its grid
  to the payload, tries every engine on every stage, counts every evaluation,
  and streams its search in a few MB. (#12, #13)
- `--max-stages` was used as the exact stage count. (#14)
- `--stage1-engine` and `--stage2-engine` added engines to the pool instead of
  pinning them. (#15)
- `--gravity` changed only the printout. It now sets the gravity the
  optimizer enforces TWR against, and Mars and Moon launches use vacuum Isp
  for the first stage. (#16)
- JSON `"twr"` for stage 1 was vacuum TWR, not liftoff TWR. (#17)
- NaN and infinite inputs passed validation and failed later with unrelated
  errors. (#18)
- The Falcon 9 validation test summed stage delta-v without stacking the
  upper stage on the booster, and asserted more than 18 km/s. (#20)

### Added

- `Constraints::min_booster_delta_v` (default 2,000 m/s for Earth launches of
  two or more stages). Validating against Saturn V showed that without it the
  optimizer would give the first stage no propellant at all, using it only
  for liftoff thrust, and let vacuum-only upper stages fly from sea level.
- The analytical optimizer handles any number of stages and any mix of
  engines, trying every engine-to-stage assignment. It is now the automatic
  choice for every problem.
- `Problem::with_pinned_engine`, `Problem::design_delta_v`,
  `Problem::stage_count_range`, `Problem::engines_for_stage`.
- `Constraints::{margin, surface_gravity, booster_isp}` and their `with_*`
  builders.
- `IspModel`, `Engine::{isp_for, thrust_for}`, `Stage::delta_v_with`,
  `Rocket::{with_booster_isp, with_surface_gravity, surface_gravity,
  isp_model, liftoff_twr_in, stage_twr_in}`. A rocket remembers the gravity
  it was sized for, and `liftoff_twr` and `stage_twr` quote against it, so
  Monte Carlo judges a lunar design by lunar gravity.
- Up-front errors for problems no rocket can solve: only vacuum engines for an
  Earth first stage, a vacuum engine pinned to stage 1, or a stage with no
  engine to use. Infeasibility errors name the stage and the flag that binds
  (`--min-twr` or `--min-upper-twr`), and brute force reports the same
  diagnosis as the analytical optimizer.
- `MonteCarloRunner::{with_seed, run_design}`, `MonteCarloResults::seed`,
  `ParameterSampler::{perturb_engine_with_rng, perturb_rocket}`, and
  `Solution::target_delta_v`.
- CLI: `--stages`, `--max-engines`, `--margin`, `--seed`. JSON gains
  `twr_liftoff`, `twr_ignition`, `isp_s`, `design_margin_percent`,
  `booster_isp_model`, and Monte Carlo `seed` and design fields.
- Property tests for optimizer invariants, including analytical against brute
  force; validation that the optimizer redesigns Falcon 9 within 2%. (#19, #20)
- GitHub Actions CI on Linux, macOS and Windows, with an MSRV of Rust 1.87. (#7)
- Roadmap and issues tracked with cairn; `ROADMAP.md` is generated.

### Changed

- **Breaking:** JSON stage field `twr` is replaced by `twr_ignition`, plus
  `twr_liftoff` on stage 1.
- **Breaking:** `terminal::print_solution` takes only the solution, and
  `print_solution_with_options` takes (solution, design margin); gravity
  comes from the rocket.
- Pretty output labels TWR "at liftoff" or "at ignition" and shows the
  booster Isp model.
- `--sea-level` is deprecated and does nothing; liftoff TWR always uses
  sea-level thrust.
- `Uncertainty::new` is deprecated: its argument order didn't match its
  fields. Use a struct literal.
- `BruteForceOptimizer::with_vacuum_preference` is deprecated and has no
  effect.
- `MonteCarloResults::failures` now counts builds too heavy to lift off.
  With `--uncertainty none`, every requested build is counted, not just one.
- Offering so many engines that the largest stage counts exceed the search
  cap keeps the best design from smaller counts instead of failing.

### Known limitations

- Ideal staging theory ignores gravity losses, so it favours high-Isp,
  low-thrust upper stages more than real designers do. Asked to do Saturn V's
  job, tsi designs a rocket about 30% lighter than the real one. Tracked as #58.

## [0.6.0] - 2026-01-16

### Added

- ASCII rocket diagram with `--diagram`, stage heights scaled by propellant
- Gravity and drag loss estimates with `--show-losses` (empirical models)
- Inline custom engines: `--custom-engine "name:thrust_kn:isp_s:mass_kg:propellant"`
- `tsi completions` for bash, zsh and fish, and `--man` for a man page
- Suggestions in error messages for infeasible problems

## [0.5.0] - 2026-01-16

### Added

- Monte Carlo uncertainty analysis: `--monte-carlo N` and
  `--uncertainty none|low|default|high`
- `Uncertainty`, `ParameterSampler`, `MonteCarloRunner`, `MonteCarloResults`
- Success probability, percentiles, required margin, and an ASCII histogram
- Monte Carlo results in JSON output

## [0.4.0] - 2026-01-15

### Added

- `BruteForceOptimizer`: parallel grid search (rayon) over stage count,
  engine choice, engine count and propellant mass, with coarse-to-fine
  refinement
- `--optimizer auto|analytical|brute-force`, comma-separated `--engine`
  lists, `--stage1-engine` and `--stage2-engine`
- Runtime, iteration count and optimizer name in `Solution` and in output

## [0.3.0] - 2026-01-15

### Added

- **Two-stage optimization** with `tsi optimize` command
  - Analytical optimizer using Lagrange multiplier solution
  - Optimal staging theory: equal delta-v split for identical engines
  - 2% margin on target delta-v for robustness
  - JSON output support with `--output json`

- **Rocket type** for multi-stage vehicle analysis
  - Total delta-v aggregation across stages
  - Payload fraction calculation
  - Liftoff TWR validation
  - Mass above each stage tracking

- **Constraints and Problem types** for optimization
  - Configurable min TWR for liftoff and upper stages
  - Maximum stage count (currently limited to 2)
  - Structural ratio configuration

- **Terminal output formatting** with Unicode box drawing
  - Professional stage-by-stage breakdown
  - Payload fraction and margin display
  - Consistent formatting with thousands separators

- **Property-based tests** using proptest (10 tests)
  - Mass addition commutativity
  - Delta-v monotonicity with mass ratio and Isp
  - Round-trip conversions for units

- **Validation tests** against real rocket data (10 tests)
  - Saturn V, Falcon 9, Space Shuttle, Starship verification
  - Optimal staging theory validation

- **Doc tests** for all public API examples (21 doc tests)

### Changed

- Total test count: 168 tests (117 unit + 31 CLI + 10 property + 10 validation + 21 doc)
- Refactored pretty output to use dedicated terminal module

### Fixed

- Optimizer test import issues
- Doc test assertions for realistic delta-v values

## [0.2.0] - 2026-01-15

### Added

- **Engine database** with 11 real rocket engines:
  - Merlin-1D, Merlin-Vacuum (SpaceX Falcon 9)
  - Raptor-2, Raptor-Vacuum (SpaceX Starship)
  - RS-25, RL-10C (NASA)
  - F-1, J-2 (Saturn V)
  - RD-180 (Atlas V)
  - BE-4 (Blue Origin)
  - Rutherford (Rocket Lab)

- **`tsi engines` command** to list available engines
  - `--output json` for machine-readable output
  - `--verbose` to show sea-level performance values
  - `--propellant <TYPE>` to filter by propellant (methane, hydrogen, kerosene)
  - `--name <PATTERN>` to filter by name

- **Enhanced `tsi calculate` command**
  - `--engine <NAME>` to use engine from database
  - `--engine-count <N>` for multiple engines
  - `--propellant-mass <KG>` for propellant-based calculations
  - `--structural-ratio <R>` to configure structural mass fraction
  - `--output compact` for one-line output

- **Stage type** with full performance calculations:
  - Delta-v with payload
  - TWR at ignition (vacuum and sea-level)
  - Burn time

- **Improved user experience**
  - Thousands separators in number output (100,000 kg instead of 100000 kg)
  - Engine name suggestions on typos ("Did you mean: Raptor-2?")
  - Multi-error validation (reports all errors at once)
  - Examples in help text for all commands

- **Integration tests** (23 tests covering CLI functionality)

- **Documentation**
  - Getting started guide
  - Command reference
  - Engine database reference
  - Physics reference
  - Examples

### Changed

- Display formatting now uses thousands separators for readability
- Force display uses kN for values under 10 MN (matches spec expectations)

## [0.1.0] - 2026-01-15

### Added

- **Type-safe unit system** with newtypes for:
  - Mass (kg, tonnes)
  - Velocity (m/s, km/s)
  - Force (N, kN, MN)
  - Time (s, minutes)
  - Isp (seconds)
  - Ratio (dimensionless)

- **Physics calculations**
  - Tsiolkovsky rocket equation: `Δv = Isp × g₀ × ln(mass_ratio)`
  - Inverse calculation: mass ratio from delta-v
  - Thrust-to-weight ratio (TWR)
  - Burn time calculation

- **`tsi calculate` command** for single-stage analysis
  - `--isp` for specific impulse
  - `--mass-ratio` or `--wet-mass`/`--dry-mass` for mass input
  - `--thrust` for burn time calculation

- **CLI framework** using clap with derive macros

- **Unit tests** for all physics calculations (70+ tests)

### Technical

- Rust 2021 edition
- MIT license
- Validates against Falcon 9 stage parameters

[Unreleased]: https://github.com/oddurs/tsi/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/oddurs/tsi/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/oddurs/tsi/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/oddurs/tsi/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/oddurs/tsi/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/oddurs/tsi/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/oddurs/tsi/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/oddurs/tsi/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/oddurs/tsi/releases/tag/v0.1.0
