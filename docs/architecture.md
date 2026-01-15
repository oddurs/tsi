# tsi — Rocket Staging Optimizer

A CLI tool for optimizing rocket staging configurations. Given payload mass, target delta-v, and available engines, `tsi` finds the optimal staging solution that maximizes payload fraction or minimizes total mass.

## Overview

```
tsi --payload 5000 --target-dv 9400 --engines merlin-1d,raptor-2 --min-twr 1.2
```

The tool solves a constrained optimization problem: find the staging configuration that delivers the required delta-v with minimum mass (or maximum payload), subject to thrust-to-weight and structural constraints.

## Project Structure

```
tsi/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI setup
│   ├── lib.rs               # Library root, public API
│   ├── units/
│   │   ├── mod.rs           # Unit type exports
│   │   ├── mass.rs          # Mass newtype (kg)
│   │   ├── velocity.rs      # Velocity newtype (m/s)
│   │   ├── force.rs         # Force/thrust newtype (N)
│   │   ├── time.rs          # Time/duration newtype (s)
│   │   ├── isp.rs           # Specific impulse newtype (s)
│   │   └── ratio.rs         # Dimensionless ratios
│   ├── engine/
│   │   ├── mod.rs           # Engine types and database
│   │   ├── engine.rs        # Engine struct definition
│   │   ├── propellant.rs    # Propellant types enum
│   │   └── database.rs      # Built-in engine data, TOML loading
│   ├── stage/
│   │   ├── mod.rs           # Stage module exports
│   │   ├── stage.rs         # Single stage representation
│   │   └── rocket.rs        # Multi-stage rocket assembly
│   ├── physics/
│   │   ├── mod.rs           # Physics calculations
│   │   ├── tsiolkovsky.rs   # Rocket equation implementation
│   │   ├── thrust.rs        # TWR, burn time calculations
│   │   └── losses.rs        # Gravity drag, atmospheric losses
│   ├── optimizer/
│   │   ├── mod.rs           # Optimizer trait and implementations
│   │   ├── analytical.rs    # Closed-form 2-stage solution
│   │   ├── bruteforce.rs    # Grid search for discrete choices
│   │   ├── genetic.rs       # Evolutionary optimizer (later)
│   │   └── montecarlo.rs    # Uncertainty quantification
│   ├── output/
│   │   ├── mod.rs           # Output formatting
│   │   ├── terminal.rs      # Pretty terminal output
│   │   ├── json.rs          # Machine-readable output
│   │   └── ascii.rs         # ASCII rocket diagram
│   └── cli/
│       ├── mod.rs           # CLI argument handling
│       ├── args.rs          # Clap argument definitions
│       └── commands.rs      # Subcommand implementations
├── data/
│   └── engines.toml         # Default engine database
├── tests/
│   ├── integration.rs       # End-to-end CLI tests
│   └── physics_tests.rs     # Physics calculation validation
└── benches/
    └── optimizer_bench.rs   # Performance benchmarks
```

## Core Types

### Units (compile-time safety)

```rust
// src/units/mass.rs
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Mass(f64);

impl Mass {
    pub fn kg(value: f64) -> Self { Mass(value) }
    pub fn tonnes(value: f64) -> Self { Mass(value * 1000.0) }
    pub fn as_kg(&self) -> f64 { self.0 }
}

impl std::ops::Add for Mass {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Mass(self.0 + rhs.0) }
}

impl std::ops::Sub for Mass {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Mass(self.0 - rhs.0) }
}

// Mass / Mass = dimensionless ratio
impl std::ops::Div for Mass {
    type Output = Ratio;
    fn div(self, rhs: Self) -> Ratio { Ratio::new(self.0 / rhs.0) }
}
```

Similar newtypes for `Velocity`, `Force`, `Time`, `Isp`, and `Ratio`. The compiler prevents unit errors like adding mass to velocity.

### Engine

