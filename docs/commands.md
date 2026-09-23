# Command Reference

## Global Options

```
-h, --help     Print help
-V, --version  Print version
```

---

## tsi calculate

Calculate delta-v and performance metrics for a single rocket stage.

### Usage

```bash
tsi calculate [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--engine <NAME>` | Engine name from database (e.g., raptor-2, merlin-1d) |
| `--engine-count <N>` | Number of engines [default: 1] |
| `--isp <SECONDS>` | Specific impulse (required if --engine not provided) |
| `--propellant-mass <KG>` | Propellant mass in kg |
| `--mass-ratio <RATIO>` | Mass ratio (wet/dry) |
| `--wet-mass <KG>` | Wet mass in kg (requires --dry-mass) |
| `--dry-mass <KG>` | Dry mass in kg (requires --wet-mass) |
| `--thrust <N>` | Thrust in Newtons (overrides engine thrust) |
| `--structural-ratio <R>` | Structural mass / propellant mass [default: 0.1] |
| `-o, --output <FORMAT>` | Output format: pretty, compact [default: pretty] |

### Input Modes

You must provide either:
1. `--engine` with `--propellant-mass`
2. `--isp` with mass information (`--mass-ratio` or `--wet-mass`/`--dry-mass`)

### Examples

```bash
# Single Raptor-2 with 100 tonnes of propellant
tsi calculate --engine raptor-2 --propellant-mass 100000

# Falcon 9 first stage approximation (9 Merlin-1D engines)
tsi calculate --engine merlin-1d --engine-count 9 --propellant-mass 400000

# Manual calculation with Isp and mass ratio
tsi calculate --isp 311 --mass-ratio 3.5

# Using wet/dry mass directly
tsi calculate --isp 350 --wet-mass 100000 --dry-mass 10000

# Compact one-line output
tsi calculate --engine raptor-2 --propellant-mass 100000 -o compact
```

### Output Fields

| Field | Description |
|-------|-------------|
| Engine | Engine name and count |
| Propellant | Propellant mass and type |
| Dry mass | Stage dry mass (structure + engines) |
| Δv | Delta-v achievable |
| Burn time | Time to consume all propellant |
| TWR (vac) | Vacuum thrust-to-weight ratio at ignition |

---

## tsi engines

List available rocket engines in the database.

### Usage

```bash
tsi engines [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `-o, --output <FORMAT>` | Output format: table, json [default: table] |
| `-p, --propellant <TYPE>` | Filter by propellant type |
| `-n, --name <PATTERN>` | Filter by name (case-insensitive substring) |
| `-v, --verbose` | Show sea-level values (thrust_sl, isp_sl) |

### Propellant Filters

The `--propellant` filter accepts various formats:

| Propellant | Accepted filters |
|------------|-----------------|
| LOX/RP-1 | `loxrp1`, `kerosene`, `rp1`, `rp-1` |
| LOX/LH2 | `loxlh2`, `hydrogen`, `lh2`, `hydrolox` |
| LOX/CH4 | `loxch4`, `methane`, `ch4`, `methalox` |
| N2O4/UDMH | `n2o4udmh`, `hypergolic`, `udmh` |
| Solid | `solid`, `srb` |

### Examples

```bash
# List all engines
tsi engines

# Filter by propellant type
tsi engines --propellant methane
tsi engines --propellant hydrogen

# Filter by name
tsi engines --name raptor
tsi engines --name merlin

# Verbose output with sea-level values
tsi engines --verbose

# JSON output for scripting
tsi engines --output json

