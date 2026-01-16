# tsi

A command-line tool for rocket staging analysis and optimization.

Named after [Konstantin Tsiolkovsky](https://en.wikipedia.org/wiki/Konstantin_Tsiolkovsky), the father of astronautics who derived the fundamental rocket equation in 1903.

## Features

- **Multi-engine optimization** - Find optimal staging with mixed engine types
- **Automatic optimizer selection** - Fast analytical or exhaustive brute-force search
- **Stage performance calculations** - Delta-v, burn time, TWR
- **Built-in engine database** - 11 real rocket engines with accurate specs
- **Type-safe physics** - Compile-time unit safety prevents calculation errors
- **Scriptable output** - JSON and compact formats with optimization metadata

## Installation

```bash
# From source
git clone https://github.com/yourusername/tsi.git
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
  Solution:   2-stage    Total mass:  205,430 kg

  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 2 (upper)                                            │
  │  Engine:     Raptor-2 (×1)                                  │
  │  Propellant: 26,534 kg (LOX/CH4)                            │
  │  Δv:         4,794 m/s                                      │
  └─────────────────────────────────────────────────────────────┘
  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 1 (booster)                                          │
  │  Engine:     Raptor-2 (×2)                                  │
  │  Propellant: 154,605 kg (LOX/CH4)                           │
  │  Δv:         4,794 m/s                                      │
  └─────────────────────────────────────────────────────────────┘

  Payload fraction:  2.43%
  Delta-v margin:    +188 m/s (2.0%)

═══════════════════════════════════════════════════════════════
```

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

### Multi-engine optimization

```bash
# Let the optimizer find the best engine combination
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2,merlin-1d

# Force brute-force search for exhaustive exploration
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --optimizer brute-force
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
- [ ] v0.5 - Monte Carlo uncertainty analysis
- [ ] v0.6 - Polish (ASCII diagrams, shell completions)

See [docs/plan/roadmap.md](docs/plan/roadmap.md) for detailed plans.

## Contributing

Contributions welcome! Please read the existing code style and add tests for new functionality.

## License

MIT
