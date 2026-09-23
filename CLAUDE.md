# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`tsi` (Tsiolkovsky) is a Rust CLI tool for rocket staging optimization. Given payload mass, target delta-v, and available engines, it finds optimal staging configurations that maximize payload fraction or minimize total mass.

**Current Status:** v0.7.0 (Static fire). Working CLI with engine database, N-stage multi-engine optimization cross-checked by two independent optimizers, Monte Carlo analysis of fixed designs, and validation against real vehicles. MSRV is Rust 1.87; CI runs on Linux, macOS and Windows.

## Build Commands

```bash
cargo build              # Compile
cargo test               # Run all tests
cargo test <name>        # Run specific test
cargo test --lib         # Run only unit tests
cargo clippy --all-targets -- -D warnings   # Lint (CI denies warnings)
cargo fmt                # Format code
cargo run -- <cmd>       # Run CLI (e.g., cargo run -- optimize --help)
cairn check              # Validate the roadmap (CI runs this too)
```

## Architecture

The tool is a Rust library + CLI application:

### Module Structure
- **units/** - Type-safe newtypes for physical quantities (Mass, Velocity, Force, Time, Isp, Ratio). Prevents unit errors at compile time.
- **engine/** - Engine struct, Propellant enum, TOML database loading (11 real engines: Merlin-1D, Raptor-2, RS-25, RL-10C, etc.)
- **stage/** - Stage (single stage) and Rocket (multi-stage assembly) types. `Rocket` evaluates its first stage with an `IspModel` (ascent-averaged by default).
- **physics/** - Tsiolkovsky equation, TWR, burn time, `IspModel` (why Isp depends on altitude), empirical loss estimates
- **optimizer/** - `Problem`/`Constraints`, the `Optimizer` trait, `AnalyticalOptimizer` (Lagrange staging solution refined numerically; any stage count, any engine mix), `BruteForceOptimizer` (streaming grid search, used as an independent cross-check), `MonteCarloRunner` (stresses a fixed design; seeded). `sizing.rs` holds the shared top-down stage sizing and the closed-form minimum engine count.
- **cli/** - clap-based argument parsing: `calculate`, `optimize`, `engines`, `completions`
- **output/** - Terminal (box-drawing) and ASCII diagram formatters

### Key Design Decisions
- Newtype pattern for all physical units (compiler prevents adding kg to m/s)
- Engine data embedded via `include_str!` for single-binary distribution
- No hidden margins: rockets are sized to hit the target exactly; margin is an explicit constraint
- Comparisons that validate input are written so NaN fails them (`!(x > 0.0)` style, or `is_nan() ||`)
- Property-based testing with proptest for physics and optimizer invariants
- Validation tests against real rockets, honest about where ideal theory stops (see the Saturn V test)

### Test Suite (308 tests)
- **173 unit tests** - Inline in source modules
- **69 integration tests** - CLI end-to-end tests (`tests/cli.rs`)
- **16 property tests** - Invariants via proptest (`tests/properties.rs`), including analytical vs brute force
- **18 validation tests** - Real rocket comparisons (`tests/validation.rs`)
- **32 doc tests** - Examples in rustdoc comments (none ignored)

## Development Roadmap

The roadmap and issues live in cairn (see the section at the end of this file).
`ROADMAP.md` is generated from `cairn/items/`; never edit it by hand. Milestones:

- **v0.7 Static fire** - optimizer and Monte Carlo correctness, clippy clean, CI
- **v0.8 Stacking** - library-first API: feature-gated CLI, typed errors, no panics, serde
- **v0.9 Wet dress** - character: `--explain`, mission targets, vehicle library, cited engine data
- **v1.0 Liftoff** - API freeze, packaging, crates.io as `tsiolkovsky`

Items flagged `breaking=true` must land before the 1.0 freeze (`cairn list --view breaking`).
The v0.1-v0.6 phase plan is archived at `docs/plan/roadmap-v0.md`.

## Key Files

- `docs/plan/architecture.md` - Technical design with module structure and type definitions
- `docs/plan/testing.md` - Test categories, example tests, CI configuration
- `docs/plan/interface.md` - CLI UX design, output formats, error handling patterns
- `docs/plan/concept.md` - Project vision, target users, design principles
- `docs/commands.md`, `docs/physics.md` - User-facing command reference and physics guide
- `CHANGELOG.md` - Keep a Changelog format; update it with every user-visible change

## Physics Reference

Core equation: `Δv = Isp × g₀ × ln(m_wet / m_dry)` where g₀ = 9.80665 m/s²

Validation targets:
- Falcon 9 S1: ~8,700 m/s isolated ideal delta-v (ascent-averaged Isp ~302 s)
- Falcon 9 stacked (S1 + S2 with 22.8 t payload): ~9,300 m/s
- Optimizer redesign of Falcon 9: within 5% of the real 571.5 t
- Saturn V S-IC: ~7,500-8,500 m/s isolated ideal delta-v

## User Preferences

Based on prior conversations, the following preferences have been expressed:

### Testing Philosophy
- **Thorough testing matters**: Not just unit tests, but property-based tests (proptest) and validation against real-world rocket data
- **Validate against reality**: Tests should compare calculations to known values from Saturn V, Falcon 9, Space Shuttle, Starship
- **Test invariants, not just examples**: Property-based tests catch edge cases that example-based tests miss

### Code Quality
- **Keep clippy clean**: Run `cargo clippy` and fix all warnings before committing
- **Consistent formatting**: Always run `cargo fmt`
- **Doc tests must pass**: All code examples in documentation should compile and run

### Documentation Style
- **Educational, not just descriptive**: Comments should explain the "why" and physical intuition, not just the "what"
- **Professional quality**: Documentation should be thorough enough for someone learning rocket science
- **Include reference tables**: Typical values, propellant comparisons, orbital delta-v requirements help users understand context
- **Real-world examples**: Use actual rocket data (Merlin-1D, Raptor-2, RS-25) in examples

### Code Style
- **Type safety over convenience**: The newtype pattern for units prevents bugs at compile time
- **Embed data for distribution**: Use `include_str!` so the binary is self-contained
- **Avoid over-engineering**: Keep implementations simple and focused on the current task

<!-- cairn:begin -->
## Roadmap and issues

This project tracks its roadmap and issues with `cairn`. Every item is a Markdown file under `cairn/items`, described by the schema in `cairn.toml`.

**Do not create ad-hoc TODO, PLAN or NOTES files.** Create a cairn item instead, so the work appears on the board and in the generated roadmap.

### The loop

1. `cairn next` — what is ready to start. It excludes anything blocked by unfinished dependencies and puts work already in progress first.
2. `cairn claim <ID>` — take it before you start, so no one duplicates the work. `cairn claim --next` picks and claims the top-ranked unclaimed item in one step, and prints its body so you can begin immediately.
3. Do the work. Record what you learn: `cairn set <ID> <field>=<value>` for fields, `cairn note <ID> "<TEXT>"` for anything that needs a sentence — why you chose something, what you tried, what to watch for.
4. `cairn tick <ID> <N>` as each acceptance criterion becomes true — `cairn show <ID> --criteria` lists them numbered. Tick what is true, not what would let you close.
5. `cairn close <ID>` when it is done, or `cairn release <ID>` to hand it back.
6. `cairn check` before you report finished. It must pass.

### Commands

```sh
cairn next --json                 # ready work, ranked
cairn claim --next                # take the next ready item
cairn search <TEXT> --json        # titles, bodies and labels
cairn list --json                 # all open items
cairn list --filter 'blocked=false,priority=p0'
cairn show <ID> --json            # one item, including its body
cairn new "<TITLE>" --type <TYPE> --milestone <MILESTONE>
cairn set <ID> status=<STATUS>    # also labels+=x, or any field below
cairn note <ID> "<TEXT>"          # append reasoning; never replaces
cairn show <ID> --criteria        # acceptance criteria, numbered
cairn tick <ID> <N>               # tick one; --all for every one
cairn close <ID>
cairn check                       # validate; run before finishing
cairn render                      # regenerate ROADMAP.md
```

### Schema

- **Types**: `feature`, `bug`, `validation`, `docs`, `chore`, `milestone`
- **Statuses**: `backlog` (open), `planned` (open), `doing` (active), `blocked` (active), `done` (done), `dropped` (dropped)
- **`due`**: date, YYYY-MM-DD — when a milestone is meant to land
- **`part_of`**: names any items, by id, several allowed — a larger piece of work this belongs to
- **`priority`**: one of p0, p1, p2, p3 — p0 blocks its milestone; p3 is nice to have
- **`effort`**: one of s, m, l, xl — s: under an hour, m: a session, l: two or three, xl: split it
- **`area`**: one of units, physics, engine, stage, optimizer, output, cli, data, docs, infra — the module or concern this touches
- **`breaking`**: true or false — changes the public library API or CLI contract
- **Milestones**: `v0.7` (due 2026-10-06), `v0.8` (due 2026-10-27), `v0.9` (due 2026-11-17), `v1.0` (due 2026-12-01), `later`
- **Saved views** (`cairn list --view NAME`): `now`, `next`, `triage`, `breaking`, `physics`

### Rules

1. Before starting work, find or create the item and set it to an active status.
2. Use the fields above rather than inventing new ones; add new fields to `cairn.toml` first.
3. Never hand-edit the generated roadmap file — change items and run `cairn render`.
4. `cairn check` must pass before the work is considered done.

<!-- cairn:end -->
