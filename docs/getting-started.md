# Getting Started

`tsi` is a command-line tool for rocket staging analysis. Named after Konstantin Tsiolkovsky, the father of astronautics, it helps you calculate delta-v, analyze stage performance, and explore rocket configurations.

## Installation

### From source (recommended)

```bash
git clone https://github.com/yourusername/tsi.git
cd tsi
cargo install --path .
```

### From crates.io

```bash
cargo install tsi
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
- [Physics Reference](physics.md) - Formulas and calculations
