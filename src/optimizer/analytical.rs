//! Analytical optimizer for two-stage rockets.
//!
//! When using a single engine type with identical structural ratios across stages,
//! there exists a closed-form solution for optimal mass distribution. This optimizer
//! implements the Lagrange multiplier solution for the two-stage case.
//!
//! # Theory
//!
//! For a two-stage rocket with identical specific impulse and structural coefficient,
//! the optimal staging splits delta-v equally between stages. This is derived from
//! the calculus of variations applied to the rocket equation.
//!
//! The optimal mass ratio for each stage is:
//!
//! ```text
//! R* = exp(Δv_total / (2 × Isp × g₀))
//! ```
//!
//! Where:
//! - R* is the optimal mass ratio per stage
//! - Δv_total is the total required delta-v
//! - Isp is the specific impulse
//! - g₀ is standard gravity (9.80665 m/s²)
//!
//! # Limitations
//!
//! This optimizer only handles:
//! - Exactly 2 stages
//! - Single engine type
//! - Uniform structural ratio across stages
//!
//! For more complex cases, use the brute force optimizer.
//!
//! # References
//!
//! - Sutton, G.P. "Rocket Propulsion Elements", Chapter 4
//! - Curtis, H.D. "Orbital Mechanics for Engineering Students", Chapter 11

use std::time::Instant;

use crate::engine::Engine;
use crate::physics::{required_mass_ratio, G0};
use crate::stage::{Rocket, Stage};
use crate::units::{Mass, Ratio, Velocity};

use super::{OptimizeError, Optimizer, Problem, Solution};

/// Analytical optimizer for two-stage, single-engine rockets.
///
/// Uses closed-form Lagrange multiplier solution for optimal staging.
/// This is the fastest optimizer but only works for simple cases.
///
/// # When to Use
///
/// - Single engine type for all stages
/// - Exactly 2 stages
/// - Same structural ratio for both stages
/// - Need quick results for preliminary design
///
/// # Example
///
/// ```
/// use tsi::optimizer::{AnalyticalOptimizer, Problem, Constraints, Optimizer};
/// use tsi::engine::EngineDatabase;
/// use tsi::units::{Mass, Velocity};
///
/// let db = EngineDatabase::load_embedded().expect("failed to load database");
/// let raptor = db.get("raptor-2").expect("engine not found");
///
/// let problem = Problem::new(
///     Mass::kg(5_000.0),
///     Velocity::mps(9_400.0),
///     vec![raptor.clone()],
///     Constraints::default(),
/// ).with_stage_count(2);
///
/// let optimizer = AnalyticalOptimizer;
/// let solution = optimizer.optimize(&problem).expect("optimization failed");
///
/// assert!(solution.meets_target());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct AnalyticalOptimizer;

impl AnalyticalOptimizer {
    /// Calculate optimal propellant mass per stage.
    ///
    /// For equal delta-v split, each stage needs mass ratio R* such that:
    /// Δv_stage = Isp × g₀ × ln(R*)
    ///
    /// Given R* and structural ratio ε, we solve for propellant mass:
    /// wet/dry = R*
    /// (propellant + structure + engine + payload) / (structure + engine + payload) = R*
    fn calculate_stage_propellant(
        target_dv_per_stage: Velocity,
        engine: &Engine,
        engine_count: u32,
        structural_ratio: Ratio,
        payload_above: Mass,
    ) -> Result<Mass, OptimizeError> {
        // Calculate required mass ratio for this stage's delta-v
        let required_ratio = required_mass_ratio(target_dv_per_stage, engine.isp_vac());

        if required_ratio.as_f64() < 1.0 {
            return Err(OptimizeError::Infeasible {
                reason: "Required mass ratio < 1.0 (impossible)".to_string(),
            });
        }

        // Engine mass contribution
        let engine_mass = engine.dry_mass().as_kg() * engine_count as f64;

        // Solve for propellant mass:
        // Let m_p = propellant, m_s = structural = ε × m_p, m_e = engine, m_pay = payload above
        // wet = m_p + m_s + m_e + m_pay = m_p(1 + ε) + m_e + m_pay
        // dry = m_s + m_e + m_pay = ε×m_p + m_e + m_pay
        // R = wet/dry
        //
        // R × (ε×m_p + m_e + m_pay) = m_p(1 + ε) + m_e + m_pay
        // R×ε×m_p + R×(m_e + m_pay) = m_p + ε×m_p + m_e + m_pay
        // R×ε×m_p - m_p - ε×m_p = m_e + m_pay - R×(m_e + m_pay)
        // m_p × (R×ε - 1 - ε) = (m_e + m_pay) × (1 - R)
        // m_p = (m_e + m_pay) × (1 - R) / (R×ε - 1 - ε)
        //
        // But (1 - R) is negative since R > 1, and (R×ε - 1 - ε) needs checking.
        // Let's rearrange:
        // m_p = (m_e + m_pay) × (R - 1) / (1 + ε - R×ε)
        // m_p = (m_e + m_pay) × (R - 1) / (1 + ε×(1 - R))

        let r = required_ratio.as_f64();
        let eps = structural_ratio.as_f64();
        let fixed_mass = engine_mass + payload_above.as_kg();

        let numerator = fixed_mass * (r - 1.0);
        let denominator = 1.0 + eps * (1.0 - r);

        if denominator <= 0.0 {
            return Err(OptimizeError::Infeasible {
                reason: format!(
                    "Structural ratio {} too high for required mass ratio {:.2}",
                    eps, r
                ),
            });
        }

        let propellant_mass = numerator / denominator;

        if propellant_mass <= 0.0 {
            return Err(OptimizeError::Infeasible {
                reason: "Calculated propellant mass is non-positive".to_string(),
            });
        }

        Ok(Mass::kg(propellant_mass))
    }

