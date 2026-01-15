# tsi — Concept

## The Idea

`tsi` is a command-line tool that answers a deceptively simple question:

> Given a payload I need to deliver and a delta-v budget I need to hit, what's the best way to stage my rocket?

You tell it what engines you have available, what constraints you're working under (minimum thrust-to-weight, maximum stages), and it tells you the optimal configuration — how many stages, which engines, how much propellant in each.

## The Gap

### What Exists Today

**Professional tools** — STK, GMAT, Rocket Propulsion Analysis. These are serious aerospace engineering tools. They're powerful, expensive, and have learning curves measured in weeks. You need them if you're actually building rockets. You don't need them if you're trying to understand rockets.

**Spreadsheets** — The most common tool for hobbyist rocket analysis. Everyone has their own Excel sheet with the Tsiolkovsky equation plugged in. They work, but they're fragile, hard to share, and tedious to iterate with.

**KSP calculators** — Web tools built for Kerbal Space Program players. Fun, but tied to KSP's fictional solar system and simplified physics.

**Online calculators** — Scattered single-purpose web pages. Calculate delta-v here, look up Isp there, plug numbers into a staging formula somewhere else. No integration, no iteration.

**Nothing in the terminal** — For a community that lives in the command line, there's no good CLI tool for rocket calculations. Want to script a parameter sweep? Want to pipe results into another tool? Want to version control your analysis? You're out of luck.

### What's Missing

A tool that is:

- **Fast to use** — Type a command, get an answer
- **Correct** — Real physics, real engine data, validated against known rockets
- **Composable** — JSON output, scriptable, pipe-friendly
- **Educational** — Shows its work, helps you understand why
- **Free and open** — No license fees, no signup, inspect the source

Something between a toy calculator and professional aerospace software.

## Who Is This For

### Primary: Space Enthusiasts

People who watch launches, read about rocket engines, argue about Starship vs SLS on Reddit. They understand the basics — delta-v, Isp, mass ratio — but don't have tools to explore "what if" questions easily.

Questions they ask:
- Why does Falcon 9 use 9 engines on the first stage?
- How much payload could you get to orbit with an all-Raptor rocket?
- What if you added a third stage to Saturn V?
- Why is hydrogen better for upper stages?

### Secondary: Students

Aerospace engineering students, physics students, anyone taking an intro to orbital mechanics course. They need to work problems, check their math, build intuition.

Use cases:
- Verify homework calculations
- Explore parameter sensitivity
- Generate examples for reports
- Understand staging trade-offs

### Tertiary: Hobbyist Rocket Builders

High-power rocketry hobbyists who design their own vehicles. Not orbital, but the same physics applies. They want to optimize staging for altitude records or specific flight profiles.

### Non-Audience

- Professional aerospace engineers (they have better tools)
- People who want a GUI (this is deliberately CLI)
- Anyone who needs regulatory compliance or safety certification

## Why a CLI

### Fits the Workflow

Developers and technical users already live in the terminal. A CLI tool:

- Integrates with existing workflows
- Can be scripted and automated
- Works over SSH
- Doesn't require a browser or display
- Runs the same everywhere

### Scriptability

```bash
# Parameter sweep
for payload in 1000 2000 5000 10000; do
  tsi optimize --payload $payload --target-dv 9400 --engine raptor-2 --output json
done | jq '.rocket.total_mass'

# Compare engines
for engine in merlin-1d raptor-2 rs-25; do
  echo "$engine: $(tsi optimize --payload 5000 --target-dv 9400 --engine $engine --output json | jq '.rocket.payload_fraction')"
done
```

### Reproducibility

```bash
# Save your analysis
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 > analysis.json

# Version control it
git add analysis.json
git commit -m "Baseline LEO config"

# Compare later
diff <(cat analysis.json) <(tsi optimize --payload 5000 --target-dv 9400 --engine raptor-3)
```

### Focused Scope

A CLI forces discipline. No temptation to add drag-and-drop, 3D visualization, or a plugin system. Just solve the core problem well.

## Why Rust

### Type Safety for Physics

Rocket calculations involve many quantities with units — mass, velocity, force, time. Mixing them up produces wrong answers silently. Excel won't stop you from adding kilograms to meters per second.

Rust's type system lets us make unit errors impossible:

