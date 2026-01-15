# tsi Documentation

## Overview

Documentation for `tsi` lives in multiple places, serving different audiences:

| Location | Audience | Purpose |
|----------|----------|---------|
| README.md | Everyone | First impression, quick start |
| CLI --help | Users | Command reference |
| docs.rs | Developers | API documentation |
| examples/ | Developers | Working code samples |
| CHANGELOG.md | Users/Devs | Version history |
| This file | Contributors | Documentation standards |

---

## README.md

The README is the front door. It should answer: "What is this, why should I care, and how do I use it?" in under 2 minutes of reading.

```markdown
# tsi

A rocket staging optimizer. Given payload mass, target delta-v, and available engines, `tsi` finds the optimal staging configuration.

Named for [Konstantin Tsiolkovsky](https://en.wikipedia.org/wiki/Konstantin_Tsiolkovsky), father of astronautics.

## Installation

```bash
cargo install tsi
```

Or with Homebrew:

```bash
brew install yourusername/tap/tsi
```

## Quick Start

Calculate delta-v for a single stage:

```bash
tsi calculate --engine raptor-2 --propellant-mass 100000 --dry-mass 8000
```

Optimize a two-stage rocket to LEO:

```bash
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
```

Find the best configuration from multiple engines:

```bash
tsi optimize \
  --payload 5000 \
  --target-dv 9400 \
  --engines merlin-1d,raptor-2,rl-10c \
  --max-stages 3
```

## Example Output

```
═══════════════════════════════════════════════════════════════
  tsi v0.1.0 — Staging Optimization Complete
═══════════════════════════════════════════════════════════════

  Target Δv:  9,400 m/s    Payload:  5,000 kg
  Solution:   2-stage      Total mass:  142,300 kg

  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 2 (upper)                                            │
  │  Engine:     Raptor-2 (×1)                                  │
  │  Propellant: 18,200 kg (LOX/CH4)                            │
  │  Dry mass:   2,100 kg                                       │
  │  Δv:         3,840 m/s                                      │
  │  Burn time:  142 s                                          │
  └─────────────────────────────────────────────────────────────┘
  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 1 (booster)                                          │
  │  Engine:     Raptor-2 (×9)                                  │
  │  Propellant: 112,400 kg (LOX/CH4)                           │
  │  Dry mass:   18,600 kg                                      │
  │  Δv:         5,560 m/s                                      │
  │  Burn time:  162 s                                          │
  └─────────────────────────────────────────────────────────────┘

  Payload fraction:  3.5%
  Mass margin:       +240 m/s (2.6%)

═══════════════════════════════════════════════════════════════
```

## Commands

| Command | Description |
|---------|-------------|
| `tsi calculate` | Compute delta-v for a single stage |
| `tsi optimize` | Find optimal staging configuration |
| `tsi engines` | List available engines |

Run `tsi <command> --help` for detailed options.

## Available Engines

`tsi` ships with data for common rocket engines:

| Engine | Propellant | Thrust (vac) | Isp (vac) |
|--------|------------|--------------|-----------|
| Merlin-1D | LOX/RP-1 | 914 kN | 311 s |
| Raptor-2 | LOX/CH4 | 2,450 kN | 350 s |
| RS-25 | LOX/LH2 | 2,279 kN | 452 s |
| RL-10C | LOX/LH2 | 106 kN | 453 s |
| ... | | | |

See `tsi engines --verbose` for the full list.

## As a Library

```rust
use tsi::{Engine, Mass, Velocity, Ratio, Problem, Constraints};
use tsi::optimizer::{BruteForceOptimizer, Optimizer};

let problem = Problem {
    payload: Mass::kg(5000.0),
    target_delta_v: Velocity::mps(9400.0),
    available_engines: vec![Engine::from_database("raptor-2")?],
    constraints: Constraints {
        min_twr: Ratio::new(1.2),
        max_stages: 2,
        structural_ratio: Ratio::new(0.1),
    },
    stage_count: None,
};

let solution = BruteForceOptimizer::default().optimize(&problem)?;
println!("Total mass: {} kg", solution.rocket.total_mass().as_kg());
```

## References

- [Tsiolkovsky rocket equation](https://en.wikipedia.org/wiki/Tsiolkovsky_rocket_equation)
- [Optimal staging](https://en.wikipedia.org/wiki/Multistage_rocket#Optimal_staging)
- [Specific impulse](https://en.wikipedia.org/wiki/Specific_impulse)

## License

MIT
```

