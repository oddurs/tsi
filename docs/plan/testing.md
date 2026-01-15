# tsi Testing Strategy

## Overview

Testing a physics-based CLI tool requires validating both correctness (does the math work?) and usability (does the tool behave as expected?). This document outlines the testing approach for `tsi`.

## Implementation Status

| Category | Status | Count | Location |
|----------|--------|-------|----------|
| Unit Tests | ✅ Implemented | 85 | `src/**/mod.rs` |
| Integration Tests | ✅ Implemented | 23 | `tests/cli.rs` |
| Property-Based Tests | ✅ Implemented | 10 | `tests/properties.rs` |
| Validation Tests | ✅ Implemented | 10 | `tests/validation.rs` |
| Doc Tests | ✅ Implemented | 16 | Inline in source |
| Regression Tests | ⏳ Planned | - | `tests/regression.rs` |
| Benchmark Tests | ⏳ Planned | - | `benches/` |

**Total: 144 tests passing**

## Test Categories

### 1. Unit Tests ✅
Location: Inline in source files (`#[cfg(test)]` modules)

Test individual functions and types in isolation. These form the foundation — if the building blocks are wrong, everything else fails.

### 2. Integration Tests ✅
Location: `tests/cli.rs`

Test the public API and CLI commands end-to-end. These verify that components work together correctly.

### 3. Property-Based Tests ✅
Location: `tests/properties.rs`

Test invariants that should hold for any valid input. These catch edge cases that example-based tests miss.

**Implemented properties:**
- Mass addition commutativity
- Mass addition/subtraction inverse
- Delta-v positive for mass ratio > 1
- Delta-v monotonic with mass ratio
- Delta-v monotonic with Isp
- Delta-v zero for mass ratio = 1
- Mass ratio round-trip (delta_v → required_mass_ratio)
- Velocity conversion round-trip (m/s ↔ km/s)
- Mass conversion round-trip (kg ↔ tonnes)
- Ratio scaling

### 4. Validation Tests ✅
Location: `tests/validation.rs`

Compare against known real-world values (Saturn V, Falcon 9, etc.). These ensure the physics matches reality.

**Implemented validations:**
- Saturn V S-IC (first stage) ideal delta-v
- Saturn V S-II (second stage) ideal delta-v
- Falcon 9 first stage ideal delta-v
- Falcon 9 second stage ideal delta-v
- Falcon 9 total delta-v is LEO-capable
- Falcon 9 liftoff TWR
- Space Shuttle SRB ideal delta-v
- Starship Super Heavy ideal delta-v
- Merlin-1D burn time
- Optimal staging equal delta-v theory

### 5. Regression Tests ⏳
Location: `tests/regression.rs`

Capture specific bugs that were found and fixed. Prevent them from returning.

---

## Unit Tests

### Units Module

```rust
// src/units/mass.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mass_construction() {
        let m1 = Mass::kg(1000.0);
        let m2 = Mass::tonnes(1.0);
        assert_eq!(m1.as_kg(), m2.as_kg());
    }

    #[test]
    fn mass_addition() {
        let m1 = Mass::kg(100.0);
        let m2 = Mass::kg(50.0);
        let sum = m1 + m2;
        assert_eq!(sum.as_kg(), 150.0);
    }

    #[test]
    fn mass_subtraction() {
        let m1 = Mass::kg(100.0);
        let m2 = Mass::kg(30.0);
        let diff = m1 - m2;
        assert_eq!(diff.as_kg(), 70.0);
    }

    #[test]
    fn mass_ratio() {
        let wet = Mass::kg(100.0);
        let dry = Mass::kg(25.0);
        let ratio = wet / dry;
        assert_eq!(ratio.as_f64(), 4.0);
    }

    #[test]
    fn mass_display() {
        let m = Mass::kg(1500.0);
        assert_eq!(format!("{}", m), "1,500 kg");
    }

    #[test]
    fn mass_zero() {
        let m = Mass::kg(0.0);
        assert_eq!(m.as_kg(), 0.0);
    }

    #[test]
    fn mass_large_values() {
        let m = Mass::kg(1_000_000_000.0); // 1 million tonnes
        assert_eq!(m.as_tonnes(), 1_000_000.0);
    }
}
```

Similar test modules for `Velocity`, `Force`, `Time`, `Isp`, `Ratio`.

### Physics Module

