# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2025-01-15

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

## [0.2.0] - 2025-01-15

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

## [0.1.0] - 2025-01-15

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

[Unreleased]: https://github.com/yourusername/tsi/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/yourusername/tsi/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/yourusername/tsi/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/yourusername/tsi/releases/tag/v0.1.0