---

## CLI Help Text

Help text is documentation. It should be clear, consistent, and complete.

### Principles

1. **First line is a one-sentence description**
2. **Examples for non-obvious usage**
3. **Defaults shown explicitly**
4. **Units specified for numeric arguments**

### Root Help

```
tsi - Rocket staging optimizer

Usage: tsi <COMMAND>

Commands:
  calculate  Compute delta-v and performance for a single stage
  optimize   Find optimal staging configuration for given constraints
  engines    List available rocket engines

Options:
  -h, --help     Print help
  -V, --version  Print version

Examples:
  tsi calculate --engine raptor-2 --propellant-mass 100000
  tsi optimize --payload 5000 --target-dv 9400 --engine merlin-1d
  tsi engines --verbose
```

### Calculate Help

```
Compute delta-v and performance for a single stage

Usage: tsi calculate [OPTIONS]

Options:
      --engine <NAME>           Engine from database (see `tsi engines`)
      --engine-count <N>        Number of engines [default: 1]
      --propellant-mass <KG>    Propellant mass in kilograms
      --dry-mass <KG>           Dry mass in kilograms (structure + engines)
      --structural-ratio <R>    Structural mass / propellant mass [default: 0.1]
      --isp <S>                 Specific impulse in seconds (overrides engine)
      --mass-ratio <R>          Mass ratio (alternative to masses)
  -o, --output <FORMAT>         Output format: pretty, json [default: pretty]
  -h, --help                    Print help

Examples:
  # Using an engine from the database
  tsi calculate --engine raptor-2 --propellant-mass 100000

  # Using raw values
  tsi calculate --isp 350 --mass-ratio 8.5

  # Multiple engines
  tsi calculate --engine merlin-1d --engine-count 9 --propellant-mass 400000
```

### Optimize Help

```
Find optimal staging configuration for given constraints

Usage: tsi optimize [OPTIONS] --payload <KG> --target-dv <M/S>

Required:
      --payload <KG>            Payload mass in kilograms
      --target-dv <M/S>         Target delta-v in meters per second

Options:
      --engine <NAME>           Single engine type for all stages
      --engines <LIST>          Comma-separated engine names
      --min-twr <R>             Minimum thrust-to-weight at liftoff [default: 1.2]
      --max-stages <N>          Maximum number of stages [default: 3]
      --structural-ratio <R>    Structural mass / propellant mass [default: 0.1]
      --optimizer <TYPE>        Optimizer: auto, analytical, bruteforce [default: auto]
      --monte-carlo <N>         Run N Monte Carlo iterations for uncertainty
  -o, --output <FORMAT>         Output format: pretty, json [default: pretty]
  -h, --help                    Print help

Examples:
  # Simple two-stage optimization
  tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2

  # Multi-engine search
  tsi optimize --payload 5000 --target-dv 9400 \
    --engines merlin-1d,raptor-2,rl-10c --max-stages 3

  # With uncertainty analysis
  tsi optimize --payload 5000 --target-dv 9400 \
    --engine raptor-2 --monte-carlo 10000

  # JSON output for scripting
  tsi optimize --payload 5000 --target-dv 9400 \
    --engine raptor-2 --output json
```

### Engines Help

```
List available rocket engines

Usage: tsi engines [OPTIONS]

Options:
      --verbose        Show all engine parameters
      --json           Output as JSON
      --propellant <P> Filter by propellant type
  -h, --help           Print help

Propellant types:
  lox-rp1   Liquid oxygen / RP-1 kerosene
  lox-lh2   Liquid oxygen / liquid hydrogen
  lox-ch4   Liquid oxygen / methane
  n2o4-udmh Nitrogen tetroxide / UDMH (hypergolic)
  solid     Solid propellant

Examples:
  tsi engines
  tsi engines --verbose
  tsi engines --propellant lox-ch4
  tsi engines --json | jq '.[] | select(.isp_vac > 400)'
```

---

## API Documentation (rustdoc)

Every public item needs documentation. Use `#![warn(missing_docs)]` in lib.rs to enforce this.

### Module-Level Docs

