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

# Or from crates.io (when published). The crate is `tsiolkovsky`;
# the command it installs is `tsi`.
cargo install tsiolkovsky
```

## Quick Start

### Calculate stage performance

```bash
# Using an engine from the database
$ tsi calculate --engine raptor-2 --propellant-mass 100000
tsi calculate  ·  Raptor-2, 100,000 kg propellant

  Δv           7,771 m/s   in vacuum, carrying nothing
  Mass ratio   9.62        111,600 kg wet, 11,600 kg dry
  Isp          350 s       vacuum, LOX/CH4
  Burn time    2m 20s      at full vacuum thrust, 2,450 kN
  TWR          2.24        vacuum thrust over fully loaded weight

# Using manual parameters
$ tsi calculate --isp 350 --mass-ratio 8.0
tsi calculate  ·  Isp 350 s, mass ratio 8.00

  Δv           7,137 m/s   in vacuum, carrying nothing
  Mass ratio   8.00        wet mass over dry mass
  Isp          350 s       vacuum
```

### List available engines

```bash
$ tsi engines
tsi engines  ·  11 engines

  Engine         Propellant  Thrust vac  Isp vac      Mass
  Merlin-1D      LOX/RP-1        914 kN    311 s    470 kg
  Merlin-Vacuum  LOX/RP-1        981 kN    348 s    470 kg
  Raptor-2       LOX/CH4       2,450 kN    350 s  1,600 kg
  ...

# Filter by propellant
$ tsi engines --propellant methane
tsi engines  ·  3 engines matching

  Engine         Propellant  Thrust vac  Isp vac      Mass
  Raptor-2       LOX/CH4       2,450 kN    350 s  1,600 kg
  Raptor-Vacuum  LOX/CH4       2,550 kN    380 s  1,600 kg
  BE-4           LOX/CH4       2,600 kN    340 s  2,000 kg
```

### Optimize a two-stage rocket

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
tsi optimize  ·  5,000 kg to 9,400 m/s

  Liftoff   186,599 kg   2 stages, 3m 51s of burning
  Payload   5,000 kg     2.68% of the liftoff mass

  Stage      Engines       Propellant   Dry mass    Isp         Δv  TWR
  1 booster  1 × Raptor-2  136,365 kg  12,509 kg  345 s  4,445 m/s  1.23 liftoff
  2 upper    1 × Raptor-2   28,819 kg   3,906 kg  350 s  4,955 m/s  6.62 ignition
  ───────────────────────────────────────────────────────────────────────────────
  total                    165,184 kg  16,415 kg         9,400 m/s

  Δv     ███████████████████████▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓   9,400 m/s
         █ stage 1 47%   ▓ stage 2 53%

  Mass   ██████████████████████████████████████▓▓▓▓▓▓▓▓▓▒   186,599 kg
         █ stage 1 80%   ▓ stage 2 18%   ▒ payload 3%

  Margin        0 m/s        hits the target exactly; --margin adds headroom
  Booster Isp   345 s        stage 1, averaged over the climb (350 s in vacuum)
  Optimizer     Analytical   181 configurations evaluated
```

The booster gets a little less delta-v than the upper stage: from sea level
its Raptor delivers 345 s rather than 350 s, and its engine mass counts for
less on a big stage. Ask for headroom with `--margin 2%`.

### Output for people and for programs

Output is coloured in a terminal and plain in pipes; `NO_COLOR` and
`--color never` turn colour off, `--ascii` swaps box and block characters for
plain ASCII. Every command also speaks JSON, in the same envelope:

```bash
$ tsi calculate --engine raptor-2 --propellant-mass 100000 -o json | jq '{command, delta_v_mps}'
{
  "command": "calculate",
  "delta_v_mps": 7770.500979171574
}
```

### Compact output for scripting

```bash
$ tsi calculate --engine raptor-2 --propellant-mass 100000 -o compact
Δv: 7,771 m/s | Burn: 140s | TWR: 2.24
```

## As a library

`tsi` is built on the `tsiolkovsky` crate, which you can use directly. Turn
off the default `cli` feature to leave out the command-line dependencies:

```toml
[dependencies]
tsiolkovsky = { version = "0.8", default-features = false }
```