    /// Determine optimal engine count for a stage.
    ///
    /// Starts with 1 engine and increases until TWR constraint is met.
    fn determine_engine_count(
        engine: &Engine,
        propellant_mass: Mass,
        structural_ratio: Ratio,
        payload_above: Mass,
        min_twr: Ratio,
        max_engines: u32,
        is_first_stage: bool,
    ) -> Result<u32, OptimizeError> {
        for count in 1..=max_engines {
            let stage = Stage::with_structural_ratio(
                engine.clone(),
                count,
                propellant_mass,
                structural_ratio.as_f64(),
            );

            let total_mass = stage.wet_mass() + payload_above;

            // Use sea-level thrust for first stage, vacuum for upper stages
            let thrust = if is_first_stage {
                stage.thrust_sl()
            } else {
                stage.thrust_vac()
            };

            let twr = Ratio::new(thrust.as_newtons() / (total_mass.as_kg() * G0));

            if twr.as_f64() >= min_twr.as_f64() {
                return Ok(count);
            }
        }

        Err(OptimizeError::Infeasible {
            reason: format!(
                "Cannot achieve TWR {} with up to {} engines",
                min_twr, max_engines
            ),
        })
    }
}

impl Optimizer for AnalyticalOptimizer {
    fn optimize(&self, problem: &Problem) -> Result<Solution, OptimizeError> {
        let start = Instant::now();

        // Validate the problem
        problem.is_valid()?;

        // Check that this optimizer can handle the problem
        if !problem.is_single_engine() {
            return Err(OptimizeError::Unsupported {
                reason: "Analytical optimizer requires single engine type".to_string(),
            });
        }

        let stage_count = problem.stage_count.unwrap_or(2);
        if stage_count != 2 {
            return Err(OptimizeError::Unsupported {
                reason: format!(
                    "Analytical optimizer only supports 2 stages, got {}",
                    stage_count
                ),
            });
        }

        let engine = problem.single_engine().unwrap();
        let constraints = &problem.constraints;

        // Add 2% margin to target delta-v to account for rounding and ensure we meet target
        let target_with_margin = Velocity::mps(problem.target_delta_v.as_mps() * 1.02);

        // For optimal 2-stage, split delta-v equally
        let dv_per_stage = Velocity::mps(target_with_margin.as_mps() / 2.0);

        // Calculate upper stage (stage 2) first
        // Start with 1 engine, then iterate
        let mut stage2_engine_count = 1u32;
        let mut stage2_propellant;

        loop {
            stage2_propellant = Self::calculate_stage_propellant(
                dv_per_stage,
                engine,
                stage2_engine_count,
                constraints.structural_ratio,
                problem.payload,
            )?;

            // Check if we need more engines for TWR
            let needed_engines = Self::determine_engine_count(
                engine,
                stage2_propellant,
                constraints.structural_ratio,
                problem.payload,
                constraints.min_stage_twr,
                constraints.max_engines_per_stage,
                false,
            )?;

            if needed_engines == stage2_engine_count {
                break;
            }
            stage2_engine_count = needed_engines;
        }

        // Create upper stage
        let stage2 = Stage::with_structural_ratio(
            engine.clone(),
            stage2_engine_count,
            stage2_propellant,
            constraints.structural_ratio.as_f64(),
        );

        // Calculate first stage (stage 1)
        // It carries stage 2 + payload
        let payload_above_stage1 = stage2.wet_mass() + problem.payload;

        let mut stage1_engine_count = 1u32;
        let mut stage1_propellant;

        loop {
            stage1_propellant = Self::calculate_stage_propellant(
                dv_per_stage,
                engine,
                stage1_engine_count,
                constraints.structural_ratio,
                payload_above_stage1,
            )?;

            // Check if we need more engines for TWR
            let needed_engines = Self::determine_engine_count(
                engine,
                stage1_propellant,
                constraints.structural_ratio,
                payload_above_stage1,
                constraints.min_liftoff_twr,
                constraints.max_engines_per_stage,
                true,
            )?;

            if needed_engines == stage1_engine_count {
                break;
            }
            stage1_engine_count = needed_engines;
        }

        // Create first stage
        let stage1 = Stage::with_structural_ratio(
            engine.clone(),
            stage1_engine_count,
            stage1_propellant,
            constraints.structural_ratio.as_f64(),
        );

        // Assemble rocket
        let rocket = Rocket::new(vec![stage1, stage2], problem.payload);

        // Validate TWR constraints
        rocket
            .validate_twr(constraints.min_stage_twr, true)
            .map_err(|e| OptimizeError::Infeasible {
                reason: e.to_string(),
            })?;

        // Create solution with metadata
        let solution = Solution::with_metadata(
            rocket,
            problem.target_delta_v,
            1, // Analytical optimizer evaluates a single configuration
            start.elapsed(),
            "Analytical",
        );

        // Verify we meet the target (with small tolerance for floating point)
        if solution.margin.as_mps() < -1.0 {
            return Err(OptimizeError::Infeasible {
                reason: format!(
                    "Solution delta-v {:.0} m/s is below target {:.0} m/s",
                    solution.rocket.total_delta_v().as_mps(),
                    problem.target_delta_v.as_mps()
                ),
            });
        }

        Ok(solution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;
    use crate::optimizer::Constraints;

    fn get_raptor() -> Engine {
        let db = EngineDatabase::default();
        db.get("Raptor-2").unwrap().clone()
    }

    fn get_merlin() -> Engine {
        let db = EngineDatabase::default();
        db.get("Merlin-1D").unwrap().clone()
    }

    #[test]
    fn analytical_optimizer_basic() {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get_raptor()],
            Constraints::default(),
        )
        .with_stage_count(2);

        let optimizer = AnalyticalOptimizer;
        let solution = optimizer.optimize(&problem).unwrap();

        // Should meet target
        assert!(solution.meets_target());

        // Should have 2 stages
        assert_eq!(solution.rocket.stage_count(), 2);

        // Payload fraction should be reasonable
        let payload_pct = solution.payload_fraction_percent();
        assert!(payload_pct > 0.5);
        assert!(payload_pct < 20.0);
    }

