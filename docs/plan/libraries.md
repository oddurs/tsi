# tsi — Rust Libraries Reference

A curated list of Rust crates that could expand `tsi`'s capabilities. Organized by what they'd enable.

**Legend:** ✅ = In use, 📋 = Planned, ⏳ = Future

---

## Core Dependencies (v1.0)

These are the essentials — already planned for the initial release.

### CLI & Configuration

| Crate | Purpose | Status | Notes |
|-------|---------|--------|-------|
| `clap` | Argument parsing | ✅ v0.1 | Derive macros, completions, excellent help generation |
| `serde` | Serialization | ✅ v0.1 | JSON/TOML for config and output |
| `serde_json` | JSON output | ✅ v0.2 | Pretty-printed and machine-readable output |
| `toml` | Config files | ✅ v0.2 | Engine database format |
| `anyhow` | Error handling | ✅ v0.1 | Convenient error propagation in binaries |
| `thiserror` | Error types | ✅ v0.1 | Derive Error for library types |
| `num-format` | Number formatting | ✅ v0.2 | Thousands separators in output |

### Parallelism & Performance

| Crate | Purpose | Status | Notes |
|-------|---------|--------|-------|
| `rayon` | Data parallelism | ✅ v0.4 | Parallel brute-force search |
| `rand` | Random numbers | 📋 v0.5 | Distributions for uncertainty analysis |

### Terminal Output

| Crate | Purpose | Status | Notes |
|-------|---------|--------|-------|
| `ratatui` | Terminal UI | ⏳ | For future TUI mode |
| `crossterm` | Terminal handling | ⏳ | Cross-platform terminal control |
| `indicatif` | Progress bars | ⏳ | Nice spinners and progress indicators |
| `comfy-table` | Tables | ✅ v0.2 | Engine listing output |
| `owo-colors` | Colored output | ⏳ | Simple coloring with NO_COLOR support |

---

## Units & Dimensional Analysis

**Current approach:** Custom newtypes in `src/units/` (Mass, Velocity, Force, Time, Isp, Ratio). Simple, fast compile times, educational.

Instead of rolling our own unit types, consider these battle-tested options for future migration.

### `uom` — Units of Measurement
```toml
uom = { version = "0.37", default-features = false, features = ["f64", "si"] }
```

Type-safe zero-cost dimensional analysis. Works with quantities (length, mass, time) rather than units (meter, kilometer, foot), making operations have zero runtime cost.

```rust
use uom::si::f64::*;
use uom::si::mass::kilogram;
use uom::si::velocity::meter_per_second;

let mass = Mass::new::<kilogram>(5000.0);
let velocity = Velocity::new::<meter_per_second>(3000.0);
// let nonsense = mass + velocity;  // Won't compile!
```

**Pros:** Comprehensive SI system, zero-cost, well-maintained, 7M+ downloads
**Cons:** Compile times increase, learning curve for custom quantities

### `simple-si-units` — Lighter Alternative
```toml
simple-si-units = "1.0"
```

`uom` performs dimensional analysis but cannot handle custom data types, while `simple-si-units` handles any number-like data type but does not attempt full compile-time dimensional analysis.

**Pros:** Simpler API, faster compiles
**Cons:** Less rigorous type checking

### Recommendation

Start with custom newtypes (as in architecture.md) for learning. Migrate to `uom` later if the type safety is worth the compile time cost.

---

## Orbital Mechanics & Astrodynamics

For trajectory simulation and orbital calculations.

### `nyx-space` — High-Fidelity Mission Toolkit
```toml
nyx-space = "2.2"
```

A high-fidelity space mission toolkit, with orbit propagation, estimation and some systems engineering.

Nyx has proven mission-critical reliability, already contributing to the success of several lunar missions including Firefly Blue Ghost 1 and NASA/Advanced Space CAPSTONE.

**Use for:** Actual orbit propagation, launch window calculations, gravity assists
**License:** AGPL-3.0 (copyleft — important consideration)

### `keplerian_sim` — Keplerian Orbits
```toml
keplerian_sim = "0.3"
```

Logic for Keplerian orbits, similar to the ones you'd find in a game like Kerbal Space Program. Keplerian orbits are special in that they are more stable and predictable than Newtonian orbits.

**Use for:** Simplified orbital mechanics, KSP-style calculations

### `sgp4` — Satellite Tracking
```toml
sgp4 = "0.9"
```

Rust wrapper around the Vallado SGP-4 orbital propagator.

**Use for:** TLE parsing, satellite pass prediction

---

## Optimization Algorithms

For the genetic/evolutionary optimizer phase.

### `genevo` — Genetic Algorithm Framework
```toml
genevo = "0.7"
```

Building blocks to run simulations of optimization and search problems using genetic algorithms (GA). The implementation is split into building blocks which are all represented by traits.

Supports wasm targets with the `wasm-bindgen` feature. On wasm32 targets multithreading (implemented using rayon) is disabled.