```rust
//! # tsi - Rocket Staging Optimizer
//!
//! `tsi` is a library and CLI tool for optimizing rocket staging configurations.
//!
//! ## Quick Start
//!
//! ```rust
//! use tsi::{Engine, Mass, Velocity, Ratio, Problem, Constraints};
//! use tsi::optimizer::{BruteForceOptimizer, Optimizer};
//!
//! let problem = Problem {
//!     payload: Mass::kg(5000.0),
//!     target_delta_v: Velocity::mps(9400.0),
//!     available_engines: vec![Engine::from_database("raptor-2").unwrap()],
//!     constraints: Constraints::default(),
//!     stage_count: None,
//! };
//!
//! let solution = BruteForceOptimizer::default().optimize(&problem).unwrap();
//! ```
//!
//! ## Modules
//!
//! - [`units`] - Type-safe physical units (mass, velocity, etc.)
//! - [`engine`] - Rocket engine definitions and database
//! - [`stage`] - Single stage and multi-stage rocket types
//! - [`physics`] - Core physics calculations
//! - [`optimizer`] - Staging optimization algorithms

#![warn(missing_docs)]
```

### Type Documentation

```rust
/// A mass value in kilograms.
///
/// `Mass` is a newtype wrapper around `f64` that provides type safety
/// for mass calculations. You cannot accidentally add a `Mass` to a
/// `Velocity` — the compiler will reject it.
///
/// # Construction
///
/// ```rust
/// use tsi::Mass;
///
/// let m1 = Mass::kg(1000.0);
/// let m2 = Mass::tonnes(1.0);
/// assert_eq!(m1, m2);
/// ```
///
/// # Arithmetic
///
/// ```rust
/// use tsi::Mass;
///
/// let wet = Mass::kg(100.0);
/// let dry = Mass::kg(25.0);
///
/// // Mass + Mass = Mass
/// let total = wet + dry;
///
/// // Mass / Mass = Ratio (dimensionless)
/// let ratio = wet / dry;
/// assert_eq!(ratio.as_f64(), 4.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Mass(f64);

impl Mass {
    /// Create a mass from a value in kilograms.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tsi::Mass;
    /// let m = Mass::kg(5000.0);
    /// ```
    pub fn kg(value: f64) -> Self {
        Mass(value)
    }

    /// Create a mass from a value in metric tonnes (1000 kg).
    ///
    /// # Example
    ///
    /// ```rust
    /// use tsi::Mass;
    /// let m = Mass::tonnes(5.0);
    /// assert_eq!(m.as_kg(), 5000.0);
    /// ```
    pub fn tonnes(value: f64) -> Self {
        Mass(value * 1000.0)
    }

    /// Get the mass value in kilograms.
    pub fn as_kg(&self) -> f64 {
        self.0
    }

    /// Get the mass value in metric tonnes.
    pub fn as_tonnes(&self) -> f64 {
        self.0 / 1000.0
    }
}
```

### Function Documentation

```rust
/// Calculate delta-v using the Tsiolkovsky rocket equation.
///
/// The [Tsiolkovsky rocket equation][wiki] relates the change in velocity
/// of a rocket to its specific impulse and mass ratio:
///
/// ```text
/// Δv = Isp × g₀ × ln(m₀/m₁)
/// ```
///
/// Where:
/// - `Isp` is the specific impulse in seconds
/// - `g₀` is standard gravity (9.80665 m/s²)
/// - `m₀/m₁` is the mass ratio (wet mass / dry mass)
///
/// [wiki]: https://en.wikipedia.org/wiki/Tsiolkovsky_rocket_equation
///
/// # Arguments
///
/// * `isp` - Specific impulse of the engine
/// * `mass_ratio` - Ratio of initial (wet) mass to final (dry) mass
///
/// # Returns
///
/// The ideal delta-v achievable, not accounting for gravity or drag losses.
///
/// # Example
///
/// ```rust
/// use tsi::{Isp, Ratio};
/// use tsi::physics::delta_v;
///
/// let isp = Isp::seconds(350.0);
/// let ratio = Ratio::new(8.0);
/// let dv = delta_v(isp, ratio);
///
/// // ~7,140 m/s
/// assert!(dv.as_mps() > 7000.0);
/// ```
///
/// # Panics
///
/// Panics if `mass_ratio` is less than or equal to zero.
pub fn delta_v(isp: Isp, mass_ratio: Ratio) -> Velocity {
    assert!(mass_ratio.as_f64() > 0.0, "mass ratio must be positive");
    Velocity::mps(isp.as_seconds() * G0 * mass_ratio.as_f64().ln())
}
```

### Error Documentation

```rust
/// Errors that can occur during optimization.
#[derive(Debug, thiserror::Error)]
pub enum OptimizeError {
    /// No feasible solution exists for the given constraints.
    ///
    /// This typically means the target delta-v is too high for the
    /// available engines and stage limits, or the minimum TWR
    /// constraint cannot be satisfied.
    #[error("no feasible solution: {reason}")]
    Infeasible {
        /// Explanation of why no solution was found.
        reason: String,
    },

