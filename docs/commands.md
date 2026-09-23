# Command Reference

## Global Options

```
    --color <WHEN>  Colour: auto, always, never [default: auto]
    --ascii         Plain ASCII instead of box-drawing and block characters
-h, --help          Print help
-V, --version       Print version
```

Global options go anywhere on the command line: `tsi --ascii engines` and
`tsi engines --ascii` are the same.

## Output

Every command can print for people or for programs.

**For people** (the default, `--output pretty`), output is laid out the same
way everywhere: a title line saying what was asked, then aligned fields,
tables, and charts. Names are in cyan, the numbers you came for in bold,
explanations dimmed, and verdicts green, yellow or red. Colour appears only
when writing to a terminal: it is off in pipes and files, when `NO_COLOR` is
set, and with `--color never`; `--color always` forces it. `--ascii` swaps
box and block characters (and symbols like × and Δ) for plain ASCII.

Charts show the shape of the answer at a glance:

- **Δv** and **Mass** bars in `tsi optimize` split the rocket's delta-v and
  liftoff mass between its stages and payload
- **losses** (`--show-losses`) as bars, gravity against drag against steering
- **Monte Carlo** (`--monte-carlo`) as a histogram of delta-v, with the
  target marked
- **the rocket** (`--diagram`), drawn with stage heights to scale

**For programs** (`--output json`), every command prints one JSON document
with the same envelope:

```json
{
  "schema_version": 1,
  "command": "optimize",
  ...
}
```

`schema_version` rises only when a field is removed or changes meaning; new
fields can appear without it. Field names carry their units (`delta_v_mps`,
`propellant_kg`).

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
| `-o, --output <FORMAT>` | pretty, compact (one line), json [default: pretty] |

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

### Output

```
$ tsi calculate --engine raptor-2 --propellant-mass 100000
tsi calculate  ·  Raptor-2, 100,000 kg propellant

  Δv           7,771 m/s   in vacuum, carrying nothing
  Mass ratio   9.62        111,600 kg wet, 11,600 kg dry
  Isp          350 s       vacuum, LOX/CH4
  Burn time    2m 20s      at full vacuum thrust, 2,450 kN
  TWR          2.24        vacuum thrust over fully loaded weight
```

Delta-v is the stage on its own, in vacuum, carrying nothing. TWR is vacuum
thrust over the fully loaded stage. With `--output json` the same numbers come
as `delta_v_mps`, `mass_ratio`, `isp_s`, `propellant_kg`, `dry_mass_kg`,
`wet_mass_kg`, `thrust_n`, `burn_time_s` and `twr_vacuum`, plus `engine`,
`engine_count` and `propellant` when an engine was named.

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
| `-o, --output <FORMAT>` | pretty, json [default: pretty; `table` also accepted] |
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

### Output

```
$ tsi engines --propellant hydrogen --verbose
tsi engines  ·  3 engines matching

  Engine  Propellant  Thrust vac  Isp vac  Thrust SL  Isp SL      Mass  T/W
  RS-25   LOX/LH2       2,279 kN    452 s   1,859 kN   366 s  3,527 kg   66
  RL-10C  LOX/LH2         106 kN    453 s          —       —    190 kg   57
  J-2     LOX/LH2       1,033 kN    421 s          —       —  1,788 kg   59

  · — : no sea-level rating; a vacuum engine, for upper stages only

  · T/W: vacuum thrust over the engine's own weight
```