```rust
use tsiolkovsky::prelude::*;

let db = EngineDatabase::builtin();
let problem = Problem::builder()
    .payload(Mass::tonnes(5.0))
    .target(Velocity::mps(9_400.0))
    .engines([db.get("merlin-1d").unwrap().clone(), db.get("rl-10c").unwrap().clone()])
    .constraints(Constraints::default().with_margin(0.02))
    .build()?;

let solution = AnalyticalOptimizer.optimize(&problem)?;
for stage in solution.rocket().stages() {
    println!("{} × {}", stage.engine_count(), stage.engine().name());
}

// How often would it work, built with real manufacturing errors?
let results = MonteCarloRunner::new(Uncertainty::default()).run_design(&solution, 10_000)?;
println!("{:.1}% of builds reach orbit", results.success_probability() * 100.0);
```

Everything serializes with serde: a `Solution` serializes to the document
`tsi optimize --output json` prints, versioned by `schema_version`, less the
CLI's `design_margin_percent` (a property of the problem, not the solution). See [`examples/`](examples/) for
more, starting with `cargo run --example falcon9`.

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

### Falcon 9 first stage

Real propellant load, and a structural ratio (structure excluding engines,
over propellant) of 4.4%, which lands on the real 22.2 t dry mass:

```bash
$ tsi calculate --engine merlin-1d --engine-count 9 --propellant-mass 411000 --structural-ratio 0.044
tsi calculate  ·  Merlin-1D × 9, 411,000 kg propellant

  Δv           9,047 m/s   in vacuum, carrying nothing
  Mass ratio   19.42       433,314 kg wet, 22,314 kg dry
  Isp          311 s       vacuum, LOX/RP-1
  Burn time    2m 32s      at full vacuum thrust, 8,226 kN
  TWR          1.94        vacuum thrust over fully loaded weight
```

That is the stage on its own, in vacuum. Carrying the second stage and 22.8 t
of payload from sea level, it delivers about 3,800 m/s
(`cargo run --example falcon9`).

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

### Rocket diagram

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --diagram
  The rocket

     ╱╲      payload    5,000 kg
    ╱  ╲
   ┌────┐
   │    │
   │ S2 │    stage 2    1 × Raptor-2    28.8 t propellant   4,955 m/s
   ├────┤
   │    │
   │    │
   │ S1 │    stage 1    1 × Raptor-2   136.4 t propellant   4,445 m/s
   │    │
   │    │
   └┬──┬┘
```

### Atmospheric loss estimation

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --show-losses
  Losses on the way to low Earth orbit

  Gravity    1,434 m/s   ████████████████████████████████████████████████
  Drag         211 m/s   ███████
  Steering     100 m/s   ███▍

  Ideal Δv   9,400 m/s
  Losses     −1,745 m/s   first-order estimates; see docs/physics.md
  Left       7,655 m/s    145 m/s short of orbital velocity
  Orbit      7,800 m/s    circular, 200 km
```

A liftoff TWR of 1.23 means a long, slow climb and heavy gravity losses. Try
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
  Monte Carlo: this design built 1,000 times

  Success       100.0%              reach 9,400 m/s, confident
  Delta-v       9,541 – 9,830 m/s   5th to 95th percentile, median 9,680
  Uncertainty   Isp ±1%, thrust ±2%, structure ±5%  (1σ, each stage)
  Seed          1                   repeat with --seed 1

                   ▂▁▃█▃▇▅▅▅▃ ▄
        ▁▁▁▃▂▅▄▃▇▆▇██████████▇██▇▆▄▄▄▂▃▁▂▁▁▁▁ ▁
  ▲───────────────────────────────────────────────
  9,400                                  9,984 m/s

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
- [Examples](docs/examples.md) and runnable [example programs](examples/)
- [Architecture](docs/plan/architecture.md) and [testing](docs/plan/testing.md), for contributors
- [CHANGELOG](CHANGELOG.md), including the 0.7 to 0.8 migration table

## Roadmap

- [x] v0.1 - Foundation (unit types, physics, basic CLI)
- [x] v0.2 - Engine database and enhanced calculations
- [x] v0.3 - Two-stage optimization (`tsi optimize`)
- [x] v0.4 - Multi-engine optimization with brute-force search
- [x] v0.5 - Monte Carlo uncertainty analysis
- [x] v0.6 - Polish (ASCII diagrams, shell completions, custom engines)
- [x] v0.7 - Static fire (optimizer correctness, CI)
- [x] v0.8 - Stacking (library-first API)
- [ ] v0.9 - Wet dress (`--explain`, mission targets, real vehicles)
- [ ] v1.0 - Liftoff (API freeze, crates.io)

See [ROADMAP.md](ROADMAP.md) for the live plan.

## Contributing

Contributions welcome! Please read the existing code style and add tests for new functionality.

## License

MIT