    #[test]
    fn analytical_optimizer_merlin() {
        let problem = Problem::new(
            Mass::kg(10_000.0),
            Velocity::mps(8_000.0),
            vec![get_merlin()],
            Constraints::default(),
        )
        .with_stage_count(2);

        let optimizer = AnalyticalOptimizer;
        let solution = optimizer.optimize(&problem).unwrap();

        assert!(solution.meets_target());
        assert_eq!(solution.rocket.stage_count(), 2);
    }

    #[test]
    fn analytical_optimizer_fails_multi_engine() {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get_raptor(), get_merlin()], // Multiple engine types
            Constraints::default(),
        )
        .with_stage_count(2);

        let optimizer = AnalyticalOptimizer;
        let result = optimizer.optimize(&problem);

        assert!(matches!(result, Err(OptimizeError::Unsupported { .. })));
    }

    #[test]
    fn analytical_optimizer_fails_three_stages() {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get_raptor()],
            Constraints::default(),
        )
        .with_stage_count(3); // Not supported

        let optimizer = AnalyticalOptimizer;
        let result = optimizer.optimize(&problem);

        assert!(matches!(result, Err(OptimizeError::Unsupported { .. })));
    }

    #[test]
    fn analytical_optimizer_meets_twr() {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get_raptor()],
            Constraints::new(Ratio::new(1.3), Ratio::new(0.7), 3, Ratio::new(0.08)),
        )
        .with_stage_count(2);

        let optimizer = AnalyticalOptimizer;
        let solution = optimizer.optimize(&problem).unwrap();

        // First stage TWR should be >= 1.3
        let liftoff_twr = solution.rocket.liftoff_twr();
        assert!(liftoff_twr.as_f64() >= 1.3);

        // Upper stage TWR should be >= 0.7
        let upper_twr = solution.rocket.stage_twr(1);
        assert!(upper_twr.as_f64() >= 0.7);
    }

    #[test]
    fn analytical_optimizer_high_delta_v() {
        // Test with high delta-v requirement (LEO + margin)
        let problem = Problem::new(
            Mass::kg(1_000.0),
            Velocity::mps(10_000.0),
            vec![get_raptor()],
            Constraints::default(),
        )
        .with_stage_count(2);

        let optimizer = AnalyticalOptimizer;
        let solution = optimizer.optimize(&problem).unwrap();

        assert!(solution.meets_target());
        assert!(solution.rocket.total_delta_v().as_mps() >= 10_000.0);
    }

    #[test]
    fn analytical_optimizer_infeasible_delta_v() {
        // Test with impossibly high delta-v
        let problem = Problem::new(
            Mass::kg(100_000.0), // Heavy payload
            Velocity::mps(50_000.0), // Way too high
            vec![get_raptor()],
            Constraints::default(),
        )
        .with_stage_count(2);

        let optimizer = AnalyticalOptimizer;
        let result = optimizer.optimize(&problem);

        // Should fail as infeasible
        assert!(matches!(result, Err(OptimizeError::Infeasible { .. })));
    }
}