```rust
// Define your fitness function
impl FitnessFunction for StagingFitness {
    type Genotype = StagingGenotype;
    
    fn fitness_of(&self, genome: &Self::Genotype) -> f64 {
        // Evaluate rocket configuration
        let rocket = genome.to_rocket();
        if rocket.total_delta_v() < self.target_dv {
            return 0.0;  // Infeasible
        }
        rocket.payload_fraction()
    }
}
```

**Pros:** Modular, well-documented, wasm support
**Cons:** Learning curve

### `genetic_algorithm` — Simpler Alternative
```toml
genetic_algorithm = "0.3"
```

Provides Evolve builder pattern with select, crossover, mutate, and compete operations. Supports binary, continuous, discrete, and permutation genotypes.

### `moors` — Multi-Objective Optimization
```toml
moors = "0.1.0-alpha"
```

Pure-Rust crate providing multi-objective evolutionary algorithms including NSGA-II.

**Use for:** Pareto-optimal solutions (e.g., minimize mass AND cost)

### `argmin` — General Optimization
```toml
argmin = "0.10"
```

Mathematical optimization framework with many algorithms: gradient descent, Nelder-Mead, simulated annealing, particle swarm, etc.

**Use for:** Continuous optimization (propellant mass allocation)

---

## Numerical Methods

For trajectory simulation and advanced physics.

### `ode_solvers` — Differential Equations
```toml
ode_solvers = "0.4"
```

Numerical methods to solve ordinary differential equations (ODEs) in Rust. Implements RK4, Dormand-Prince 5, and DOP853 methods with adaptive step size.

```rust
use ode_solvers::*;

struct Ascent {
    thrust: f64,
    mass_flow: f64,
    drag_coeff: f64,
}

impl System<f64, Vector3<f64>> for Ascent {
    fn system(&self, t: f64, y: &Vector3<f64>, dy: &mut Vector3<f64>) {
        let (alt, vel, mass) = (y[0], y[1], y[2]);
        let gravity = 9.80665 * (6371000.0 / (6371000.0 + alt)).powi(2);
        let drag = 0.5 * self.drag_coeff * atmospheric_density(alt) * vel * vel;
        
        dy[0] = vel;  // altitude rate
        dy[1] = self.thrust / mass - gravity - drag / mass;  // acceleration
        dy[2] = -self.mass_flow;  // mass rate
    }
}
```

**Use for:** Ascent trajectory simulation, gravity turn modeling

### `differential-equations` — Comprehensive ODE Library
```toml
differential-equations = "0.1"
```

High-performance library for solving differential equations including ODEs with fixed-step and adaptive solvers, event detection, dense output, and DAEs where M can be singular.

### `nalgebra` — Linear Algebra
```toml
nalgebra = "0.33"
```

The standard Rust linear algebra library. Required by `ode_solvers` and useful for:
- Vector/matrix operations
- Coordinate transformations
- Quaternion rotations (for 3D trajectory)

---

## Atmosphere & Environment

For realistic ascent modeling.

### `ussa1976` — US Standard Atmosphere
```toml
ussa1976 = "0.2"
```

US Standard Atmosphere 1976 model. Returns temperature, pressure, density at altitude.

```rust
use ussa1976::Atmosphere;

let atm = Atmosphere::new();
let (temp, pressure, density) = atm.at_altitude(10000.0);  // 10 km
```

### Custom Implementation

The atmospheric model is simple enough to implement directly:

```rust
fn atmospheric_density(altitude: f64) -> f64 {
    const RHO_0: f64 = 1.225;  // kg/m³ at sea level
    const H: f64 = 8500.0;      // scale height in meters
    RHO_0 * (-altitude / H).exp()
}
```

---

## Data Visualization

For enhanced output and future features.

### `plotters` — Charting Library
```toml
plotters = "0.3"
```

Publication-quality charts. Can output to:
- Terminal (with `plotters-backend` crate)
- SVG/PNG files
- HTML Canvas (via wasm)

**Use for:** Porkchop plots, trajectory visualization, Monte Carlo histograms

### `textplots` — Terminal Plotting
```toml
textplots = "0.8"
```

Simple ASCII plots directly in the terminal.

```rust
use textplots::{Chart, Plot, Shape};

Chart::new(120, 60, 0.0, 10.0)
    .lineplot(&Shape::Lines(&data))
    .display();
```

**Use for:** Quick inline visualizations without leaving the terminal

### `sparkline` — Inline Sparklines
```toml
sparkline = "0.2"
```

Tiny inline charts: `▁▂▃▄▅▆▇█`

**Use for:** Monte Carlo distribution summaries in compact output

---

## Testing & Quality

