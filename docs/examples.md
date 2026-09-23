# Examples

Real-world examples and common use cases for `tsi`.

## Basic Calculations

### Single Engine Stage

Calculate performance for a stage with one Raptor-2 engine:

```bash
$ tsi calculate --engine raptor-2 --propellant-mass 100000
Engine:     Raptor-2
Propellant: 100,000 kg (LOX/CH4)
Dry mass:   11,600 kg
Δv:         7,771 m/s
Burn time:  2m 20s
TWR (vac):  2.24
```

### Multiple Engines

Simulate a Falcon 9-like first stage with 9 Merlin engines:

```bash
$ tsi calculate --engine merlin-1d --engine-count 9 --propellant-mass 400000
Engine:     Merlin-1D (×9)
Propellant: 400,000 kg (LOX/RP-1)
Dry mass:   44,230 kg
Δv:         7,036 m/s
Burn time:  2m 28s
TWR (vac):  1.89
```

### Manual Parameters

When you want to explore hypothetical configurations:

```bash
$ tsi calculate --isp 380 --mass-ratio 10
Δv:         8,570 m/s
Mass ratio: 10.00
```

### Quick Calculations

Use compact output for scripting or quick checks:

```bash
$ tsi calculate --engine raptor-2 --propellant-mass 100000 -o compact
Δv: 7,771 m/s | Burn: 140s | TWR: 2.24
```

## Comparing Engines

### All Methane Engines

```bash
$ tsi engines --propellant methane
NAME             PROPELLANT    THRUST(vac)   ISP(vac)       MASS
--------------------------------------------------------------
Raptor-2         LOX/CH4           2,450 kN      350s      1,600 kg
Raptor-Vacuum    LOX/CH4           2,550 kN      380s      1,600 kg
BE-4             LOX/CH4           2,600 kN      340s      2,000 kg
```

### High-Isp Engines

Filter for hydrogen engines (highest Isp):

```bash
$ tsi engines --propellant hydrogen
NAME             PROPELLANT    THRUST(vac)   ISP(vac)       MASS
--------------------------------------------------------------
RS-25            LOX/LH2           2,279 kN      452s      3,527 kg
RL-10C           LOX/LH2             106 kN      453s        190 kg
J-2              LOX/LH2           1,033 kN      421s      1,788 kg
```

### Detailed Comparison

Use verbose output to see sea-level performance:

```bash
$ tsi engines --name raptor --verbose
NAME             PROPELLANT   THRUST(vac) THRUST(sl) ISP(vac)  ISP(sl)       MASS
------------------------------------------------------------------------------------
Raptor-2         LOX/CH4         2,450 kN   2,256 kN     350s     327s      1,600 kg
Raptor-Vacuum    LOX/CH4         2,550 kN          -     380s        -      1,600 kg
```

## Staging Optimization

### Basic LEO Mission

Optimize a two-stage rocket to reach Low Earth Orbit (9,400 m/s):

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
═══════════════════════════════════════════════════════════════
  tsi — Staging Optimization Complete
═══════════════════════════════════════════════════════════════

  Target Δv:  9,400 m/s    Payload:  5,000 kg
  Solution:   2-stage    Total mass:  186,599 kg

  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 2 (upper)                                            │
  │  Engine:     Raptor-2 (×1)                                  │
  │  Propellant: 28,819 kg (LOX/CH4)                            │
  │  Dry mass:   3,906 kg                                       │
  │  Δv:         4,955 m/s                                      │
  │  Burn time:  40.4s                                          │
  │  TWR:        6.62 at ignition                               │
  └─────────────────────────────────────────────────────────────┘
  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 1 (booster)                                          │
  │  Engine:     Raptor-2 (×1)                                  │
  │  Propellant: 136,365 kg (LOX/CH4)                           │
  │  Dry mass:   12,509 kg                                      │
  │  Δv:         4,445 m/s                                      │
  │  Burn time:  3m 11s                                         │
  │  TWR:        1.23 at liftoff                                │
  └─────────────────────────────────────────────────────────────┘

  Total propellant:  165,184 kg
  Total dry mass:    16,415 kg
  Total burn time:   231s

  Payload fraction:  2.68%
  Delta-v margin:    -0 m/s (-0.0%)
  Booster Isp:       345s (ascent-averaged); upper stages use vacuum Isp

  Optimizer: Analytical (181 configs)

═══════════════════════════════════════════════════════════════
```

Textbook staging theory says identical stages should split delta-v equally.
tsi gives the booster less: from sea level its Raptor averages 345 s rather
than 350 s, and a single 1.6 t engine is a bigger burden on the small upper
stage than on the booster.

### Higher Payload with Merlin Engines

```bash
$ tsi optimize --payload 10000 --target-dv 9400 --engine merlin-1d
```

### Let tsi Choose the Engines

Offer several engines and up to three stages. Every engine is tried on every
stage:

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine merlin-1d,rl-10c --max-stages 3
```

