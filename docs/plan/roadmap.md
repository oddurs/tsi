# tsi Development Roadmap

## Phase 1: Foundation ✅ COMPLETE (v0.1.0)
**Goal:** Working single-stage calculator with type-safe units

### 1.1 Project Setup
- [x] Initialize cargo project
- [x] Set up directory structure per architecture.md
- [x] Configure Cargo.toml with metadata and dependencies
- [x] Add MIT license
- [x] Create README.md with project description

### 1.2 Unit Types
- [x] Implement `Mass` newtype with kg/tonnes constructors
- [x] Implement `Velocity` newtype with m/s and km/s
- [x] Implement `Force` newtype for thrust (Newtons)
- [x] Implement `Time` newtype for durations
- [x] Implement `Isp` newtype for specific impulse
- [x] Implement `Ratio` for dimensionless values
- [x] Add arithmetic ops: Mass + Mass, Mass / Mass → Ratio, etc.
- [x] Add Display traits for pretty printing
- [x] Write unit tests for arithmetic correctness

### 1.3 Physics Core
- [x] Implement Tsiolkovsky equation: `delta_v(isp, mass_ratio)`
- [x] Implement inverse: `required_mass_ratio(delta_v, isp)`
- [x] Implement TWR calculation
- [x] Implement burn time calculation
- [x] Validate against known values (Saturn V, Falcon 9)

### 1.4 Basic CLI
- [x] Set up clap with derive macros
- [x] Implement `tsi calculate` subcommand
- [x] Accept: --isp, --mass-ratio (or --wet-mass, --dry-mass)
- [x] Output: delta-v, burn time (if thrust provided)
- [x] Add --help with clear descriptions

### 1.5 First Release Checkpoint
- [x] All tests passing
- [x] `cargo clippy` clean
- [x] `cargo fmt` applied
- [x] Manual testing of calculate command
- [x] Tag v0.1.0

**Deliverable:** `tsi calculate --isp 311 --wet-mass 550000 --dry-mass 26000` outputs delta-v

---

## Phase 2: Engine Database ✅ COMPLETE (v0.2.0)
**Goal:** Load real engine data, compute stage parameters

### 2.1 Engine Types
- [x] Define `Propellant` enum (LoxRp1, LoxLh2, LoxCh4, etc.)
- [x] Define `Engine` struct with all parameters
- [x] Implement `isp_at(pressure_ratio)` interpolation
- [x] Implement `thrust_at(pressure_ratio)` interpolation

### 2.2 Data Loading
- [x] Create `data/engines.toml` with 11 common engines
- [x] Implement TOML deserialization with serde
- [x] Load from embedded data (include_str!) for binary distribution
- [ ] Allow override via `--engines-file` flag (deferred)
- [x] Handle missing/malformed data gracefully

### 2.3 Engine Listing
- [x] Implement `tsi engines` subcommand
- [x] List all available engines with key stats
- [x] Add `--verbose` for full parameter dump
- [x] Add `--output json` for machine-readable output
- [x] Add `--propellant` filter
- [x] Add `--name` filter

### 2.4 Stage Type
- [x] Define `Stage` struct
- [x] Implement derived calculations (dry_mass, wet_mass, delta_v)
- [x] Implement TWR at ignition given payload mass
- [x] Implement burn time from stage parameters

### 2.5 Enhanced Calculate
- [x] Accept `--engine` flag to look up from database
- [x] Accept `--engine-count` for multiple engines
- [x] Accept `--propellant-mass` instead of mass ratio
- [x] Compute structural mass from configurable ratio
- [x] Output full stage summary
- [x] Add `--output compact` for one-line output

### 2.6 v0.2.0 Enhancements (bonus)
- [x] Thousands separators in number output
- [x] Integration tests (23 tests)
- [x] Engine name suggestions on typos
- [x] Multi-error validation messages
- [x] Help text examples

**Deliverable:** `tsi engines` lists Merlin-1D, Raptor-2, etc. `tsi calculate --engine raptor-2 --propellant-mass 100000` outputs stage performance.

---

## Phase 3: Two-Stage Optimization ✅ COMPLETE (v0.3.0)
**Goal:** Analytical optimizer for simple two-stage rockets

### 3.1 Rocket Type
- [x] Define `Rocket` struct (stages + payload)
- [x] Implement total_delta_v aggregation
- [x] Implement total_mass calculation
- [x] Implement payload_fraction
- [x] Implement liftoff_twr and validate_twr

### 3.2 Constraints
- [x] Define `Constraints` struct
- [x] Minimum TWR at each stage ignition
- [x] Maximum number of stages
- [x] Structural mass ratio

### 3.3 Analytical Optimizer
- [x] Implement Lagrange multiplier solution for 2 stages
- [x] Handle same-engine case (closed form)
- [x] Validate against textbook optimal staging ratios
- [x] Return `Solution` with rocket config and margin
- [x] 2% margin for robustness

### 3.4 Optimize Command
- [x] Implement `tsi optimize` subcommand
- [x] Accept: --payload, --target-dv, --engine, --min-twr
- [x] Run analytical optimizer
- [x] Output full solution breakdown
- [x] Add `--output json` for machine-readable output

### 3.5 Pretty Output
- [x] Design terminal output format with box drawing
- [x] Show stage-by-stage breakdown
- [x] Show payload fraction and margin
- [ ] Color output for key metrics (optional, detect tty) - deferred

**Deliverable:** `tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2` outputs optimal 2-stage configuration.

---

## Phase 4: Multi-Engine Optimization ✅ COMPLETE (v0.4.0)
**Goal:** Handle multiple engine types and discrete choices

### 4.1 Problem Definition
- [x] Define `Problem` struct with full constraints
- [x] Support multiple available engines
- [x] Support variable stage count (1-N)
- [x] Support per-stage engine count limits

### 4.2 Brute Force Optimizer
- [x] Implement grid search over discrete space
- [x] Iterate: stage count × engine choice × engine count × propellant mass
- [x] Prune infeasible configurations early (TWR check)
- [x] Track best solution seen
- [ ] Add progress indicator for long searches (deferred to v0.5)

### 4.3 Optimizer Selection
- [x] Define `Optimizer` trait
- [x] Implement for `AnalyticalOptimizer`
- [x] Implement for `BruteForceOptimizer`
- [x] Auto-select based on problem complexity
- [x] Allow manual selection via `--optimizer` flag

### 4.4 Enhanced Optimize Command
- [x] Accept comma-separated engine list
- [x] Accept `--max-stages` constraint
- [x] Accept `--max-engines` per stage constraint
- [ ] Show search progress for brute force (deferred to v0.5)
- [x] Report number of configurations evaluated

### 4.5 JSON Output
- [x] Implement `--output json` flag
- [x] Serialize full Solution to JSON
- [x] Include all stage parameters
- [x] Include metadata (runtime, iterations)

**Deliverable:** `tsi optimize --payload 5000 --target-dv 9400 --engine merlin-1d,raptor-2 --max-stages 3` finds optimal mixed-engine configuration.

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
- [x] Comprehensive README with examples
- [x] API documentation (rustdoc) with educational comments
- [x] CHANGELOG.md
- [ ] CONTRIBUTING.md

### 7.2 Testing
- [x] Unit test coverage > 80% (85 unit tests)
- [x] Integration tests for all commands (23 CLI tests)
- [x] Property-based tests for physics (10 proptest tests)
- [x] Validation tests against real rockets (10 tests)
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