# Combined filters
tsi engines --propellant kerosene --verbose
```

### Output Fields

**Standard output:**
- NAME - Engine name
- PROPELLANT - Propellant type
- THRUST(vac) - Vacuum thrust in kN
- ISP(vac) - Vacuum specific impulse in seconds
- MASS - Engine dry mass in kg

**Verbose output (adds):**
- THRUST(sl) - Sea-level thrust in kN
- ISP(sl) - Sea-level specific impulse in seconds

---

## tsi optimize

Find the lightest rocket that delivers a payload with a given delta-v.

### Usage

```bash
tsi optimize [OPTIONS] --payload <KG> --target-dv <M/S> --engine <NAMES>
```

### Options

| Option | Description |
|--------|-------------|
| `-p, --payload <KG>` | Payload mass in kg (required) |
| `-d, --target-dv <M/S>` | Target delta-v in m/s (required) |
| `-e, --engine <NAMES>` | Engines to choose from, comma-separated (required) |
| `--stage1-engine <NAME>` | Use this engine on the first stage |
| `--stage2-engine <NAME>` | Use this engine on the second stage |
| `--min-twr <RATIO>` | Minimum liftoff TWR [default: 1.2] |
| `--min-upper-twr <RATIO>` | Minimum upper-stage TWR at ignition [default: 0.5] |
| `--max-stages <N>` | Try every stage count from 1 to N [default: 2] |
| `--stages <N>` | Use exactly N stages |
| `--max-engines <N>` | Most engines on any one stage [default: 9] |
| `--structural-ratio <R>` | Structural mass / propellant mass [default: 0.08] |
| `--margin <PERCENT>` | Extra delta-v to design for, e.g. `2` or `2%` [default: 0] |
| `--gravity <BODY>` | Launch body: earth, mars, moon [default: earth] |
| `--optimizer <NAME>` | auto, analytical, brute-force [default: auto] |
| `--monte-carlo <N>` | Build the design N times with manufacturing errors |
| `--uncertainty <LEVEL>` | Monte Carlo error level: none, low, default, high |
| `--seed <N>` | Monte Carlo seed, for repeatable runs |
| `--diagram` | Draw the rocket |
| `--show-losses` | Estimate gravity and drag losses |
| `--custom-engine <SPEC>` | Define an engine inline: `name:thrust_kn:isp_s:mass_kg:propellant` |
| `--quiet` | Hide progress output |
| `-o, --output <FORMAT>` | pretty, json [default: pretty] |

### How it works

The analytical optimizer starts from the classical Lagrange multiplier
solution to the staging problem, which says identical stages should split
delta-v equally. It then refines the split against the exact mass model.
Engines have fixed mass, and an Earth-launched first stage has lower
effective Isp than it would in vacuum, so the real optimum usually puts more
delta-v on the upper stage. Each stage gets the fewest engines that meet its
TWR limit.

Every engine is tried on every stage, and with `--max-stages` every stage count
up to the maximum, so asking for `--engine merlin-1d,rl-10c` finds that the
hydrogen engine belongs on top.

A few physical rules apply:

- **Booster Isp.** An Earth launch's first stage uses Isp averaged over its
  climb through the atmosphere (302 s for a Merlin-1D, against 311 s in
  vacuum). Upper stages use vacuum Isp. On Mars and the Moon every stage uses
  vacuum Isp.
- **Liftoff TWR** uses sea-level thrust and the launch body's gravity.
- **First-stage floor.** On Earth, a first stage with a stage above it must
  deliver at least 2,000 m/s, enough to carry the upper stage above nearly all
  of the atmosphere before it lights.
- **No hidden margin.** The rocket is sized to hit the target exactly. Use
  `--margin` for headroom, and `--monte-carlo` to see how much you need.

`--optimizer brute-force` runs an exhaustive grid search instead. It is slower
and slightly less precise, and it shares none of the analytical optimizer's
search logic, which makes it a useful cross-check.

### Examples

```bash
# Two-stage methane rocket to LEO (9,400 m/s)
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2

# Let tsi choose engines per stage, and up to three stages
tsi optimize --payload 5000 --target-dv 9400 --engine merlin-1d,rl-10c,raptor-2 --max-stages 3

# Kerosene booster, methane upper stage
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --stage1-engine merlin-1d

# A Super Heavy class booster needs more than nine engines
tsi optimize --payload 100000 --target-dv 9400 --engine raptor-2 --max-engines 40

# Design in 3% margin and check it with Monte Carlo
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --margin 3 --monte-carlo 10000

# Launch from Mars
tsi optimize --payload 5000 --target-dv 4500 --engine raptor-2 --gravity mars

# JSON for scripting
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --output json
```

### Output

**Pretty output** shows each stage's engines, propellant, dry mass, delta-v,
burn time and TWR (at liftoff for stage 1, at ignition above it), then totals,
payload fraction, margin, and the booster Isp that was used.

**JSON output** fields:

- `target_delta_v_mps`, `total_delta_v_mps`, `payload_kg`, `total_mass_kg`
- `payload_fraction`, `margin_mps`, `margin_percent`, `design_margin_percent`
- `booster_isp_model`: `ascent-averaged` or `vacuum`
- `stages[]`: `engine`, `engine_count`, `propellant_kg`, `dry_mass_kg`,
  `wet_mass_kg`, `delta_v_mps`, `isp_s`, `burn_time_s`, `twr_ignition`, and
  `twr_liftoff` on stage 1
- `metadata`: `optimizer`, `iterations`, `runtime_ms`
- `monte_carlo` (with `--monte-carlo`): success probability, delta-v and mass
  distributions, required margin for 95% confidence, `seed`, and the mass and
  stage count of the design that was stressed

### Monte Carlo

`--monte-carlo N` builds the optimized design N times. Each time, every stage
gets random errors in Isp, thrust and structural mass, and tsi counts how many
builds still reach the target. A design with no margin succeeds about half
the time, because half of all builds come out below nominal. The output says
how much margin 95% confidence needs, and the seed to repeat the run.

### Example Output

```
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

---

## Error Handling

### Unknown Engine

If you specify an engine that doesn't exist, `tsi` will suggest similar names:

```
$ tsi calculate --engine raptor --propellant-mass 100000
Error: Unknown engine: 'raptor'

Did you mean:
  Raptor-2
  Raptor-Vacuum

Run `tsi engines` to see all available engines.
```

### Validation Errors

Multiple validation errors are reported at once:

```
$ tsi calculate --isp 300 --mass-ratio 0.5 --structural-ratio 2.0
Error: Invalid arguments:
  - --mass-ratio must be greater than 1.0 (wet > dry)
  - --structural-ratio must be between 0 and 1
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error: invalid values, unknown engine, or no feasible rocket |
| 2 | Usage error: unknown or missing flags (reported by the argument parser) |
