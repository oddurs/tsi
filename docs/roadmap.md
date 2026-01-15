# tsi Development Roadmap

## Phase 1: Foundation
**Goal:** Working single-stage calculator with type-safe units

### 1.1 Project Setup
- [ ] Initialize cargo project
- [ ] Set up directory structure per architecture.md
- [ ] Configure Cargo.toml with metadata and dependencies
- [ ] Add MIT license
- [ ] Create README.md with project description

### 1.2 Unit Types
- [ ] Implement `Mass` newtype with kg/tonnes constructors
- [ ] Implement `Velocity` newtype with m/s and km/s
- [ ] Implement `Force` newtype for thrust (Newtons)
- [ ] Implement `Time` newtype for durations
- [ ] Implement `Isp` newtype for specific impulse
- [ ] Implement `Ratio` for dimensionless values
- [ ] Add arithmetic ops: Mass + Mass, Mass / Mass → Ratio, etc.
- [ ] Add Display traits for pretty printing
- [ ] Write unit tests for arithmetic correctness

### 1.3 Physics Core
- [ ] Implement Tsiolkovsky equation: `delta_v(isp, mass_ratio)`
- [ ] Implement inverse: `required_mass_ratio(delta_v, isp)`
- [ ] Implement TWR calculation
- [ ] Implement burn time calculation
- [ ] Validate against known values (Saturn V, Falcon 9)

### 1.4 Basic CLI
- [ ] Set up clap with derive macros
- [ ] Implement `tsi calculate` subcommand
- [ ] Accept: --isp, --mass-ratio (or --wet-mass, --dry-mass)
- [ ] Output: delta-v, burn time (if thrust provided)
- [ ] Add --help with clear descriptions

### 1.5 First Release Checkpoint
- [ ] All tests passing
- [ ] `cargo clippy` clean
- [ ] `cargo fmt` applied
- [ ] Manual testing of calculate command
- [ ] Tag v0.1.0

**Deliverable:** `tsi calculate --isp 311 --wet-mass 550000 --dry-mass 26000` outputs delta-v

---

## Phase 2: Engine Database
**Goal:** Load real engine data, compute stage parameters

### 2.1 Engine Types
- [ ] Define `Propellant` enum (LoxRp1, LoxLh2, LoxCh4, etc.)
- [ ] Define `Engine` struct with all parameters
- [ ] Implement `isp_at(pressure_ratio)` interpolation
- [ ] Implement `thrust_at(pressure_ratio)` interpolation

### 2.2 Data Loading
- [ ] Create `data/engines.toml` with 8-10 common engines
- [ ] Implement TOML deserialization with serde
- [ ] Load from embedded data (include_str!) for binary distribution
- [ ] Allow override via `--engines-file` flag
- [ ] Handle missing/malformed data gracefully

### 2.3 Engine Listing
- [ ] Implement `tsi engines` subcommand
- [ ] List all available engines with key stats
- [ ] Add `--verbose` for full parameter dump
- [ ] Add `--json` for machine-readable output

### 2.4 Stage Type
- [ ] Define `Stage` struct
- [ ] Implement derived calculations (dry_mass, wet_mass, delta_v)
- [ ] Implement TWR at ignition given payload mass
- [ ] Implement burn time from stage parameters

### 2.5 Enhanced Calculate
- [ ] Accept `--engine` flag to look up from database
- [ ] Accept `--engine-count` for multiple engines
- [ ] Accept `--propellant-mass` instead of mass ratio
- [ ] Compute structural mass from configurable ratio
- [ ] Output full stage summary

**Deliverable:** `tsi engines` lists Merlin-1D, Raptor-2, etc. `tsi calculate --engine raptor-2 --propellant-mass 100000` outputs stage performance.

---

## Phase 3: Two-Stage Optimization
**Goal:** Analytical optimizer for simple two-stage rockets