```rust
// src/engine/engine.rs
#[derive(Debug, Clone)]
pub struct Engine {
    pub name: String,
    pub thrust_sl: Force,      // Sea level thrust
    pub thrust_vac: Force,     // Vacuum thrust
    pub isp_sl: Isp,           // Sea level specific impulse
    pub isp_vac: Isp,          // Vacuum specific impulse
    pub dry_mass: Mass,        // Engine mass
    pub propellant: Propellant,
}

impl Engine {
    /// Effective Isp at given atmospheric pressure ratio (0 = vacuum, 1 = sea level)
    pub fn isp_at(&self, pressure_ratio: Ratio) -> Isp { ... }
    
    /// Effective thrust at given atmospheric pressure ratio
    pub fn thrust_at(&self, pressure_ratio: Ratio) -> Force { ... }
}
```

### Propellant

```rust
// src/engine/propellant.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Propellant {
    LoxRp1,    // Kerosene
    LoxLh2,    // Liquid hydrogen
    LoxCh4,    // Methane
    N2o4Udmh,  // Hypergolic
    Solid,     // Solid propellant
}

impl Propellant {
    /// Typical bulk density (kg/m³) for tank sizing estimates
    pub fn density(&self) -> f64 { ... }
    
    /// Display name
    pub fn name(&self) -> &'static str { ... }
}
```

### Stage

```rust
// src/stage/stage.rs
#[derive(Debug, Clone)]
pub struct Stage {
    pub engine: Engine,
    pub engine_count: u32,
    pub propellant_mass: Mass,
    pub structural_mass: Mass,
}

impl Stage {
    pub fn dry_mass(&self) -> Mass {
        self.structural_mass + self.engine.dry_mass * self.engine_count
    }
    
    pub fn wet_mass(&self) -> Mass {
        self.dry_mass() + self.propellant_mass
    }
    
    pub fn mass_ratio(&self) -> Ratio {
        self.wet_mass() / self.dry_mass()
    }
    
    /// Delta-v this stage provides (vacuum)
    pub fn delta_v(&self) -> Velocity {
        tsiolkovsky(self.engine.isp_vac, self.mass_ratio())
    }
    
    /// Thrust-to-weight ratio at ignition with given payload above
    pub fn twr_at_ignition(&self, payload: Mass, gravity: f64) -> Ratio { ... }
    
    /// Burn duration
    pub fn burn_time(&self) -> Time { ... }
}
```

### Rocket

```rust
// src/stage/rocket.rs
#[derive(Debug, Clone)]
pub struct Rocket {
    pub stages: Vec<Stage>,  // Bottom to top (index 0 = first stage)
    pub payload: Mass,
}

impl Rocket {
    pub fn total_delta_v(&self) -> Velocity { ... }
    pub fn total_mass(&self) -> Mass { ... }
    pub fn payload_fraction(&self) -> Ratio { ... }
    
    /// Validate all constraints (TWR, etc.)
    pub fn validate(&self, constraints: &Constraints) -> Result<(), ValidationError> { ... }
}
```

## Physics Module

### Tsiolkovsky Equation

```rust
// src/physics/tsiolkovsky.rs
use crate::units::{Isp, Ratio, Velocity};

const G0: f64 = 9.80665;  // Standard gravity (m/s²)

/// Δv = Isp × g₀ × ln(mass_ratio)
pub fn delta_v(isp: Isp, mass_ratio: Ratio) -> Velocity {
    Velocity::mps(isp.as_seconds() * G0 * mass_ratio.as_f64().ln())
}

/// Inverse: given Δv and Isp, what mass ratio is needed?
pub fn required_mass_ratio(delta_v: Velocity, isp: Isp) -> Ratio {
    Ratio::new((delta_v.as_mps() / (isp.as_seconds() * G0)).exp())
}
```

### Thrust Calculations

```rust
// src/physics/thrust.rs

/// Thrust-to-weight ratio
pub fn twr(thrust: Force, mass: Mass, gravity: f64) -> Ratio {
    Ratio::new(thrust.as_newtons() / (mass.as_kg() * gravity))
}

/// Burn time = propellant_mass / mass_flow_rate
/// mass_flow_rate = thrust / (Isp × g₀)
pub fn burn_time(propellant: Mass, thrust: Force, isp: Isp) -> Time {
    let mass_flow = thrust.as_newtons() / (isp.as_seconds() * G0);
    Time::seconds(propellant.as_kg() / mass_flow)
}
```

## Optimizer Module

### Trait Definition