`--verbose` adds sea-level thrust and Isp (a dash means the engine has no
sea-level rating: a vacuum engine, for upper stages only) and T/W, vacuum
thrust over the engine's own weight. JSON lists each engine's `name`,
`propellant`, `thrust_sl`, `thrust_vac` (N), `isp_sl`, `isp_vac` (s) and
`dry_mass` (kg) under `"engines"`.

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
  climb through the atmosphere (305 s for a Merlin-1D, against 311 s in
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

**Pretty output** starts with the liftoff mass and payload fraction, then a
table of stages (engines, propellant, dry mass, Isp, delta-v, and TWR at
liftoff for stage 1 and at ignition above it), bars showing how delta-v and
mass are shared between stages, and the margin, booster Isp and optimizer.

**JSON output** follows a versioned schema. `schema_version` rises only when
a field is removed or changes meaning; new fields can appear without it.

#### Schema version 1

| Field | Type | Meaning |
|-------|------|---------|
| `schema_version` | integer | `1` |
| `command` | string | `"optimize"` |
| `target_delta_v_mps` | number | Delta-v asked for |
| `design_margin_percent` | number | Margin designed in (`--margin`) |
| `total_delta_v_mps` | number | Delta-v the rocket delivers |
| `margin_mps`, `margin_percent` | number | Delivered minus target |
| `payload_kg`, `total_mass_kg` | number | Payload, and liftoff mass including it |
| `payload_fraction` | number | Payload / liftoff mass (0-1) |
| `booster_isp_model` | string | `ascent-averaged` or `vacuum` |
| `surface_gravity_mps2` | number | Gravity TWR is quoted against |
| `stages` | array | One object per stage, first stage first |
| `metadata` | object | `optimizer`, `iterations`, `runtime_ms` |
| `monte_carlo` | object | Only with `--monte-carlo`; see below |

Each entry of `stages`:

| Field | Type | Meaning |
|-------|------|---------|
| `stage` | integer | 1 = first stage |
| `engine`, `engine_count` | string, integer | Engine name and how many |
| `propellant_kg`, `structural_mass_kg` | number | Propellant; structure excluding engines |
| `dry_mass_kg`, `wet_mass_kg` | number | Structure plus engines; plus propellant |
| `delta_v_mps` | number | Delta-v this stage delivers, carrying everything above it |
| `isp_s` | number | Effective Isp for this stage's burn |
| `burn_time_s` | number | Burn time at full thrust |
| `twr_ignition` | number | Vacuum TWR at this stage's ignition |
| `twr_liftoff` | number | Liftoff TWR (first stage only) |

`monte_carlo`:

| Field | Type | Meaning |
|-------|------|---------|
| `success_probability` | number | Fraction of builds reaching the target that can lift off |
| `total_runs`, `successes`, `failures` | integer | Builds; successes; too heavy to lift off |
| `delta_v`, `mass` | object | `mean`, `std_dev`, `min`, `max`, `percentile_5`, `percentile_50`, `percentile_95` |
| `required_margin_95_mps` | number | Margin needed for 95% confidence |
| `seed` | integer | Repeats the run with `--seed` |
| `design_total_mass_kg`, `design_stage_count` | number, integer | The design that was stressed |
| `target_delta_v_mps`, `runtime_ms` | number | |

Library users get the same document by serializing a `Solution` (which
includes `schema_version`) and `MonteCarloResults`, both of which implement
`serde::Serialize`. Only `design_margin_percent` is added by the CLI, since
the margin belongs to the problem rather than the solution.

### Monte Carlo

`--monte-carlo N` builds the optimized design N times. Each time, every stage
gets random errors in Isp, thrust and structural mass, and tsi counts how many
builds still reach the target. A design with no margin succeeds about half
the time, because half of all builds come out below nominal. The output says
how much margin 95% confidence needs, and the seed to repeat the run.

### Example Output

```
$ tsi optimize --payload 5000 --target-dv 9400 --engine merlin-1d,rl-10c --max-stages 3 --diagram
  The rocket

     ╱╲      payload    5,000 kg
    ╱  ╲
   ┌────┐
   │    │
   │ S3 │    stage 3    1 × RL-10C       8.5 t propellant   3,982 m/s
   ├────┤
   │    │
   │    │
   │ S2 │    stage 2    2 × RL-10C      18.9 t propellant   3,418 m/s
   │    │
   ├────┤
   │    │
   │    │
   │ S1 │    stage 1    2 × Merlin-1D   37.1 t propellant   2,000 m/s
   │    │
   │    │
   └┬──┬┘
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