| Crate | Purpose | Status | Notes |
|-------|---------|--------|-------|
| `proptest` | Property-based testing | ✅ v0.1 | Generates random inputs, 10 physics tests |
| `assert_cmd` | CLI testing | ✅ v0.2 | Integration tests for commands, 37 tests |
| `predicates` | Test assertions | ✅ v0.2 | String matching for CLI output |
| `approx` | Float comparisons | ⏳ | Safe epsilon comparisons |
| `criterion` | Benchmarking | ⏳ | Statistical benchmarks |
| `insta` | Snapshot testing | ⏳ | CLI output regression testing |

### `proptest` — Property-Based Testing ✅
```toml
[dev-dependencies]
proptest = "1.4"
```

Generates random inputs to find edge cases. Currently used for physics invariants.

### `approx` — Float Comparisons
```toml
[dev-dependencies]
approx = "0.5"
```

Safe floating-point comparisons for physics tests:

```rust
use approx::assert_relative_eq;
assert_relative_eq!(calculated_dv, expected_dv, epsilon = 1.0);
```

### `criterion` — Benchmarking
```toml
[dev-dependencies]
criterion = "0.5"
```

Statistical benchmarking with regression detection.

### `insta` — Snapshot Testing
```toml
[dev-dependencies]
insta = "1.34"
```

Snapshot testing for CLI output. Catches unexpected changes in formatting.

---

## Future Possibilities

### Web Assembly

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = "0.3"
```

Compile to wasm for browser-based calculator. `genevo` and `plotters` both support wasm.

### Python Bindings

```toml
[dependencies]
pyo3 = { version = "0.20", features = ["extension-module"] }
```

Expose `tsi` as a Python library via PyO3/maturin.

### GUI (if ever)

```toml
[dependencies]
egui = "0.27"
eframe = "0.27"
```

Immediate-mode GUI for a desktop app. Cross-platform, Rust-native.

---

## Dependency Philosophy

### Principles

1. **Prefer fewer dependencies** — Each dep is a maintenance burden
2. **Prefer well-maintained crates** — Check last commit date, download count
3. **Prefer permissive licenses** — MIT/Apache-2.0 for maximum compatibility
4. **Start simple, add later** — Don't add `uom` until you need it

### License Compatibility

| License | Compatible with MIT? | Notes |
|---------|---------------------|-------|
| MIT | ✅ | No issues |
| Apache-2.0 | ✅ | No issues |
| BSD | ✅ | No issues |
| MPL-2.0 | ✅ | File-level copyleft |
| LGPL | ⚠️ | Dynamic linking required |
| GPL | ❌ | Viral, infects your code |
| AGPL | ❌ | Even network use triggers |

**Watch out for:** `nyx-space` is AGPL. If you use it, your project becomes AGPL.

### Recommended Cargo.toml

```toml
[package]
name = "tsi"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Rocket staging optimizer"

[dependencies]
# CLI
clap = { version = "4", features = ["derive", "env"] }
anyhow = "1"
thiserror = "2"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Parallelism
rayon = "1"
rand = "0.8"

# Terminal
indicatif = "0.17"
comfy-table = "7"
owo-colors = "4"

[dev-dependencies]
approx = "0.5"
assert_cmd = "2"
predicates = "3"
proptest = "1"
criterion = "0.5"

[[bench]]
name = "optimizer"
harness = false

# Future phases - uncomment as needed
# [dependencies.ratatui]
# version = "0.26"
# optional = true

# [dependencies.ode_solvers]
# version = "0.4"
# optional = true

# [features]
# tui = ["ratatui", "crossterm"]
# trajectory = ["ode_solvers", "nalgebra"]
```

---

## Quick Reference by Feature

| Feature | Crates |
|---------|--------|
| Units | `uom`, `simple-si-units`, or custom |
| Optimization | `argmin`, `genevo`, `genetic_algorithm` |
| Orbits | `nyx-space`, `keplerian_sim`, `sgp4` |
| ODEs | `ode_solvers`, `differential-equations` |
| Atmosphere | `ussa1976` or custom |
| Plotting | `plotters`, `textplots`, `sparkline` |
| TUI | `ratatui`, `crossterm` |
| Wasm | `wasm-bindgen`, compatible crates |
| Python | `pyo3`, `maturin` |

---

## What to Use When

### Phase 1-3 (Foundation through Two-Stage) ✅ COMPLETE
Custom units, `clap`, `serde`, `serde_json`, `toml`, `anyhow`, `thiserror`, `comfy-table`, `num-format`

### Phase 4 (Multi-Engine) ✅ COMPLETE
Added `rayon` for parallel search (custom stderr progress, not `indicatif`)

### Phase 5 (Monte Carlo) — NEXT
Add `rand`, `rand_distr` for distributions

### Phase 6 (Polish)
Consider `indicatif` for better progress bars, `owo-colors` for colored output

### Phase 7+ (Trajectory Simulation)
Add `ode_solvers`, `nalgebra`, consider `uom`

### Post-1.0 (Advanced Features)
`nyx-space` (if AGPL ok), `genevo`, `plotters`, `ratatui`
