# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`tsi` (Tsiolkovsky) is a Rust CLI tool for rocket staging optimization. Given payload mass, target delta-v, and available engines, it finds optimal staging configurations that maximize payload fraction or minimize total mass.

**Current Status:** Phase 2 complete (v0.2.0). Working CLI with engine database, stage calculations, and comprehensive test suite (144 tests).

## Build Commands

Once the Cargo project is initialized:

```bash
cargo build              # Compile
cargo test               # Run all tests
cargo test <name>        # Run specific test
cargo test --lib         # Run only unit tests
cargo clippy             # Lint
cargo fmt                # Format code
cargo run -- <cmd>       # Run CLI (e.g., cargo run -- optimize --help)
cargo bench              # Run benchmarks
```

## Architecture

The tool is designed as a Rust library + CLI application:

### Module Structure
- **units/** - Type-safe newtypes for physical quantities (Mass, Velocity, Force, Time, Isp, Ratio). Prevents unit errors at compile time.
- **engine/** - Engine struct, Propellant enum, TOML database loading (~10 real engines: Merlin-1D, Raptor-2, RS-25, RL-10C, etc.)
- **stage/** - Stage (single stage) and Rocket (multi-stage assembly) types
- **physics/** - Tsiolkovsky equation (`Δv = Isp × g₀ × ln(mass_ratio)`), TWR, burn time calculations
- **optimizer/** - Optimizer trait with implementations: AnalyticalOptimizer (closed-form 2-stage), BruteForceOptimizer (grid search), MonteCarloRunner (uncertainty via rayon parallelism)
- **cli/** - clap-based argument parsing with three subcommands: `calculate`, `optimize`, `engines`
- **output/** - Terminal (box-drawing), JSON, and ASCII diagram formatters

### Key Design Decisions
- Newtype pattern for all physical units (compiler prevents adding kg to m/s)
- Engine data embedded via `include_str!` for single-binary distribution
- Property-based testing with proptest for physics invariants
- Validation tests against real rockets (Saturn V, Falcon 9, Space Shuttle)

### Test Suite (144 tests)
- **85 unit tests** - Inline in source modules
- **23 integration tests** - CLI end-to-end tests (`tests/cli.rs`)
- **10 property tests** - Invariants via proptest (`tests/properties.rs`)
- **10 validation tests** - Real rocket comparisons (`tests/validation.rs`)
- **16 doc tests** - Examples in rustdoc comments

## Development Roadmap

See `docs/roadmap.md` for detailed phases. Summary:
1. **Foundation** - Unit types, physics core, `calculate` command
2. **Engine Database** - TOML loading, `engines` command, Stage type
3. **Two-Stage Optimization** - Rocket type, analytical optimizer, `optimize` command
4. **Multi-Engine Search** - Brute force optimizer, multiple engine types
5. **Uncertainty Analysis** - Monte Carlo simulation
6. **Polish** - ASCII diagrams, shell completions
7. **Release** - crates.io publication

## Key Files

- `docs/architecture.md` - Technical design with module structure and type definitions
- `docs/testing.md` - Test categories, example tests, CI configuration
- `docs/interface.md` - CLI UX design, output formats, error handling patterns
- `docs/concept.md` - Project vision, target users, design principles

## Physics Reference

Core equation: `Δv = Isp × g₀ × ln(m_wet / m_dry)` where g₀ = 9.80665 m/s²

Validation targets:
- Falcon 9 S1: ~8500 m/s ideal delta-v
- Falcon 9 S2: ~11000 m/s ideal delta-v
- Saturn V S-IC: ~7500 m/s ideal delta-v

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