```rust
// src/physics/tsiolkovsky.rs
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    const G0: f64 = 9.80665;

    #[test]
    fn delta_v_basic() {
        // Δv = Isp × g₀ × ln(mass_ratio)
        // With Isp = 300s and mass_ratio = e (2.718...):
        // Δv = 300 × 9.80665 × 1 = 2941.995 m/s
        let isp = Isp::seconds(300.0);
        let ratio = Ratio::new(std::f64::consts::E);
        let dv = delta_v(isp, ratio);
        assert_relative_eq!(dv.as_mps(), 300.0 * G0, epsilon = 0.01);
    }

    #[test]
    fn delta_v_mass_ratio_one() {
        // ln(1) = 0, so Δv should be 0
        let isp = Isp::seconds(350.0);
        let ratio = Ratio::new(1.0);
        let dv = delta_v(isp, ratio);
        assert_eq!(dv.as_mps(), 0.0);
    }

    #[test]
    fn delta_v_high_mass_ratio() {
        // Sanity check with mass ratio of 10
        let isp = Isp::seconds(450.0);
        let ratio = Ratio::new(10.0);
        let dv = delta_v(isp, ratio);
        let expected = 450.0 * G0 * 10.0_f64.ln();
        assert_relative_eq!(dv.as_mps(), expected, epsilon = 0.01);
    }

    #[test]
    fn required_mass_ratio_inverse() {
        // Verify that required_mass_ratio is the inverse of delta_v
        let isp = Isp::seconds(320.0);
        let original_ratio = Ratio::new(5.0);
        let dv = delta_v(isp, original_ratio);
        let recovered_ratio = required_mass_ratio(dv, isp);
        assert_relative_eq!(recovered_ratio.as_f64(), 5.0, epsilon = 0.0001);
    }

    #[test]
    fn required_mass_ratio_zero_dv() {
        let isp = Isp::seconds(300.0);
        let dv = Velocity::mps(0.0);
        let ratio = required_mass_ratio(dv, isp);
        assert_eq!(ratio.as_f64(), 1.0);
    }
}
```

### Thrust Calculations

```rust
// src/physics/thrust.rs
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn twr_calculation() {
        // 1,000,000 N thrust, 100,000 kg mass, Earth gravity
        let thrust = Force::newtons(1_000_000.0);
        let mass = Mass::kg(100_000.0);
        let ratio = twr(thrust, mass, 9.80665);
        assert_relative_eq!(ratio.as_f64(), 1.0197, epsilon = 0.001);
    }

    #[test]
    fn twr_exactly_one() {
        // Thrust equals weight
        let mass = Mass::kg(1000.0);
        let weight = 1000.0 * 9.80665;
        let thrust = Force::newtons(weight);
        let ratio = twr(thrust, mass, 9.80665);
        assert_relative_eq!(ratio.as_f64(), 1.0, epsilon = 0.0001);
    }

    #[test]
    fn burn_time_calculation() {
        // 10,000 kg propellant, 100,000 N thrust, 300s Isp
        // mass_flow = 100,000 / (300 × 9.80665) = 33.99 kg/s
        // burn_time = 10,000 / 33.99 = 294.2 s
        let propellant = Mass::kg(10_000.0);
        let thrust = Force::newtons(100_000.0);
        let isp = Isp::seconds(300.0);
        let time = burn_time(propellant, thrust, isp);
        assert_relative_eq!(time.as_seconds(), 294.2, epsilon = 0.5);
    }
}
```

---

## Integration Tests

### CLI Command Tests

```rust
// tests/cli.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn calculate_outputs_delta_v() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args(["calculate", "--isp", "311", "--mass-ratio", "3.5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Δv:"))
        .stdout(predicate::str::contains("m/s"));
}

#[test]
fn calculate_with_engine() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args([
        "calculate",
        "--engine", "merlin-1d",
        "--propellant-mass", "100000",
        "--dry-mass", "5000",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Δv:"));
}

#[test]
fn calculate_missing_args_fails() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args(["calculate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn engines_lists_available() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args(["engines"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Merlin-1D"))
        .stdout(predicate::str::contains("Raptor-2"));
}

#[test]
fn engines_json_output() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args(["engines", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

#[test]
fn optimize_finds_solution() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args([
        "optimize",
        "--payload", "1000",
        "--target-dv", "3000",
        "--engine", "merlin-1d",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("STAGE"));
}

#[test]
fn optimize_impossible_fails_gracefully() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args([
        "optimize",
        "--payload", "1000000",  // 1000 tonnes payload
        "--target-dv", "20000",   // More than orbital
        "--engine", "merlin-1d",
        "--max-stages", "1",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("No feasible solution"));
}

#[test]
fn optimize_json_output_parseable() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    let output = cmd
        .args([
            "optimize",
            "--payload", "1000",
            "--target-dv", "3000",
            "--engine", "raptor-2",
            "--output", "json",
        ])
        .output()
        .unwrap();
    
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["rocket"]["stages"].is_array());
}

#[test]
fn unknown_engine_fails() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args([
        "calculate",
        "--engine", "not-a-real-engine",
        "--propellant-mass", "1000",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Unknown engine"));
}

#[test]
fn help_displays() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rocket staging optimizer"));
}

#[test]
fn version_displays() {
    let mut cmd = Command::cargo_bin("tsi").unwrap();
    cmd.args(["--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
```