### 3.1 Rocket Type
- [ ] Define `Rocket` struct (stages + payload)
- [ ] Implement total_delta_v aggregation
- [ ] Implement total_mass calculation
- [ ] Implement payload_fraction

### 3.2 Constraints
- [ ] Define `Constraints` struct
- [ ] Minimum TWR at each stage ignition
- [ ] Maximum number of stages
- [ ] Structural mass ratio

### 3.3 Analytical Optimizer
- [ ] Implement Lagrange multiplier solution for 2 stages
- [ ] Handle same-engine case (closed form)
- [ ] Validate against textbook optimal staging ratios
- [ ] Return `Solution` with rocket config and margin

### 3.4 Optimize Command
- [ ] Implement `tsi optimize` subcommand
- [ ] Accept: --payload, --target-dv, --engine, --min-twr
- [ ] Run analytical optimizer
- [ ] Output full solution breakdown

### 3.5 Pretty Output
- [ ] Design terminal output format with box drawing
- [ ] Show stage-by-stage breakdown
- [ ] Show payload fraction and margin
- [ ] Color output for key metrics (optional, detect tty)

**Deliverable:** `tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2` outputs optimal 2-stage configuration.

---

## Phase 4: Multi-Engine Optimization
**Goal:** Handle multiple engine types and discrete choices

### 4.1 Problem Definition
- [ ] Define `Problem` struct with full constraints
- [ ] Support multiple available engines
- [ ] Support variable stage count (1-N)
- [ ] Support per-stage engine count limits

### 4.2 Brute Force Optimizer
- [ ] Implement grid search over discrete space
- [ ] Iterate: stage count × engine choice × engine count × propellant mass
- [ ] Prune infeasible configurations early (TWR check)
- [ ] Track best solution seen
- [ ] Add progress indicator for long searches

### 4.3 Optimizer Selection
- [ ] Define `Optimizer` trait
- [ ] Implement for `AnalyticalOptimizer`
- [ ] Implement for `BruteForceOptimizer`
- [ ] Auto-select based on problem complexity
- [ ] Allow manual selection via `--optimizer` flag

### 4.4 Enhanced Optimize Command
- [ ] Accept comma-separated engine list
- [ ] Accept `--max-stages` constraint
- [ ] Accept `--max-engines` per stage constraint
- [ ] Show search progress for brute force
- [ ] Report number of configurations evaluated

### 4.5 JSON Output
- [ ] Implement `--output json` flag
- [ ] Serialize full Solution to JSON
- [ ] Include all stage parameters
- [ ] Include metadata (runtime, iterations)

**Deliverable:** `tsi optimize --payload 5000 --target-dv 9400 --engines merlin-1d,raptor-2,rl-10c --max-stages 3` finds optimal mixed-engine configuration.

---

## Phase 5: Uncertainty Analysis
**Goal:** Monte Carlo simulation for robust solutions

### 5.1 Random Sampling
- [ ] Add parameter uncertainty to Problem definition
- [ ] Isp uncertainty (±X%)
- [ ] Structural mass uncertainty (±X%)
- [ ] Thrust uncertainty (±X%)
- [ ] Sample from normal distributions

### 5.2 Monte Carlo Runner
- [ ] Implement parallel execution with rayon
- [ ] Run N iterations with perturbed parameters
- [ ] Collect delta-v and mass distributions
- [ ] Compute percentiles (5th, 50th, 95th)

### 5.3 Results Reporting
- [ ] Calculate success probability (delta-v ≥ target)
- [ ] Report confidence intervals
- [ ] Show histogram in terminal (ASCII)
- [ ] Include in JSON output

### 5.4 CLI Integration
- [ ] Add `--monte-carlo N` flag to optimize command
- [ ] Add `--uncertainty` flag for parameter spread
- [ ] Show Monte Carlo summary after nominal solution
- [ ] Warn if success probability < 95%

**Deliverable:** `tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2 --monte-carlo 10000` reports success probability and confidence intervals.

---

## Phase 6: Polish
**Goal:** Production-ready CLI experience

