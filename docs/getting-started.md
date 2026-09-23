# Getting Started

`tsi` is a command-line tool for rocket staging analysis. Named after Konstantin Tsiolkovsky, the father of astronautics, it helps you calculate delta-v, analyze stage performance, and design the lightest rocket for a job. The same physics is available as a Rust library, `tsiolkovsky`.

## Installation

### From source (recommended)

```bash
git clone https://github.com/oddurs/tsi.git
cd tsi
cargo install --path .
```

### From crates.io

```bash
cargo install tsiolkovsky   # installs the `tsi` command
```

### Verify installation

```bash
tsi --version
```

## Quick Start

### Calculate delta-v for a stage

The most basic use case is calculating the delta-v for a rocket stage:

```bash
# Using an engine from the database
tsi calculate --engine raptor-2 --propellant-mass 100000

# Using manual Isp and mass ratio
tsi calculate --isp 350 --mass-ratio 8.0
```

### List available engines

See all engines in the built-in database:

```bash
tsi engines
```

Filter by propellant type:

```bash
tsi engines --propellant methane
```

### Example output

```
$ tsi calculate --engine raptor-2 --propellant-mass 100000
Engine:     Raptor-2
Propellant: 100,000 kg (LOX/CH4)
Dry mass:   11,600 kg
Δv:         7,771 m/s
Burn time:  2m 20s
TWR (vac):  2.24
```

### Design a rocket

Give `tsi` a payload, a delta-v target and some engines, and it designs the
lightest rocket that does the job:

```bash
# 5 t to low Earth orbit (about 9,400 m/s including losses) with Raptors
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2

# Offer several engines; tsi decides which goes where
tsi optimize --payload 5000 --target-dv 9400 --engine merlin-1d,rl-10c --max-stages 3
```

The rocket is sized to hit the target exactly. Real hardware comes out a
little better or worse than the drawings, so check how often the design would
work, and add margin until the answer is comfortable:

```bash
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --monte-carlo 10000
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --margin 3 --monte-carlo 10000
```

### Use it from Rust

```toml
[dependencies]
tsiolkovsky = { version = "0.8", default-features = false }
```

```rust
use tsiolkovsky::prelude::*;

let raptor = EngineDatabase::builtin().get("raptor-2").unwrap().clone();
let problem = Problem::builder()
    .payload(Mass::tonnes(5.0))
    .target(Velocity::mps(9_400.0))
    .engine(raptor)
    .build()?;
let rocket = AnalyticalOptimizer.optimize(&problem)?.into_rocket();
println!("{} stages, {} at liftoff", rocket.stage_count(), rocket.total_mass());
```

The `examples/` directory has runnable programs that answer real questions:
why Falcon 9 has nine engines, what Raptors would do for Saturn V, and how
much margin is enough. Run one with `cargo run --example falcon9`.

## Core Concepts

### Delta-v (Δv)

Delta-v is the change in velocity a rocket can achieve. It's calculated using the Tsiolkovsky rocket equation:

```
Δv = Isp × g₀ × ln(mass_ratio)
```

Where:
- **Isp** = Specific impulse (seconds) - engine efficiency
- **g₀** = Standard gravity (9.80665 m/s²)
- **mass_ratio** = wet mass / dry mass

### Mass Ratio

The ratio of a rocket's fully-fueled mass to its empty mass:

```
mass_ratio = (dry_mass + propellant_mass) / dry_mass
```

Higher mass ratios mean more delta-v, but are harder to achieve structurally.

### TWR (Thrust-to-Weight Ratio)

The ratio of thrust to weight:

```
TWR = thrust / (mass × g₀)
```

- TWR > 1.0 is required to lift off from Earth
- Higher TWR means faster acceleration but often less efficiency

## Next Steps

- [Command Reference](commands.md) - Full CLI documentation
- [Engine Database](engines.md) - Available engines and their specs
- [Examples](examples.md) - Common use cases and workflows
- [Physics Reference](physics.md) - Formulas, optimal staging, and where ideal theory stops
- [API documentation](https://docs.rs/tsiolkovsky) - The library, with the theory behind each optimizer