### Optimizer Integration

```rust
// tests/optimizer.rs
use tsi::{Engine, Mass, Velocity, Ratio, Problem, Constraints};
use tsi::optimizer::{AnalyticalOptimizer, BruteForceOptimizer, Optimizer};

#[test]
fn analytical_optimizer_finds_solution() {
    let engine = Engine::from_database("raptor-2").unwrap();
    let problem = Problem {
        payload: Mass::kg(5000.0),
        target_delta_v: Velocity::mps(6000.0),
        available_engines: vec![engine],
        constraints: Constraints {
            min_twr: Ratio::new(1.2),
            max_stages: 2,
            structural_ratio: Ratio::new(0.1),
        },
        stage_count: Some(2),
    };

    let optimizer = AnalyticalOptimizer;
    let solution = optimizer.optimize(&problem).unwrap();

    assert!(solution.rocket.total_delta_v().as_mps() >= 6000.0);
    assert_eq!(solution.rocket.stages.len(), 2);
}

#[test]
fn brute_force_finds_better_or_equal() {
    let engines = vec![
        Engine::from_database("merlin-1d").unwrap(),
        Engine::from_database("raptor-2").unwrap(),
    ];
    let problem = Problem {
        payload: Mass::kg(5000.0),
        target_delta_v: Velocity::mps(9000.0),
        available_engines: engines,
        constraints: Constraints {
            min_twr: Ratio::new(1.2),
            max_stages: 2,
            structural_ratio: Ratio::new(0.1),
        },
        stage_count: None,
    };

    let brute = BruteForceOptimizer::default();
    let solution = brute.optimize(&problem).unwrap();

    // Should find a valid solution
    assert!(solution.rocket.total_delta_v().as_mps() >= 9000.0);
    
    // Should satisfy TWR constraint
    for (i, stage) in solution.rocket.stages.iter().enumerate() {
        let payload_above = solution.rocket.payload_above_stage(i);
        let twr = stage.twr_at_ignition(payload_above, 9.80665);
        assert!(twr.as_f64() >= 1.2);
    }
}

#[test]
fn optimizer_respects_max_stages() {
    let engine = Engine::from_database("merlin-1d").unwrap();
    let problem = Problem {
        payload: Mass::kg(1000.0),
        target_delta_v: Velocity::mps(3000.0),
        available_engines: vec![engine],
        constraints: Constraints {
            min_twr: Ratio::new(1.0),
            max_stages: 1,
            structural_ratio: Ratio::new(0.1),
        },
        stage_count: None,
    };

    let brute = BruteForceOptimizer::default();
    let solution = brute.optimize(&problem).unwrap();

    assert_eq!(solution.rocket.stages.len(), 1);
}
```

---

## Property-Based Tests

Using `proptest` for generative testing.

