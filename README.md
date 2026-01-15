# tsi

A command-line tool for rocket staging analysis and optimization.

Named after [Konstantin Tsiolkovsky](https://en.wikipedia.org/wiki/Konstantin_Tsiolkovsky), the father of astronautics who derived the fundamental rocket equation in 1903.

## Features

- **Stage performance calculations** - Delta-v, burn time, TWR
- **Built-in engine database** - 11 real rocket engines with accurate specs
- **Type-safe physics** - Compile-time unit safety prevents calculation errors
- **Scriptable output** - JSON and compact formats for automation

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

### Compact output for scripting

```bash
$ tsi calculate --engine raptor-2 --propellant-mass 100000 -o compact
Δv: 7,771 m/s | Burn: 140s | TWR: 2.24
```

## Commands

| Command | Description |
|---------|-------------|
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

## Documentation

- [Getting Started](docs/getting-started.md)
- [Command Reference](docs/commands.md)
- [Engine Database](docs/engines.md)
- [Physics Reference](docs/physics.md)
- [Examples](docs/examples.md)

## Roadmap

- [x] v0.1 - Foundation (unit types, physics, basic CLI)
- [x] v0.2 - Engine database and enhanced calculations
- [ ] v0.3 - Multi-stage optimization (`tsi optimize`)
- [ ] v0.4 - Multiple engine types per rocket
- [ ] v0.5 - Monte Carlo uncertainty analysis

See [docs/plan/roadmap.md](docs/plan/roadmap.md) for detailed plans.

## Contributing

Contributions welcome! Please read the existing code style and add tests for new functionality.

## License

MIT