```rust
let mass = Mass::kg(1000.0);
let velocity = Velocity::mps(3000.0);

// This won't compile — can't add mass to velocity
let nonsense = mass + velocity;  // ERROR

// This works — mass ratio is dimensionless
let ratio = wet_mass / dry_mass;  // Returns Ratio
```

The compiler catches bugs that would otherwise produce subtly wrong numbers.

### Performance for Optimization

Brute-force search over staging configurations evaluates millions of candidates. Monte Carlo uncertainty analysis runs thousands of simulations. Rust makes this fast without sacrificing safety.

### Single Binary Distribution

`cargo install tsi` gives you a single executable with no runtime dependencies. No Python virtual environments, no Node modules, no Java runtime. It just works.

### The Ecosystem

Rust has excellent libraries for exactly what we need:

- `clap` — Best-in-class CLI argument parsing
- `serde` — Effortless JSON/TOML serialization
- `rayon` — Trivial parallelism for Monte Carlo
- `ratatui` — Terminal UI if we want it later

## Core Principles

### Correctness Over Features

Physics must be right. A wrong answer presented beautifully is worse than no answer. Every calculation should be:

- Validated against textbook examples
- Checked against real rocket data
- Tested with property-based tests

### Show Your Work

When `tsi` produces a result, you should be able to understand why. Output includes:

- Per-stage breakdown
- Mass fractions and ratios
- Burn times and TWR
- Margin over target

### Sensible Defaults

The tool should be useful immediately:

```bash
# This should just work
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
```

Reasonable defaults for structural ratios, TWR constraints, stage limits. Override when you need to, but don't require configuration for common cases.

### Graceful Degradation

If optimization fails, explain why:

```
Error: No feasible solution found

The target delta-v (15,000 m/s) cannot be achieved with:
  - Payload: 10,000 kg
  - Engine: Merlin-1D
  - Max stages: 2
  - Min TWR: 1.2

Suggestions:
  - Allow more stages (--max-stages 3)
  - Use a higher-Isp engine for upper stage
  - Reduce payload mass
  - Lower minimum TWR constraint
```

### Honest Uncertainty

Real rockets don't match ideal calculations. `tsi` should acknowledge this:

- Ideal delta-v vs. losses (gravity, drag)
- Nominal vs. off-nominal performance
- Monte Carlo for uncertainty quantification

Don't pretend precision that doesn't exist.

## What Success Looks Like

### Short Term (v1.0)

- `cargo install tsi` works
- Calculate single-stage performance
- Optimize multi-stage rockets
- Ship with 10+ real engines
- JSON output for scripting
- Useful help text and error messages

### Medium Term (v2.0)

- Monte Carlo uncertainty analysis
- Atmospheric loss estimation
- Interactive TUI mode
- Custom engine definitions
- Shell completions

### Long Term (Aspirational)

- Trajectory simulation
- Launch window calculations
- Cost optimization
- Community engine database
- Educational mode with explanations

### Measure of Success

Not downloads or stars, but:

- Someone learns something about rockets by using it
- Someone verifies their coursework calculation
- Someone scripts a trade study that would have taken hours in Excel
- Someone forks it to build something cooler

## The Name

`tsi` — short for Tsiolkovsky.

Konstantin Tsiolkovsky (1857–1935) was a Russian scientist who derived the fundamental equation of rocketry — the relationship between delta-v, exhaust velocity, and mass ratio — before anyone had ever built a liquid-fueled rocket.

He worked in isolation, was largely deaf, and spent his career as a provincial schoolteacher. He imagined space stations, airlocks, and multi-stage rockets decades before they existed.

The tool is named for him because:

1. The rocket equation is the foundation of everything `tsi` calculates
2. He represents the spirit of working out the fundamentals from first principles
3. "tsi" is short, memorable, and easy to type

---

## Summary

`tsi` fills a gap between toy calculators and professional software. It's a correct, fast, scriptable CLI tool for rocket staging analysis — built for space enthusiasts, students, and hobbyists who want to explore "what if" questions from their terminal.

It's written in Rust for type safety and performance, distributed as a single binary, and designed around the Unix philosophy: do one thing well, compose with other tools, make the common case easy and the complex case possible.

The goal isn't to replace professional tools. It's to give curious people a way to play with real rocket physics without friction.