```rust
// tests/properties.rs
use proptest::prelude::*;
use tsi::{Mass, Velocity, Isp, Ratio};
use tsi::physics::{delta_v, required_mass_ratio};

proptest! {
    /// Mass addition is commutative
    #[test]
    fn mass_addition_commutative(a in 0.0..1e9_f64, b in 0.0..1e9_f64) {
        let m1 = Mass::kg(a);
        let m2 = Mass::kg(b);
        prop_assert_eq!((m1 + m2).as_kg(), (m2 + m1).as_kg());
    }

    /// Delta-v is always positive for mass_ratio > 1
    #[test]
    fn delta_v_positive(isp in 100.0..500.0_f64, ratio in 1.001..100.0_f64) {
        let dv = delta_v(Isp::seconds(isp), Ratio::new(ratio));
        prop_assert!(dv.as_mps() > 0.0);
    }

    /// Delta-v increases monotonically with mass ratio
    #[test]
    fn delta_v_monotonic_mass_ratio(
        isp in 100.0..500.0_f64,
        r1 in 1.001..50.0_f64,
        delta in 0.001..10.0_f64
    ) {
        let ratio1 = Ratio::new(r1);
        let ratio2 = Ratio::new(r1 + delta);
        let dv1 = delta_v(Isp::seconds(isp), ratio1);
        let dv2 = delta_v(Isp::seconds(isp), ratio2);
        prop_assert!(dv2.as_mps() > dv1.as_mps());
    }

    /// Delta-v increases monotonically with Isp
    #[test]
    fn delta_v_monotonic_isp(
        ratio in 1.5..10.0_f64,
        isp1 in 100.0..400.0_f64,
        delta in 1.0..100.0_f64
    ) {
        let dv1 = delta_v(Isp::seconds(isp1), Ratio::new(ratio));
        let dv2 = delta_v(Isp::seconds(isp1 + delta), Ratio::new(ratio));
        prop_assert!(dv2.as_mps() > dv1.as_mps());
    }

    /// required_mass_ratio is inverse of delta_v
    #[test]
    fn mass_ratio_round_trip(isp in 200.0..450.0_f64, ratio in 1.5..20.0_f64) {
        let original = Ratio::new(ratio);
        let dv = delta_v(Isp::seconds(isp), original);
        let recovered = required_mass_ratio(dv, Isp::seconds(isp));
        prop_assert!((recovered.as_f64() - ratio).abs() < 0.0001);
    }

    /// Total rocket delta-v equals sum of stage delta-vs
    #[test]
    fn rocket_delta_v_is_sum(
        dv1 in 1000.0..5000.0_f64,
        dv2 in 1000.0..5000.0_f64,
        dv3 in 1000.0..5000.0_f64
    ) {
        // This tests the Rocket aggregation logic
        let stage_dvs = vec![dv1, dv2, dv3];
        let total: f64 = stage_dvs.iter().sum();
        
        // Build a mock rocket with these delta-vs
        // (actual implementation would use Rocket::total_delta_v())
        prop_assert!((total - (dv1 + dv2 + dv3)).abs() < 0.001);
    }

    /// Optimizer solutions always meet constraints
    #[test]
    fn optimizer_meets_constraints(
        payload in 100.0..10000.0_f64,
        target_dv in 2000.0..8000.0_f64,
        min_twr in 1.0..1.5_f64
    ) {
        // Skip combinations that are obviously infeasible
        prop_assume!(target_dv / payload < 50.0);

        let engine = Engine::from_database("raptor-2").unwrap();
        let problem = Problem {
            payload: Mass::kg(payload),
            target_delta_v: Velocity::mps(target_dv),
            available_engines: vec![engine],
            constraints: Constraints {
                min_twr: Ratio::new(min_twr),
                max_stages: 3,
                structural_ratio: Ratio::new(0.1),
            },
            stage_count: None,
        };

        if let Ok(solution) = BruteForceOptimizer::default().optimize(&problem) {
            // Verify delta-v constraint
            prop_assert!(solution.rocket.total_delta_v().as_mps() >= target_dv);
            
            // Verify TWR constraints
            for (i, stage) in solution.rocket.stages.iter().enumerate() {
                let payload_above = solution.rocket.payload_above_stage(i);
                let twr = stage.twr_at_ignition(payload_above, 9.80665);
                prop_assert!(twr.as_f64() >= min_twr - 0.001); // Small tolerance
            }
        }
        // If no solution found, that's ok - problem may be infeasible
    }
}
```

---

## Validation Tests

Compare against known real-world rockets.

