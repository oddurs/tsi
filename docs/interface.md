# tsi — Interface Design

## Philosophy

`tsi` is a CLI tool. The interface is text — arguments in, formatted output out. But "just text" doesn't mean "no design." Good CLI UX is the difference between a tool people reach for and one they forget exists.

### Guiding Principles

1. **Progressive disclosure** — Simple things simple, complex things possible
2. **No surprises** — Behave like other Unix tools
3. **Scannable output** — Key information visible at a glance
4. **Machine-friendly** — JSON output for scripting, exit codes for automation
5. **Helpful errors** — Tell users what went wrong and how to fix it

---

## Command Structure

### Top-Level

```
tsi <command> [options]
```

Three commands, each with a clear purpose:

| Command | Purpose | Complexity |
|---------|---------|------------|
| `calculate` | Single-stage analysis | Simple |
| `optimize` | Multi-stage optimization | Medium |
| `engines` | List available engines | Reference |

No subcommand nesting. No `tsi rocket stage calculate`. Flat and obvious.

### Command Discovery

```bash
$ tsi
tsi - Rocket staging optimizer

Usage: tsi <COMMAND>

Commands:
  calculate  Compute delta-v for a single stage
  optimize   Find optimal staging configuration
  engines    List available rocket engines

Run `tsi <command> --help` for details.
```

Typing `tsi` alone shows what's available. No error, no wall of text.

---

## Argument Design

### Naming Conventions

**Long flags are words:**
```bash
--payload          # Not --p or --pl
--target-dv        # Not --tdv
--engine           # Not --eng
--monte-carlo      # Not --mc
```

**Short flags for frequent options:**
```bash
-o, --output       # Output format
-h, --help         # Help
-V, --version      # Version
```

**Units in flag names where ambiguous:**
```bash
--payload <KG>           # Kilograms
--target-dv <M/S>        # Meters per second
--burn-time <S>          # Seconds
```

### Required vs Optional

Required arguments have no defaults — you must provide them:
```bash
tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
             ^^^^^^^^^^^^^^ ^^^^^^^^^^^^^^^^ ^^^^^^^^^^^^^^^^^
             required       required         required
```

Optional arguments have sensible defaults:
```bash
--min-twr 1.2            # Default: 1.2
--max-stages 3           # Default: 3
--structural-ratio 0.1   # Default: 0.1
--output pretty          # Default: pretty
```

### Argument Grouping

Related arguments cluster together in help text:

```
Target:
  --payload <KG>         Payload mass to deliver
  --target-dv <M/S>      Required delta-v

Engines:
  --engine <NAME>        Single engine for all stages
  --engines <LIST>       Comma-separated engine names

Constraints:
  --min-twr <RATIO>      Minimum thrust-to-weight [default: 1.2]
  --max-stages <N>       Maximum stage count [default: 3]
  --structural-ratio <R> Structure/propellant ratio [default: 0.1]

Output:
  -o, --output <FORMAT>  Output format: pretty, json [default: pretty]
```

---

## Input Patterns

### Single Values

```bash
--payload 5000
--engine raptor-2
--min-twr 1.2
```

### Lists

Comma-separated, no spaces:
```bash
--engines merlin-1d,raptor-2,rl-10c
```

### Alternatives

Some arguments accept either raw values or named references:

```bash
# By name (from database)
--engine raptor-2

# By value (custom)
--isp 350 --thrust 2000000

# Either works for propellant mass
--propellant-mass 100000
--mass-ratio 8.5
```

### Flags (Boolean)

```bash
--verbose          # Enable verbose output
--monte-carlo 1000 # Not a flag — takes a value
```

---

## Output Design

### The Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│  HEADER — What was computed                                 │
├─────────────────────────────────────────────────────────────┤
│  SUMMARY — Key results at a glance                          │
├─────────────────────────────────────────────────────────────┤
│  DETAILS — Stage-by-stage breakdown                         │
├─────────────────────────────────────────────────────────────┤
│  FOOTER — Margins, warnings, notes                          │
└─────────────────────────────────────────────────────────────┘
```

Users scan top-to-bottom. Most important information first.

### Pretty Output (Default)

```
═══════════════════════════════════════════════════════════════
  tsi v0.1.0 — Staging Optimization Complete
