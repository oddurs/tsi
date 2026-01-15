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