    /// The requested engine was not found in the database.
    #[error("unknown engine: {name}")]
    UnknownEngine {
        /// The engine name that was requested.
        name: String,
    },

    /// A constraint value was invalid.
    #[error("invalid constraint: {message}")]
    InvalidConstraint {
        /// Description of the invalid constraint.
        message: String,
    },
}
```

---

## Examples Directory

Standalone examples that can be run with `cargo run --example`.

### examples/basic.rs

```rust
//! Basic usage of the tsi library.
//!
//! Run with: cargo run --example basic

use tsi::{Engine, Mass, Velocity, Isp, Ratio};
use tsi::physics::delta_v;

fn main() -> anyhow::Result<()> {
    // Calculate delta-v directly
    let isp = Isp::seconds(350.0);
    let mass_ratio = Ratio::new(8.0);
    let dv = delta_v(isp, mass_ratio);
    println!("Delta-v: {:.0} m/s", dv.as_mps());

    // Load an engine from the database
    let raptor = Engine::from_database("raptor-2")?;
    println!("\nRaptor-2:");
    println!("  Thrust (vac): {:.0} kN", raptor.thrust_vac.as_kn());
    println!("  Isp (vac): {:.0} s", raptor.isp_vac.as_seconds());

    // Calculate stage performance
    let propellant = Mass::kg(100_000.0);
    let dry_mass = Mass::kg(8_000.0);
    let wet_mass = propellant + dry_mass;
    let ratio = wet_mass / dry_mass;
    let stage_dv = delta_v(raptor.isp_vac, ratio);
    println!("\nWith 100t propellant, 8t dry:");
    println!("  Mass ratio: {:.2}", ratio.as_f64());
    println!("  Delta-v: {:.0} m/s", stage_dv.as_mps());

    Ok(())
}
```

### examples/optimize.rs

```rust
//! Optimization example.
//!
//! Run with: cargo run --example optimize

use tsi::{Engine, Mass, Velocity, Ratio, Problem, Constraints};
use tsi::optimizer::{BruteForceOptimizer, Optimizer};

fn main() -> anyhow::Result<()> {
    // Define the problem
    let problem = Problem {
        payload: Mass::kg(5_000.0),
        target_delta_v: Velocity::mps(9_400.0),
        available_engines: vec![
            Engine::from_database("merlin-1d")?,
            Engine::from_database("raptor-2")?,
            Engine::from_database("rl-10c")?,
        ],
        constraints: Constraints {
            min_twr: Ratio::new(1.2),
            max_stages: 3,
            structural_ratio: Ratio::new(0.1),
        },
        stage_count: None, // Let optimizer choose
    };

    // Run optimization
    println!("Optimizing...");
    let optimizer = BruteForceOptimizer::default();
    let solution = optimizer.optimize(&problem)?;

    // Print results
    println!("\nSolution found!");
    println!("Total mass: {:.0} kg", solution.rocket.total_mass().as_kg());
    println!("Payload fraction: {:.2}%", 
        solution.rocket.payload_fraction().as_f64() * 100.0);
    println!("Delta-v margin: +{:.0} m/s", solution.margin.as_mps());

    for (i, stage) in solution.rocket.stages.iter().enumerate().rev() {
        println!("\nStage {} ({}):", i + 1, if i == 0 { "booster" } else { "upper" });
        println!("  Engine: {} (×{})", stage.engine.name, stage.engine_count);
        println!("  Propellant: {:.0} kg", stage.propellant_mass.as_kg());
        println!("  Δv: {:.0} m/s", stage.delta_v().as_mps());
    }

    Ok(())
}
```

### examples/monte_carlo.rs

```rust
//! Monte Carlo uncertainty analysis.
//!
//! Run with: cargo run --example monte_carlo

use tsi::{Engine, Mass, Velocity, Ratio, Problem, Constraints};
use tsi::optimizer::{BruteForceOptimizer, MonteCarloRunner, Optimizer};