═══════════════════════════════════════════════════════════════

  Target Δv:  9,400 m/s    Payload:  5,000 kg
  Solution:   2-stage      Total mass:  142,300 kg

  ┌─────────────────────────────────────────────────────────────┐
  │  STAGE 2 (upper)                                            │
  │  Engine:     Raptor-2 (×1)                                  │
  │  Propellant: 18,200 kg (LOX/CH4)                            │
  │  Dry mass:   2,100 kg                                       │
  │  Δv:         3,840 m/s                                      │
  │  Burn time:  142 s                                          │
  │  TWR:        0.8 → 2.1 (vacuum)                             │
  ├─────────────────────────────────────────────────────────────┤
  │  STAGE 1 (booster)                                          │
  │  Engine:     Raptor-2 (×9)                                  │
  │  Propellant: 112,400 kg (LOX/CH4)                           │
  │  Dry mass:   18,600 kg                                      │
  │  Δv:         5,560 m/s                                      │
  │  Burn time:  162 s                                          │
  │  TWR:        1.4 → 4.2                                      │
  └─────────────────────────────────────────────────────────────┘

  Payload fraction:  3.5%
  Mass margin:       +240 m/s (2.6%)

═══════════════════════════════════════════════════════════════
```

Design choices:
- Box drawing characters for structure
- Stages listed top-to-bottom (upper first) — matches physical stacking
- Key metrics aligned for easy scanning
- Units always shown
- Numbers formatted with thousand separators

### Compact Output

For quick calculations where full formatting is overhead:

```bash
$ tsi calculate --engine raptor-2 --propellant-mass 100000 --dry-mass 8000
Δv: 8,247 m/s | Burn: 143 s | TWR: 3.1 (vac)
```

One line, essential numbers only.

### JSON Output

```bash
$ tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 -o json
```

```json
{
  "version": "0.1.0",
  "problem": {
    "payload_kg": 5000,
    "target_delta_v_mps": 9400,
    "constraints": {
      "min_twr": 1.2,
      "max_stages": 3,
      "structural_ratio": 0.1
    }
  },
  "solution": {
    "rocket": {
      "stages": [
        {
          "position": 1,
          "engine": "Raptor-2",
          "engine_count": 9,
          "propellant_mass_kg": 112400,
          "dry_mass_kg": 18600,
          "delta_v_mps": 5560,
          "burn_time_s": 162,
          "twr_initial": 1.4,
          "twr_final": 4.2
        },
        {
          "position": 2,
          "engine": "Raptor-2",
          "engine_count": 1,
          "propellant_mass_kg": 18200,
          "dry_mass_kg": 2100,
          "delta_v_mps": 3840,
          "burn_time_s": 142,
          "twr_initial": 0.8,
          "twr_final": 2.1
        }
      ],
      "payload_kg": 5000,
      "total_mass_kg": 142300,
      "total_delta_v_mps": 9400,
      "payload_fraction": 0.035
    },
    "margin_mps": 240,
    "iterations": 12847,
    "elapsed_ms": 342
  }
}
```

Design choices:
- Flat structure where possible
- Units in key names (`_kg`, `_mps`, `_s`)
- Metadata included (version, timing)
- Stages indexed from 1 (human-friendly)

### Table Output

For `tsi engines`:

```
┌──────────────┬────────────┬────────────┬─────────┬──────────┐
│ Engine       │ Propellant │ Thrust(vac)│ Isp(vac)│ Mass     │
├──────────────┼────────────┼────────────┼─────────┼──────────┤
│ Merlin-1D    │ LOX/RP-1   │ 914 kN     │ 311 s   │ 470 kg   │
│ Raptor-2     │ LOX/CH4    │ 2,450 kN   │ 350 s   │ 1,600 kg │
│ RS-25        │ LOX/LH2    │ 2,279 kN   │ 452 s   │ 3,527 kg │
│ RL-10C       │ LOX/LH2    │ 106 kN     │ 453 s   │ 190 kg   │
│ ...          │            │            │         │          │
└──────────────┴────────────┴────────────┴─────────┴──────────┘
```

---

## Color Usage

### When to Use Color

- **Success indicators** — Green checkmarks, positive margins
- **Warnings** — Yellow for near-constraint values
- **Errors** — Red for failures
- **Emphasis** — Bold for key numbers (not color)

### When Not to Use Color

- Piped output (detect with `isatty`)
- `--no-color` flag set
- `NO_COLOR` environment variable set

### Color Palette

```
Success:    Green   (#22c55e)
Warning:    Yellow  (#eab308)
Error:      Red     (#ef4444)
Emphasis:   Bold    (no color)
Muted:      Gray    (#6b7280)
```

### Implementation

```rust
use std::io::IsTerminal;

fn should_colorize() -> bool {
    std::io::stdout().is_terminal()
        && std::env::var("NO_COLOR").is_err()
        && !args.no_color
}
```

---

## Error Messages

### Anatomy of a Good Error

```
Error: Unknown engine 'raptor-3'

Available engines:
  merlin-1d, raptor-2, rs-25, rl-10c, ...

Run `tsi engines` for the full list.
```

Components:
1. **What happened** — Clear, specific
2. **Context** — What options exist
3. **How to fix** — Actionable suggestion

### Error Categories

**User errors** (exit code 1):
```
Error: --payload is required

Usage: tsi optimize --payload <KG> --target-dv <M/S> --engine <NAME>
```

**Infeasible problems** (exit code 2):
```
Error: No feasible solution found

The target Δv (15,000 m/s) cannot be achieved with the given constraints.

Try:
  • Increase --max-stages (currently 2)
  • Use higher-Isp engines for upper stages
  • Reduce payload mass
```

**Internal errors** (exit code 3):
```
Error: Internal error during optimization

This is a bug. Please report it at:
https://github.com/user/tsi/issues

Include this information:
  Version: 0.1.0
  Command: tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
  Error: index out of bounds: len is 0 but index is 0
```

### Validation Errors

Check all inputs before running:

```
Error: Invalid arguments

  • --min-twr must be positive (got: -1.5)
  • --payload must be positive (got: 0)
  • Unknown engine: 'raptor-3' (did you mean 'raptor-2'?)
```

All problems reported at once, not one at a time.

---

## Progress Indication

### When to Show Progress

- Operations taking > 1 second
- Operations with unknown duration
- Operations that might appear hung

### Spinner for Short Operations

```
⠋ Optimizing...
⠙ Optimizing...
⠹ Optimizing...
⠸ Optimizing...
```

### Progress Bar for Long Operations

```
Monte Carlo: [████████████░░░░░░░░░░░░░░░░░░] 42% (4,200 / 10,000)
```

### No Progress for Scripting

Progress goes to stderr, results to stdout. When piped, suppress progress:

```rust
if std::io::stderr().is_terminal() {
    show_progress();
}
```

---

## Help System

### Layered Help

**Level 1: Command list** (`tsi` or `tsi --help`)
```
tsi - Rocket staging optimizer

Commands:
  calculate  Compute delta-v for a single stage
  optimize   Find optimal staging configuration
  engines    List available rocket engines

Run `tsi <command> --help` for details.
```

**Level 2: Command help** (`tsi optimize --help`)
```
Find optimal staging configuration for given constraints

Usage: tsi optimize [OPTIONS] --payload <KG> --target-dv <M/S> --engine <NAME>

[Full options list]

Examples:
  tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
  tsi optimize --payload 5000 --target-dv 9400 --engines merlin-1d,raptor-2
```

**Level 3: Detailed docs** (external)
```
For detailed documentation, visit:
https://github.com/user/tsi#readme
```

### Examples in Help

Every command's help includes at least one working example:

```
Examples:
  # Basic two-stage optimization
  tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2

  # Multi-engine search
  tsi optimize --payload 5000 --target-dv 9400 \
    --engines merlin-1d,raptor-2,rl-10c --max-stages 3

  # With uncertainty analysis
  tsi optimize --payload 5000 --target-dv 9400 \
    --engine raptor-2 --monte-carlo 10000
```

Users copy-paste examples and modify. Make sure examples work.

---

## Interactive Elements

### Confirmation Prompts

For destructive or slow operations:

```
This will run 100,000 Monte Carlo iterations.
Estimated time: ~5 minutes.

Continue? [y/N]
```

Only in interactive mode. Skip with `--yes` flag.

### Suggestions

When results are marginal:

```
⚠ Warning: Payload fraction (0.8%) is very low.

Consider:
  • Higher-Isp upper stage (try --engines merlin-1d,rl-10c)
  • Reducing payload mass
  • Accepting lower delta-v margin
```

---

## Consistency

### Terminology

Use the same terms everywhere:

| Concept | Term | Not |
|---------|------|-----|
| Change in velocity | delta-v, Δv | dv, deltaV |
| Specific impulse | Isp | specific impulse, ISP |
| Thrust-to-weight | TWR | T/W, thrust/weight |
| Propellant | propellant | fuel (technically wrong) |
| Mass ratio | mass ratio | MR |

### Number Formatting

| Type | Format | Example |
|------|--------|---------|
| Mass | Thousands separator | 142,300 kg |
| Velocity | Thousands separator | 9,400 m/s |
| Ratio | 1-2 decimal places | 3.52 |
| Percentage | 1 decimal place | 3.5% |
| Time | Whole seconds | 162 s |

### Symbols

| Quantity | Symbol |
|----------|--------|
| Delta-v | Δv |
| Mass | m |
| Specific impulse | Isp |

Use symbols in output, words in help text.

---

## Accessibility

### Screen Readers

- ASCII art has text alternatives
- Progress doesn't rely solely on animation
- Color is never the only indicator

### Terminal Compatibility

- Works in 80-column terminals
- Box drawing degrades to ASCII if needed
- No mandatory Unicode beyond Δ (provide `--ascii` fallback)

### Internationalization

- Numbers follow locale (future)
- Units are SI (always)
- Error messages in English (initially)

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | User error (bad arguments, unknown engine) |
| 2 | No solution (infeasible problem) |
| 3 | Internal error (bug) |

Scripts can check exit codes:

```bash
if tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 > result.json; then
    echo "Success"
else
    case $? in
        1) echo "Bad arguments" ;;
        2) echo "No solution possible" ;;
        3) echo "Bug in tsi" ;;
    esac
fi
```

---

## Future: TUI Mode

Potential interactive terminal UI with ratatui:

```
┌─ tsi ─────────────────────────────────────────────────────────┐
│                                                               │
│  Payload: [5000] kg        Target Δv: [9400] m/s              │
│                                                               │
│  Engines: [■] Merlin-1D  [■] Raptor-2  [ ] RS-25  [ ] RL-10C  │
│                                                               │
│  Max stages: [2]  Min TWR: [1.2]                              │
│                                                               │
│  ─────────────────────────────────────────────────────────────│
│                                                               │
│  Stage 2:  Raptor-2 ×1   │████████░░░░░░│  3,840 m/s          │
│  Stage 1:  Raptor-2 ×9   │██████████████│  5,560 m/s          │
│                                                               │
│  Total: 9,400 m/s   Margin: +240 m/s   Mass: 142,300 kg       │
│                                                               │
│  [Optimize]  [Export JSON]  [Reset]                           │
└───────────────────────────────────────────────────────────────┘
```

Not for v1.0, but the architecture should allow it.

---

## Summary

Good CLI UX is invisible. Users don't notice it — they just get their work done. The interface should be:

- **Predictable** — Works like other Unix tools
- **Informative** — Shows what you need, hides what you don't
- **Forgiving** — Helpful errors, sensible defaults
- **Scriptable** — JSON output, meaningful exit codes
- **Fast** — No unnecessary prompts or pauses

The goal: users think about rockets, not about the tool.