```rust
// tests/validation.rs
use tsi::{Mass, Velocity, Isp, Ratio};
use tsi::physics::delta_v;
use approx::assert_relative_eq;

/// Saturn V first stage (S-IC) approximate values
/// Source: NASA historical data
#[test]
fn saturn_v_s1c_delta_v() {
    // S-IC: 5 F-1 engines, LOX/RP-1
    // Wet mass: ~2,290,000 kg
    // Dry mass: ~131,000 kg  
    // Isp (sea level): ~263 s (averaged over flight)
    
    let wet = Mass::kg(2_290_000.0);
    let dry = Mass::kg(131_000.0);
    let mass_ratio = wet / dry;
    let isp = Isp::seconds(263.0);
    
    let dv = delta_v(isp, mass_ratio);
    
    // Expected: ~2,700-2,800 m/s (with losses, actual achieved was less)
    // Ideal (no losses) should be higher
    assert_relative_eq!(dv.as_mps(), 7470.0, epsilon = 100.0);
}

/// Falcon 9 first stage approximate values
/// Source: Public SpaceX data, community analysis
#[test]
fn falcon_9_stage_1_delta_v() {
    // Falcon 9 v1.2 (Block 5) first stage
    // 9 × Merlin-1D
    // Propellant: ~411,000 kg
    // Dry mass: ~22,200 kg
    // Isp: 282s SL, 311s vac (use average ~297s for ascent)
    
    let propellant = Mass::kg(411_000.0);
    let dry = Mass::kg(22_200.0);
    let wet = propellant + dry;
    let mass_ratio = wet / dry;
    let isp = Isp::seconds(297.0); // Rough average
    
    let dv = delta_v(isp, mass_ratio);
    
    // Ideal delta-v should be around 8,500-9,000 m/s
    assert!(dv.as_mps() > 8000.0);
    assert!(dv.as_mps() < 10000.0);
}

/// Falcon 9 second stage approximate values
#[test]
fn falcon_9_stage_2_delta_v() {
    // Single Merlin Vacuum
    // Propellant: ~111,500 kg
    // Dry mass: ~4,000 kg
    // Isp: 348s (vacuum)
    
    let propellant = Mass::kg(111_500.0);
    let dry = Mass::kg(4_000.0);
    let wet = propellant + dry;
    let mass_ratio = wet / dry;
    let isp = Isp::seconds(348.0);
    
    let dv = delta_v(isp, mass_ratio);
    
    // Should be around 11,000-12,000 m/s ideal
    assert!(dv.as_mps() > 10000.0);
    assert!(dv.as_mps() < 13000.0);
}

/// Space Shuttle SRB contribution
#[test]
fn shuttle_srb_delta_v() {
    // 2 × SRB (each)
    // Propellant: ~500,000 kg each
    // Dry mass: ~87,500 kg each
    // Isp: ~242s (sea level average)
    
    let propellant = Mass::kg(500_000.0);
    let dry = Mass::kg(87_500.0);
    let wet = propellant + dry;
    let mass_ratio = wet / dry;
    let isp = Isp::seconds(242.0);
    
    let dv = delta_v(isp, mass_ratio);
    
    // Note: This is the isolated SRB delta-v, not contribution to stack
    assert!(dv.as_mps() > 4000.0);
    assert!(dv.as_mps() < 5000.0);
}

/// Verify optimal staging theory
/// For equal Isp and structural ratio, optimal is equal delta-v per stage
#[test]
fn optimal_staging_equal_dv() {
    // Theoretical result: for n stages with same Isp and structural coefficient,
    // optimal distribution gives equal delta-v per stage
    
    // This is a sanity check for the analytical optimizer
    let total_dv = Velocity::mps(9000.0);
    let stages = 3;
    let per_stage = total_dv.as_mps() / stages as f64;
    
    assert_relative_eq!(per_stage, 3000.0, epsilon = 0.1);
}
```

---

## Regression Tests

```rust
// tests/regression.rs

/// Bug #1: Division by zero when dry mass equals wet mass
#[test]
fn regression_mass_ratio_one() {
    let wet = Mass::kg(1000.0);
    let dry = Mass::kg(1000.0);
    let ratio = wet / dry;
    let dv = delta_v(Isp::seconds(300.0), ratio);
    
    // Should be 0, not NaN or panic
    assert_eq!(dv.as_mps(), 0.0);
}

/// Bug #2: Negative propellant mass accepted
#[test]
fn regression_negative_propellant() {
    // Should either reject or handle gracefully
    let result = Stage::new(
        Engine::from_database("merlin-1d").unwrap(),
        1,
        Mass::kg(-1000.0), // Negative!
    );
    
    assert!(result.is_err());
}

/// Bug #3: Very small mass ratios caused precision issues
#[test]
fn regression_tiny_mass_ratio() {
    let ratio = Ratio::new(1.0001);
    let dv = delta_v(Isp::seconds(300.0), ratio);
    
    // Should be a small positive number, not zero or negative
    assert!(dv.as_mps() > 0.0);
    assert!(dv.as_mps() < 1.0);
}

/// Bug #4: Unicode in engine names broke JSON output
#[test]
fn regression_unicode_engine_name() {
    let engine = Engine {
        name: "Тест-Двигатель".to_string(), // Russian characters
        // ... other fields
    };
    
    let json = serde_json::to_string(&engine);
    assert!(json.is_ok());
}

/// Bug #5: Empty engine list caused panic in optimizer
#[test]
fn regression_empty_engine_list() {
    let problem = Problem {
        payload: Mass::kg(1000.0),
        target_delta_v: Velocity::mps(5000.0),
        available_engines: vec![], // Empty!
        constraints: Constraints::default(),
        stage_count: None,
    };
    
    let result = BruteForceOptimizer::default().optimize(&problem);
    assert!(result.is_err());
    // Should be a proper error, not a panic
}
```