```rust
// src/optimizer/mod.rs
pub trait Optimizer {
    fn optimize(&self, problem: &Problem) -> Result<Solution, OptimizeError>;
}

pub struct Problem {
    pub payload: Mass,
    pub target_delta_v: Velocity,
    pub available_engines: Vec<Engine>,
    pub constraints: Constraints,
    pub stage_count: Option<u32>,  // None = optimize this too
}

pub struct Constraints {
    pub min_twr: Ratio,
    pub max_stages: u32,
    pub structural_ratio: Ratio,  // Structural mass / propellant mass
}

pub struct Solution {
    pub rocket: Rocket,
    pub margin: Velocity,  // Excess delta-v beyond target
    pub iterations: u64,
}
```

### Analytical Solver (2-stage)

For a two-stage rocket with the same engine type, there's a closed-form solution for optimal mass distribution. This is the fast path for simple cases.

```rust
// src/optimizer/analytical.rs
pub struct AnalyticalOptimizer;

impl Optimizer for AnalyticalOptimizer {
    fn optimize(&self, problem: &Problem) -> Result<Solution, OptimizeError> {
        // Lagrange multiplier solution for optimal staging
        // Only works for 2 stages, same propellant, same structural ratio
        ...
    }
}
```

### Brute Force Solver

For discrete engine choices and integer engine counts, exhaustively search the space.

```rust
// src/optimizer/bruteforce.rs
pub struct BruteForceOptimizer {
    pub propellant_mass_step: Mass,  // Grid resolution
    pub max_engines_per_stage: u32,
}

impl Optimizer for BruteForceOptimizer {
    fn optimize(&self, problem: &Problem) -> Result<Solution, OptimizeError> {
        // Iterate over:
        // - Number of stages (1..=max_stages)
        // - Engine choice per stage
        // - Engine count per stage (1..=max_engines)
        // - Propellant mass per stage (grid search)
        // Return configuration with minimum total mass that meets constraints
        ...
    }
}
```

### Monte Carlo (uncertainty quantification)

```rust
// src/optimizer/montecarlo.rs
pub struct MonteCarloRunner {
    pub iterations: u64,
    pub isp_uncertainty: Ratio,        // e.g., ±2%
    pub structural_uncertainty: Ratio, // e.g., ±10%
}

pub struct MonteCarloResult {
    pub nominal: Solution,
    pub success_probability: f64,
    pub delta_v_percentiles: Percentiles,
    pub mass_percentiles: Percentiles,
}
```

## CLI Structure

### Main Entry Point

```rust
// src/main.rs
use clap::Parser;
use tsi::cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Command::Optimize(args) => commands::optimize(args),
        Command::Engines(args) => commands::list_engines(args),
        Command::Calculate(args) => commands::calculate(args),
    }
}
```

### CLI Arguments

```rust
// src/cli/args.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tsi")]
#[command(about = "Rocket staging optimizer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Optimize staging for given constraints
    Optimize(OptimizeArgs),
    /// List available engines
    Engines(EnginesArgs),
    /// Calculate single-stage parameters
    Calculate(CalculateArgs),
}

#[derive(clap::Args)]
pub struct OptimizeArgs {
    /// Payload mass in kg
    #[arg(short, long)]
    pub payload: f64,
    
    /// Target delta-v in m/s
    #[arg(short = 'd', long)]
    pub target_dv: f64,
    
    /// Comma-separated list of engine names
    #[arg(short, long, value_delimiter = ',')]
    pub engines: Vec<String>,
    
    /// Minimum thrust-to-weight ratio at liftoff
    #[arg(long, default_value = "1.2")]
    pub min_twr: f64,
    
    /// Maximum number of stages
    #[arg(long, default_value = "3")]
    pub max_stages: u32,
    
    /// Output format (pretty, json)
    #[arg(short, long, default_value = "pretty")]
    pub output: OutputFormat,
    
    /// Run Monte Carlo uncertainty analysis
    #[arg(long)]
    pub monte_carlo: Option<u64>,
}
```

## Output Formatting

