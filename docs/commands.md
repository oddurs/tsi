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

Optimize a two-stage rocket configuration to meet a delta-v target.

### Usage

```bash
tsi optimize [OPTIONS] --payload <KG> --target-dv <M/S> --engine <NAME>
```

### Options

| Option | Description |
|--------|-------------|
| `--payload <KG>` | Payload mass in kg (required) |
| `--target-dv <M/S>` | Target delta-v in m/s (required) |
| `--engine <NAME>` | Engine name from database (required) |
| `--min-twr <RATIO>` | Minimum first stage TWR [default: 1.2] |
| `--min-upper-twr <RATIO>` | Minimum upper stage TWR [default: 0.5] |
| `--max-stages <N>` | Maximum number of stages [default: 2] |
| `--structural-ratio <R>` | Structural mass / propellant mass [default: 0.08] |
| `--sea-level` | Use sea-level thrust/ISP for first stage TWR display |
| `--gravity <BODY>` | Surface gravity for TWR display: earth, mars, moon [default: earth] |
| `-o, --output <FORMAT>` | Output format: pretty, json [default: pretty] |

### Algorithm

The optimizer uses an analytical solution based on optimal staging theory:

1. **Equal delta-v split** - For identical engines, optimal staging splits delta-v equally between stages
2. **Iterative engine count** - Determines minimum engines per stage to meet TWR constraints
3. **2% margin** - Adds 2% to target delta-v for robustness

### Examples

```bash
# Basic two-stage rocket to LEO (9,400 m/s)
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2

# Higher payload with custom TWR constraint
tsi optimize --payload 10000 --target-dv 9400 --engine raptor-2 --min-twr 1.3

# Using Merlin-1D for Falcon 9-style vehicle
tsi optimize --payload 10000 --target-dv 8000 --engine merlin-1d

# Sea-level TWR for first stage (important for Earth launch)
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --sea-level

# Show TWR adjusted for Mars gravity
tsi optimize --payload 5000 --target-dv 5700 --engine raptor-2 --gravity mars

# JSON output for scripting
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --output json
```

### Output Fields

**Pretty output includes:**
- Target and achieved delta-v
- Total rocket mass
- Stage-by-stage breakdown with:
  - Engine name and count
  - Propellant mass and type
  - Dry mass
  - Stage delta-v
  - Burn time
  - TWR at ignition
- Payload fraction
- Delta-v margin (m/s and %)

**JSON output includes:**
- `target_delta_v_mps` - Target delta-v
- `total_mass_kg` - Total rocket mass
- `total_delta_v_mps` - Achieved delta-v
- `payload_fraction` - Payload / total mass
- `margin_mps` - Delta-v margin
- `stages[]` - Array of stage details

### Example Output

```
═══════════════════════════════════════════════════════════════
  tsi — Staging Optimization Complete
═══════════════════════════════════════════════════════════════

  Target Δv:  9,400 m/s    Payload:  5,000 kg
  Solution:   2-stage    Total mass:  205,430 kg

  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 2 (upper)                                            │
  │  Engine:     Raptor-2 (×1)                                  │
  │  Propellant: 26,534 kg (LOX/CH4)                            │
  │  Dry mass:   3,723 kg                                       │
  │  Δv:         4,794 m/s                                      │
  │  Burn time:  37.2s                                          │
  │  TWR:        7.09                                           │
  └─────────────────────────────────────────────────────────────┘
  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 1 (booster)                                          │
  │  Engine:     Raptor-2 (×2)                                  │
  │  Propellant: 154,605 kg (LOX/CH4)                           │
  │  Dry mass:   15,568 kg                                      │
  │  Δv:         4,794 m/s                                      │
  │  Burn time:  1m 48s                                         │
  │  TWR:        2.43                                           │
  └─────────────────────────────────────────────────────────────┘

  Payload fraction:  2.43%
  Delta-v margin:    +188 m/s (2.0%)

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
| 1 | User error (bad arguments, unknown engine) |
| 2 | No solution (infeasible problem) |
