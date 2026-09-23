# Examples

Real-world examples and common use cases for `tsi`.

## Basic Calculations

### Single Engine Stage

Calculate performance for a stage with one Raptor-2 engine:

```bash
$ tsi calculate --engine raptor-2 --propellant-mass 100000
tsi calculate  ·  Raptor-2, 100,000 kg propellant

  Δv           7,771 m/s   in vacuum, carrying nothing
  Mass ratio   9.62        111,600 kg wet, 11,600 kg dry
  Isp          350 s       vacuum, LOX/CH4
  Burn time    2m 20s      at full vacuum thrust, 2,450 kN
  TWR          2.24        vacuum thrust over fully loaded weight
```

### Multiple Engines

Simulate a Falcon 9-like first stage with 9 Merlin engines:

```bash
$ tsi calculate --engine merlin-1d --engine-count 9 --propellant-mass 400000
tsi calculate  ·  Merlin-1D × 9, 400,000 kg propellant

  Δv           7,036 m/s   in vacuum, carrying nothing
  Mass ratio   10.04       444,230 kg wet, 44,230 kg dry
  Isp          311 s       vacuum, LOX/RP-1
  Burn time    2m 28s      at full vacuum thrust, 8,226 kN
  TWR          1.89        vacuum thrust over fully loaded weight
```

### Manual Parameters

When you want to explore hypothetical configurations:

```bash
$ tsi calculate --isp 380 --mass-ratio 10
tsi calculate  ·  Isp 380 s, mass ratio 10.00

  Δv           8,581 m/s   in vacuum, carrying nothing
  Mass ratio   10.00       wet mass over dry mass
  Isp          380 s       vacuum
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
tsi engines  ·  3 engines matching

  Engine         Propellant  Thrust vac  Isp vac      Mass
  Raptor-2       LOX/CH4       2,450 kN    350 s  1,600 kg
  Raptor-Vacuum  LOX/CH4       2,550 kN    380 s  1,600 kg
  BE-4           LOX/CH4       2,600 kN    340 s  2,000 kg
```

### High-Isp Engines

Filter for hydrogen engines (highest Isp):

```bash
$ tsi engines --propellant hydrogen
tsi engines  ·  3 engines matching

  Engine  Propellant  Thrust vac  Isp vac      Mass
  RS-25   LOX/LH2       2,279 kN    452 s  3,527 kg
  RL-10C  LOX/LH2         106 kN    453 s    190 kg
  J-2     LOX/LH2       1,033 kN    421 s  1,788 kg
```

### Detailed Comparison

Use verbose output to see sea-level performance:

```bash
$ tsi engines --name raptor --verbose
tsi engines  ·  2 engines matching

  Engine         Propellant  Thrust vac  Isp vac  Thrust SL  Isp SL      Mass  T/W
  Raptor-2       LOX/CH4       2,450 kN    350 s   2,256 kN   327 s  1,600 kg  156
  Raptor-Vacuum  LOX/CH4       2,550 kN    380 s          —       —  1,600 kg  163

  · — : no sea-level rating; a vacuum engine, for upper stages only

  · T/W: vacuum thrust over the engine's own weight
```

## Staging Optimization

### Basic LEO Mission

Optimize a two-stage rocket to reach Low Earth Orbit (9,400 m/s):

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

These use each stage's real propellant load and the structural ratios in the
reference table on `Stage` (structure excluding engines, over propellant), so
the dry masses land on the published figures: 22.2 t, 4.0 t and 131 t. The
delta-v is each stage on its own, in vacuum, carrying nothing. Stacked, and
with the booster's Isp averaged over its climb through the air, Falcon 9's
two stages give about 9,400 m/s (`cargo run --example falcon9`).

### Falcon 9 First Stage

```bash
$ tsi calculate --engine merlin-1d --engine-count 9 --propellant-mass 411000 --structural-ratio 0.044
tsi calculate  ·  Merlin-1D × 9, 411,000 kg propellant

  Δv           9,047 m/s   in vacuum, carrying nothing
  Mass ratio   19.42       433,314 kg wet, 22,314 kg dry
  Isp          311 s       vacuum, LOX/RP-1
  Burn time    2m 32s      at full vacuum thrust, 8,226 kN
  TWR          1.94        vacuum thrust over fully loaded weight
```

### Falcon 9 Second Stage

```bash
$ tsi calculate --engine merlin-vacuum --propellant-mass 111500 --structural-ratio 0.032
tsi calculate  ·  Merlin-Vacuum, 111,500 kg propellant

  Δv           11,446 m/s   in vacuum, carrying nothing
  Mass ratio   28.61        115,538 kg wet, 4,038 kg dry
  Isp          348 s        vacuum, LOX/RP-1
  Burn time    6m 28s       at full vacuum thrust, 981 kN
  TWR          0.87         vacuum thrust over fully loaded weight
```

### Saturn V S-IC (First Stage)

```bash
$ tsi calculate --engine f-1 --engine-count 5 --propellant-mass 2160000 --structural-ratio 0.041
tsi calculate  ·  F-1 × 5, 2,160,000 kg propellant

  Δv           8,540 m/s   in vacuum, carrying nothing
  Mass ratio   17.54       2,290,560 kg wet, 130,560 kg dry
  Isp          304 s       vacuum, LOX/RP-1
  Burn time    2m 46s      at full vacuum thrust, 38,850 kN
  TWR          1.73        vacuum thrust over fully loaded weight
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
tsi calculate  ·  Raptor-2, 100,000 kg propellant

  Δv           9,549 m/s   in vacuum, carrying nothing
  Mass ratio   16.15       106,600 kg wet, 6,600 kg dry
  Isp          350 s       vacuum, LOX/CH4
  Burn time    2m 20s      at full vacuum thrust, 2,450 kN
  TWR          2.34        vacuum thrust over fully loaded weight

$ tsi calculate --engine raptor-2 --propellant-mass 100000 --structural-ratio 0.15
tsi calculate  ·  Raptor-2, 100,000 kg propellant

  Δv           6,691 m/s   in vacuum, carrying nothing
  Mass ratio   7.02        116,600 kg wet, 16,600 kg dry
  Isp          350 s       vacuum, LOX/CH4
  Burn time    2m 20s      at full vacuum thrust, 2,450 kN
  TWR          2.14        vacuum thrust over fully loaded weight
```

Going from 5% to 15% structure costs almost 2,900 m/s: a third of the way to orbit.
