# tsi

[![CI](https://github.com/oddurs/tsi/actions/workflows/ci.yml/badge.svg)](https://github.com/oddurs/tsi/actions/workflows/ci.yml)

A command-line tool for rocket staging analysis and optimization.

Named after [Konstantin Tsiolkovsky](https://en.wikipedia.org/wiki/Konstantin_Tsiolkovsky), the father of astronautics who derived the fundamental rocket equation in 1903.

## Features

- **Optimal staging** - Lagrange multiplier theory refined against real engine masses, any number of stages
- **Multi-engine optimization** - Every engine is tried on every stage; pin engines to stages when you know better
- **Monte Carlo uncertainty analysis** - Build your design a thousand times with manufacturing errors and see how often it still works
- **Custom engine support** - Define inline engines for hypothetical analysis
- **Atmospheric loss estimation** - Gravity drag and atmospheric drag approximations
- **ASCII rocket diagrams** - Visual representation of rocket configurations
- **Shell completions** - Bash, Zsh, and Fish auto-completion
- **Two independent optimizers** - Analytical, cross-checked by exhaustive brute-force search
- **Stage performance calculations** - Delta-v, burn time, TWR
- **Built-in engine database** - 11 real rocket engines
- **Validated against real rockets** - Redesigns Falcon 9 to within 2%, and is honest about where ideal theory stops
- **Type-safe physics** - Compile-time unit safety prevents calculation errors
- **Scriptable output** - JSON and compact formats with optimization metadata

## Installation

```bash
# From source
git clone https://github.com/oddurs/tsi.git
cd tsi
cargo install --path .

# Or from crates.io (when published)
cargo install tsi
```

## Quick Start

### Calculate stage performance

```bash
# Using an engine from the database
$ tsi calculate --engine raptor-2 --propellant-mass 100000
Engine:     Raptor-2
Propellant: 100,000 kg (LOX/CH4)
Dry mass:   11,600 kg
Δv:         7,771 m/s
Burn time:  2m 20s
TWR (vac):  2.24

# Using manual parameters
$ tsi calculate --isp 350 --mass-ratio 8.0
Δv:         7,127 m/s
Mass ratio: 8.00
```

### List available engines

```bash
$ tsi engines
NAME             PROPELLANT    THRUST(vac)   ISP(vac)       MASS
--------------------------------------------------------------
Merlin-1D        LOX/RP-1            914 kN      311s        470 kg
Raptor-2         LOX/CH4           2,450 kN      350s      1,600 kg
RS-25            LOX/LH2           2,279 kN      452s      3,527 kg
...

# Filter by propellant
$ tsi engines --propellant methane
NAME             PROPELLANT    THRUST(vac)   ISP(vac)       MASS
--------------------------------------------------------------
Raptor-2         LOX/CH4           2,450 kN      350s      1,600 kg
Raptor-Vacuum    LOX/CH4           2,550 kN      380s      1,600 kg
BE-4             LOX/CH4           2,600 kN      340s      2,000 kg
```

### Optimize a two-stage rocket

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
═══════════════════════════════════════════════════════════════
  tsi — Staging Optimization Complete
═══════════════════════════════════════════════════════════════

  Target Δv:  9,400 m/s    Payload:  5,000 kg
  Solution:   2-stage    Total mass:  188,866 kg

  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 2 (upper)                                            │
  │  Engine:     Raptor-2 (×1)                                  │
  │  Propellant: 29,388 kg (LOX/CH4)                            │
  │  Dry mass:   3,951 kg                                       │
  │  Δv:         4,993 m/s                                      │
  │  Burn time:  41.2s                                          │
  │  TWR:        6.52 at ignition                               │
  └─────────────────────────────────────────────────────────────┘
  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 1 (booster)                                          │
  │  Engine:     Raptor-2 (×1)                                  │
  │  Propellant: 137,896 kg (LOX/CH4)                           │
  │  Dry mass:   12,632 kg                                      │
  │  Δv:         4,407 m/s                                      │
  │  Burn time:  3m 13s                                         │
  │  TWR:        1.22 at liftoff                                │
  └─────────────────────────────────────────────────────────────┘

  Total propellant:  167,284 kg
  Total dry mass:    16,583 kg
  Total burn time:   234s

  Payload fraction:  2.65%
  Delta-v margin:    +0 m/s (+0.0%)
  Booster Isp:       343s (ascent-averaged); upper stages use vacuum Isp

  Optimizer: Analytical (181 configs)

═══════════════════════════════════════════════════════════════
```

The booster gets a little less delta-v than the upper stage: from sea level
its Raptor delivers 343 s rather than 350 s, and its engine mass counts for
less on a big stage. Ask for headroom with `--margin 2%`.

### Compact output for scripting

```bash
$ tsi calculate --engine raptor-2 --propellant-mass 100000 -o compact
Δv: 7,771 m/s | Burn: 140s | TWR: 2.24
```

## Commands

| Command | Description |
|---------|-------------|
| `tsi optimize` | Find optimal staging for a delta-v target |
| `tsi calculate` | Calculate delta-v for a single stage |
| `tsi engines` | List available rocket engines |
| `tsi completions` | Generate shell completions or man page |

Run `tsi <command> --help` for detailed options.

## Engine Database

Includes 11 real rocket engines:

| Engine | Vehicle | Propellant |
|--------|---------|------------|
| Merlin-1D | Falcon 9 (stage 1) | LOX/RP-1 |
| Merlin-Vacuum | Falcon 9 (stage 2) | LOX/RP-1 |
| Raptor-2 | Starship | LOX/CH4 |
| Raptor-Vacuum | Starship (upper) | LOX/CH4 |
| RS-25 | SLS / Shuttle | LOX/LH2 |
| RL-10C | Centaur | LOX/LH2 |
| F-1 | Saturn V (stage 1) | LOX/RP-1 |
| J-2 | Saturn V (stages 2-3) | LOX/LH2 |
| RD-180 | Atlas V | LOX/RP-1 |
| BE-4 | New Glenn | LOX/CH4 |
| Rutherford | Electron | LOX/RP-1 |

## Examples

### Falcon 9 first stage approximation

```bash
$ tsi calculate --engine merlin-1d --engine-count 9 --propellant-mass 400000
Engine:     Merlin-1D (×9)
Propellant: 400,000 kg (LOX/RP-1)
Dry mass:   44,230 kg
Δv:         7,036 m/s
Burn time:  2m 28s
TWR (vac):  1.89
```

### Parameter sweep

```bash
for mass in 50000 100000 150000; do
  tsi calculate --engine raptor-2 --propellant-mass $mass -o compact
done
```

### JSON output for processing

```bash
tsi engines --output json | jq '.[] | select(.propellant == "LoxCh4")'
```

### Custom engines

```bash
# Define a hypothetical engine inline (name:thrust_kn:isp_s:mass_kg:propellant)
$ tsi optimize --payload 5000 --target-dv 9400 \
    --custom-engine "SuperEngine:3000:380:2000:loxch4" --engine SuperEngine
```

### ASCII rocket diagram

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --diagram

      /\
     /  \
    /    \   <- Payload (5k kg)
   /______\
 |            |  <- Stage 2: Raptor-2 x1
 |     S2     |     29k kg
 |            |
 |____________|
 |            |  <- Stage 1: Raptor-2 x1
 |            |     138k kg
 |            |
 |     S1     |
 |            |
 |____________|
    \    /
     \  /
      \/
```

### Atmospheric loss estimation

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --show-losses

  Gravity losses:     1,459 m/s
  Drag losses:          212 m/s
  Steering losses:      100 m/s
  ──────────────────────────────
  Total losses:       1,771 m/s

  Ideal delta-v:      9,400 m/s
  After losses:       7,629 m/s
  LEO orbital v:      7,800 m/s
  Shortfall:            171 m/s (insufficient)
```

A liftoff TWR of 1.22 means a long, slow climb and heavy gravity losses. Try
`--min-twr 1.4` and compare.

### Multi-engine optimization

```bash
# Every engine is tried on every stage: hydrogen ends up on top
$ tsi optimize --payload 5000 --target-dv 9400 --engine merlin-1d,rl-10c --max-stages 3

# Pin an engine to a stage
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --stage1-engine merlin-1d

# Cross-check with exhaustive grid search
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --optimizer brute-force
```

### Monte Carlo uncertainty analysis

Real engines come off the line a percent or so off their rated Isp, and
structures come out heavier or lighter than drawn. `--monte-carlo` builds
your design many times with those errors. A design with no margin works about
half the time, so give it some:

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 \
    --margin 3 --monte-carlo 1000 --seed 1

  Design stressed:      2-stage, 220,098 kg (the solution above)
  Success probability:  100.0% (HIGH CONFIDENCE)
  Builds:               1000 (0 too heavy to lift off)
  Seed:                 1 (repeat with --seed 1)

  Confidence Intervals:
    5th %ile:     9,541 m/s  (worst case)
    50th %ile:    9,680 m/s  (median)
    95th %ile:    9,831 m/s  (best case)

# Higher uncertainty for development engines
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 \
    --monte-carlo 1000 --uncertainty high

# JSON output includes monte_carlo statistics
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 \
    --monte-carlo 100 --output json | jq '.monte_carlo.success_probability'
```

## Documentation

- [Getting Started](docs/getting-started.md)
- [Command Reference](docs/commands.md)
- [Engine Database](docs/engines.md)
- [Physics Reference](docs/physics.md)
- [Examples](docs/examples.md)

## Roadmap

- [x] v0.1 - Foundation (unit types, physics, basic CLI)
- [x] v0.2 - Engine database and enhanced calculations
- [x] v0.3 - Two-stage optimization (`tsi optimize`)
- [x] v0.4 - Multi-engine optimization with brute-force search
- [x] v0.5 - Monte Carlo uncertainty analysis
- [x] v0.6 - Polish (ASCII diagrams, shell completions, custom engines)
- [x] v0.7 - Static fire (optimizer correctness, CI)
- [ ] v0.8 - Stacking (library-first API)
- [ ] v0.9 - Wet dress (`--explain`, mission targets, real vehicles)
- [ ] v1.0 - Liftoff (API freeze, crates.io)

See [ROADMAP.md](ROADMAP.md) for the live plan.

## Contributing

Contributions welcome! Please read the existing code style and add tests for new functionality.

## License

MIT