fn main() -> anyhow::Result<()> {
    let problem = Problem {
        payload: Mass::kg(5_000.0),
        target_delta_v: Velocity::mps(9_400.0),
        available_engines: vec![Engine::from_database("raptor-2")?],
        constraints: Constraints::default(),
        stage_count: Some(2),
    };

    // First get nominal solution
    let optimizer = BruteForceOptimizer::default();
    let nominal = optimizer.optimize(&problem)?;

    // Run Monte Carlo
    let runner = MonteCarloRunner {
        iterations: 10_000,
        isp_uncertainty: Ratio::new(0.02),      // ±2%
        structural_uncertainty: Ratio::new(0.10), // ±10%
    };

    println!("Running {} Monte Carlo iterations...", runner.iterations);
    let result = runner.run(&problem, &nominal)?;

    println!("\nResults:");
    println!("Success probability: {:.1}%", result.success_probability * 100.0);
    println!("Delta-v (5th percentile): {:.0} m/s", result.delta_v_percentiles.p5);
    println!("Delta-v (median): {:.0} m/s", result.delta_v_percentiles.p50);
    println!("Delta-v (95th percentile): {:.0} m/s", result.delta_v_percentiles.p95);

    Ok(())
}
```

---

## CHANGELOG.md

Follow [Keep a Changelog](https://keepachangelog.com/) format.

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2025-XX-XX

### Added
- Multi-engine optimization with brute force search
- JSON output format (`--output json`)
- `tsi engines` command to list available engines

### Changed
- Improved terminal output formatting

### Fixed
- Panic when engine list is empty

## [0.2.0] - 2025-XX-XX

### Added
- Two-stage analytical optimizer
- Pretty terminal output with box drawing
- Engine database with Merlin-1D, Raptor-2, RS-25, RL-10C

### Changed
- `calculate` command now accepts `--engine` flag

## [0.1.0] - 2025-XX-XX

### Added
- Initial release
- Type-safe unit system (Mass, Velocity, Force, Time, Isp)
- Tsiolkovsky rocket equation implementation
- `tsi calculate` command for single-stage analysis
- Basic CLI with clap

[Unreleased]: https://github.com/user/tsi/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/user/tsi/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/user/tsi/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/user/tsi/releases/tag/v0.1.0
```

---

## CONTRIBUTING.md

```markdown
# Contributing to tsi

Thanks for your interest in contributing!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/tsi`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Run clippy: `cargo clippy`
7. Format code: `cargo fmt`
8. Commit and push
9. Open a pull request

## Development Setup

```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/YOUR_USERNAME/tsi
cd tsi
cargo build

# Run tests
cargo test

# Run the CLI
cargo run -- calculate --help
```

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix any warnings
- Write tests for new functionality
- Document public APIs with rustdoc comments
- Keep commits atomic and write clear commit messages

## Adding an Engine

1. Find authoritative data (manufacturer specs, NASA documents)
2. Add entry to `data/engines.toml`
3. Add a validation test in `tests/validation.rs`
4. Update the README engine table

## Reporting Bugs

Please include:
- `tsi --version` output
- The exact command you ran
- Expected vs actual behavior
- Any error messages

## Feature Requests

Open an issue describing:
- The problem you're trying to solve
- Your proposed solution
- Any alternatives you considered

## Questions?

Open a discussion or issue. We're happy to help!
```

---

## Documentation Build

```bash
# Build and open docs locally
cargo doc --open

# Build docs with private items (for development)
cargo doc --document-private-items --open

# Check for broken doc links
cargo doc 2>&1 | grep "warning:"
```

### docs.rs Configuration

```toml
# Cargo.toml
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

```rust
// lib.rs
#![cfg_attr(docsrs, feature(doc_cfg))]
```

---

## Documentation Checklist

Before each release:

- [ ] README.md is up to date
- [ ] All public items have rustdoc comments
- [ ] Examples compile and run
- [ ] CHANGELOG.md updated
- [ ] CLI help text matches implementation
- [ ] `cargo doc` builds without warnings
- [ ] Links in docs are not broken

---

## Writing Style

### Be Concise
Bad: "This function is used for the purpose of calculating..."
Good: "Calculates..."

### Use Active Voice
Bad: "The delta-v is calculated by this function."
Good: "Calculates the delta-v."

### Show, Don't Tell
Bad: "This is very useful for rocket calculations."
Good: Show a code example that demonstrates the usefulness.

### Avoid Jargon (or explain it)
Bad: "Uses Lagrange multipliers for the KKT conditions."
Good: "Uses Lagrange multipliers to find the optimal solution (see [Optimal Staging][link])."

### Include Units
Bad: `--payload <VALUE>`
Good: `--payload <KG>`