### 6.1 ASCII Rocket Diagram
- [ ] Generate simple ASCII art of rocket configuration
- [ ] Scale stage heights by propellant mass
- [ ] Label stages with engine names
- [ ] Add `--diagram` flag to optimize output

### 6.2 Atmospheric Losses
- [ ] Implement gravity drag estimation
- [ ] Implement atmospheric drag estimation
- [ ] Add to effective delta-v requirements
- [ ] Document assumptions and limitations

### 6.3 Custom Engines
- [ ] Accept inline engine definition via CLI
- [ ] Format: `--custom-engine "name:thrust:isp:mass:propellant"`
- [ ] Validate parameters before optimization
- [ ] Allow mixing custom and database engines

### 6.4 Shell Integration
- [ ] Generate shell completions (bash, zsh, fish)
- [ ] Generate man page
- [ ] Add `tsi completions` subcommand
- [ ] Document installation in README

### 6.5 Error Handling
- [ ] Review all error paths
- [ ] Add helpful error messages
- [ ] Suggest fixes for common mistakes
- [ ] Exit codes follow conventions

**Deliverable:** Polished CLI with completions, man page, and helpful errors.

---

## Phase 7: Release
**Goal:** Public release on crates.io

### 7.1 Documentation
- [ ] Comprehensive README with examples
- [ ] API documentation (rustdoc)
- [ ] CHANGELOG.md
- [ ] CONTRIBUTING.md

### 7.2 Testing
- [ ] Unit test coverage > 80%
- [ ] Integration tests for all commands
- [ ] Property-based tests for physics
- [ ] CI pipeline (GitHub Actions)

### 7.3 Packaging
- [ ] Verify crates.io metadata
- [ ] Test `cargo install` from local path
- [ ] Create GitHub release with binaries
- [ ] Create Homebrew formula
- [ ] Test cross-platform (Linux, macOS, Windows)

### 7.4 Launch
- [ ] Publish to crates.io
- [ ] Submit Homebrew formula
- [ ] Write announcement post
- [ ] Share on relevant communities

**Deliverable:** `cargo install tsi` works. Homebrew formula submitted.

---

## Future Phases (Post-1.0)

### Phase 8: Interactive TUI
- [ ] Ratatui-based interactive mode
- [ ] Real-time parameter adjustment
- [ ] Visual staging diagram
- [ ] Live delta-v budget display

### Phase 9: Trajectory Simulation
- [ ] Numerical integration of ascent
- [ ] Altitude/velocity profiles
- [ ] Actual atmospheric model
- [ ] Comparison with ideal delta-v

### Phase 10: Cost Optimization
- [ ] Engine cost database
- [ ] $/kg to orbit optimization
- [ ] Reusability cost models
- [ ] Trade study generation

---

## Time Estimates

| Phase | Effort | Cumulative |
|-------|--------|------------|
| 1. Foundation | 2-3 sessions | 2-3 sessions |
| 2. Engine Database | 2 sessions | 4-5 sessions |
| 3. Two-Stage Optimization | 2-3 sessions | 6-8 sessions |
| 4. Multi-Engine | 3-4 sessions | 9-12 sessions |
| 5. Uncertainty | 2-3 sessions | 11-15 sessions |
| 6. Polish | 2-3 sessions | 13-18 sessions |
| 7. Release | 1-2 sessions | 14-20 sessions |

A "session" is roughly 2-4 hours of focused work.

---

## Definition of Done

Each phase is complete when:
1. All checkboxes marked done
2. Tests passing (`cargo test`)
3. No clippy warnings (`cargo clippy`)
4. Code formatted (`cargo fmt`)
5. README updated if user-facing changes
6. Git tagged with version number

---

## Notes

- Prioritize correctness over features — physics must be right
- Keep the CLI simple; complexity lives in the library
- Write tests as you go, not after
- Each phase should produce something usable
- Don't over-engineer early; refactor as patterns emerge