### Terminal Output

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
  │  TWR:        0.8 → 2.1 (vacuum)                             │
  ├─────────────────────────────────────────────────────────────┤
  │  STAGE 1 (booster)                                          │
  │  Engine:     Merlin-1D (×9)                                 │
  │  Propellant: 112,400 kg (LOX/RP-1)                          │
  │  Dry mass:   4,600 kg                                       │
  │  Δv:         5,560 m/s                                      │
  │  Burn time:  162 s                                          │
  │  TWR:        1.4 → 4.2                                      │
  └─────────────────────────────────────────────────────────────┘

  Payload fraction:  3.5%
  Mass margin:       +240 m/s (2.6%)

═══════════════════════════════════════════════════════════════
```

## Data Files

### Engine Database (TOML)

```toml
# data/engines.toml

[[engine]]
name = "Merlin-1D"
thrust_sl = 845000      # N
thrust_vac = 914000     # N
isp_sl = 282            # s
isp_vac = 311           # s
dry_mass = 470          # kg
propellant = "LoxRp1"

[[engine]]
name = "Raptor-2"
thrust_sl = 2256000
thrust_vac = 2450000
isp_sl = 327
isp_vac = 350
dry_mass = 1600
propellant = "LoxCh4"

[[engine]]
name = "RS-25"
thrust_sl = 1859000
thrust_vac = 2279000
isp_sl = 366
isp_vac = 452
dry_mass = 3527
propellant = "LoxLh2"

[[engine]]
name = "RL-10C"
thrust_sl = 0           # Upper stage only
thrust_vac = 106000
isp_sl = 0
isp_vac = 453
dry_mass = 190
propellant = "LoxLh2"
```

## Dependencies

```toml
# Cargo.toml
[package]
name = "tsi"
version = "0.1.0"
edition = "2021"
description = "Rocket staging optimizer"
license = "MIT"
repository = "https://github.com/yourusername/tsi"
keywords = ["rocket", "space", "aerospace", "optimization"]
categories = ["command-line-utilities", "science", "simulation"]

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
anyhow = "1"
thiserror = "2"
rand = "0.8"
rayon = "1"              # Parallel Monte Carlo

[dev-dependencies]
criterion = "0.5"
approx = "0.5"           # Float comparisons in tests

[[bench]]
name = "optimizer"
harness = false
```

## Development Milestones

### v0.1 — Foundation
- [ ] Unit types with arithmetic operations
- [ ] Engine struct and TOML loading
- [ ] Tsiolkovsky equation implementation
- [ ] Single-stage calculator (`tsi calculate`)
- [ ] Basic CLI with clap

### v0.2 — Two-Stage Optimization
- [ ] Stage and Rocket types
- [ ] Analytical 2-stage optimizer
- [ ] Pretty terminal output
- [ ] `tsi optimize` command (single engine type)

### v0.3 — Multi-Engine Search
- [ ] Brute force optimizer
- [ ] Multiple engine types per rocket
- [ ] JSON output format
- [ ] Engine listing command (`tsi engines`)

### v0.4 — Uncertainty Analysis
- [ ] Monte Carlo runner
- [ ] Parallel execution with rayon
- [ ] Confidence interval reporting
- [ ] `--monte-carlo` flag

### v0.5 — Polish
- [ ] ASCII rocket diagram
- [ ] Atmospheric loss estimation
- [ ] Custom engine definition via CLI
- [ ] Shell completions
- [ ] Man page generation

### v1.0 — Release
- [ ] Comprehensive test suite
- [ ] Documentation
- [ ] Publish to crates.io
- [ ] Homebrew formula

## Testing Strategy

Unit tests for physics calculations with known values:
- Tsiolkovsky equation against textbook examples
- Saturn V staging parameters
- Falcon 9 approximate numbers

Integration tests for CLI:
- Round-trip: optimize then verify constraints met
- JSON output parsing
- Error handling for invalid inputs

Property-based tests:
- Mass conservation
- Delta-v monotonicity (more propellant = more delta-v)
- TWR bounds

## Future Possibilities

- **TUI mode**: Interactive parameter exploration with ratatui
- **Trajectory integration**: Actual ascent simulation with drag
- **Cost optimization**: $/kg to orbit with engine costs
- **Reusability modeling**: Account for landing propellant
- **Web assembly**: Run in browser