The answer is a kerosene booster under two hydrogen stages. Hydrogen's 450 s
wins in vacuum but can't lift off: the RL-10C has no sea-level rating at all.

### Pin an Engine to a Stage

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --stage1-engine merlin-1d
```

### Custom TWR Constraints

Increase minimum liftoff TWR for a more aggressive ascent:

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --min-twr 1.5
```

Compare `--show-losses` with and without it: a faster climb spends less
delta-v fighting gravity.

### Launch from Mars

```bash
$ tsi optimize --payload 5000 --target-dv 4500 --engine raptor-2 --gravity mars
```

Mars gravity is 38% of Earth's, so each engine lifts 2.6 times as much, and
the thin atmosphere means the first stage gets full vacuum Isp. The same
engine builds a much lighter rocket.

### Design Margin and Monte Carlo

tsi sizes rockets to hit the target exactly. Real hardware varies, so check
how often the design would actually work, then add margin:

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --monte-carlo 10000
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --margin 3 --monte-carlo 10000
```

The first reports about 50% success, the second nearly 100%.

### JSON Output for Scripting

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --output json | jq '.payload_fraction'
0.0267954474962633
```

## Real Rocket Approximations

### Falcon 9 First Stage

```bash
$ tsi calculate --engine merlin-1d --engine-count 9 --propellant-mass 411000 --structural-ratio 0.054
Engine:     Merlin-1D (×9)
Propellant: 411,000 kg (LOX/RP-1)
Dry mass:   26,424 kg
Δv:         8,527 m/s
Burn time:  2m 31s
TWR (vac):  1.97
```

### Falcon 9 Second Stage

```bash
$ tsi calculate --engine merlin-vacuum --propellant-mass 111500 --structural-ratio 0.036
Engine:     Merlin-Vacuum
Propellant: 111,500 kg (LOX/RP-1)
Dry mass:   4,484 kg
Δv:         11,140 m/s
Burn time:  6m 27s
TWR (vac):  2.23
```

### Saturn V S-IC (First Stage)

```bash
$ tsi calculate --engine f-1 --engine-count 5 --propellant-mass 2160000 --structural-ratio 0.06
Engine:     F-1 (×5)
Propellant: 2,160,000 kg (LOX/RP-1)
Dry mass:   171,600 kg
Δv:         7,550 m/s
Burn time:  2m 27s
TWR (vac):  2.33
```

## Scripting Examples

### Parameter Sweep

Compare delta-v across different propellant masses:

```bash
for mass in 50000 100000 150000 200000; do
  echo -n "$mass kg: "
  tsi calculate --engine raptor-2 --propellant-mass $mass -o compact
done
```

Output:
```
50000 kg: Δv: 5,918 m/s | Burn: 70s | TWR: 4.21
100000 kg: Δv: 7,771 m/s | Burn: 140s | TWR: 2.24
150000 kg: Δv: 8,753 m/s | Burn: 210s | TWR: 1.53
200000 kg: Δv: 9,402 m/s | Burn: 280s | TWR: 1.16
```

### Engine Comparison Script

```bash
for engine in merlin-1d raptor-2 rs-25; do
  echo "=== $engine ==="
  tsi calculate --engine $engine --propellant-mass 100000
  echo
done
```

### JSON Processing

Export engine data for further analysis:

```bash
tsi engines --output json | jq '.[] | select(.propellant == "LoxCh4") | .name'
```

Output:
```
"Raptor-2"
"Raptor-Vacuum"
"BE-4"
```

## Understanding the Results

### Why does my TWR seem low?

Remember that `tsi` calculates vacuum TWR. Sea-level TWR at launch would be lower due to:
- Atmospheric pressure reducing effective thrust
- Sea-level Isp being lower than vacuum Isp

### Why is burn time different than expected?

Burn time assumes:
- Constant thrust (no throttling)
- All propellant consumed
- Vacuum conditions

Real rockets throttle, retain reserves, and face varying conditions.

### Structural ratio impact

Lower structural ratios dramatically improve delta-v:

```bash
$ tsi calculate --engine raptor-2 --propellant-mass 100000 --structural-ratio 0.05
Δv:         8,570 m/s

$ tsi calculate --engine raptor-2 --propellant-mass 100000 --structural-ratio 0.15
Δv:         7,036 m/s
```

A 10% difference in structural ratio yields ~1,500 m/s difference in delta-v.
