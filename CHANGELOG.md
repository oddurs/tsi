# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/oddurs/tsi/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/oddurs/tsi/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/oddurs/tsi/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/oddurs/tsi/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/oddurs/tsi/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/oddurs/tsi/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/oddurs/tsi/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/oddurs/tsi/releases/tag/v0.1.0