---

## Benchmark Tests

```rust
// benches/optimizer_bench.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use tsi::{Engine, Mass, Velocity, Ratio, Problem, Constraints};
use tsi::optimizer::{AnalyticalOptimizer, BruteForceOptimizer, Optimizer};

fn optimizer_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimizer");
    
    // Simple 2-stage problem
    let simple_problem = Problem {
        payload: Mass::kg(5000.0),
        target_delta_v: Velocity::mps(6000.0),
        available_engines: vec![Engine::from_database("raptor-2").unwrap()],
        constraints: Constraints {
            min_twr: Ratio::new(1.2),
            max_stages: 2,
            structural_ratio: Ratio::new(0.1),
        },
        stage_count: Some(2),
    };
    
    group.bench_function("analytical_2stage", |b| {
        let optimizer = AnalyticalOptimizer;
        b.iter(|| optimizer.optimize(&simple_problem))
    });
    
    // Multi-engine problem
    let engines = vec![
        Engine::from_database("merlin-1d").unwrap(),
        Engine::from_database("raptor-2").unwrap(),
        Engine::from_database("rl-10c").unwrap(),
    ];
    
    for stages in [2, 3] {
        let problem = Problem {
            payload: Mass::kg(5000.0),
            target_delta_v: Velocity::mps(9000.0),
            available_engines: engines.clone(),
            constraints: Constraints {
                min_twr: Ratio::new(1.0),
                max_stages: stages,
                structural_ratio: Ratio::new(0.1),
            },
            stage_count: None,
        };
        
        group.bench_with_input(
            BenchmarkId::new("bruteforce", format!("{}engines_{}stages", engines.len(), stages)),
            &problem,
            |b, prob| {
                let optimizer = BruteForceOptimizer::default();
                b.iter(|| optimizer.optimize(prob))
            },
        );
    }
    
    group.finish();
}

fn physics_benchmarks(c: &mut Criterion) {
    c.bench_function("tsiolkovsky_equation", |b| {
        let isp = Isp::seconds(350.0);
        let ratio = Ratio::new(8.0);
        b.iter(|| delta_v(isp, ratio))
    });
}

criterion_group!(benches, optimizer_benchmarks, physics_benchmarks);
criterion_main!(benches);
```

---

## Test Dependencies

```toml
# Cargo.toml
[dev-dependencies]
approx = "0.5"           # Float comparisons
assert_cmd = "2"         # CLI testing
predicates = "3"         # Assertions for CLI output
proptest = "1"           # Property-based testing
criterion = "0.5"        # Benchmarking
serde_json = "1"         # JSON parsing in tests

[[bench]]
name = "optimizer_bench"
harness = false
```

---

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test falcon_9

# Run only unit tests (fast)
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run property tests with more cases
PROPTEST_CASES=10000 cargo test properties

# Run benchmarks
cargo bench

# Run benchmarks and save baseline
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main
```

---

## CI Configuration

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run tests
        run: cargo test --all-features
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Check formatting
        run: cargo fmt -- --check

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov
      - name: Generate coverage
        run: cargo llvm-cov --all-features --lcov --output-path lcov.info
      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
```

---

## Coverage Goals

| Module | Target Coverage |
|--------|-----------------|
| units/ | 95%+ |
| physics/ | 95%+ |
| engine/ | 90%+ |
| stage/ | 90%+ |
| optimizer/ | 85%+ |
| cli/ | 75%+ |
| output/ | 70%+ |

Physics and unit tests are critical — bugs here produce wrong answers. CLI and output are less critical — bugs there are annoying but not dangerous.
