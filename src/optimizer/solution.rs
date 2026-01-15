//! Optimization solution representation.
//!
//! A solution contains the optimal rocket configuration found by the optimizer,
//! along with metadata about the optimization process.

use crate::stage::Rocket;
use crate::units::Velocity;

/// Result of an optimization run.
///
/// Contains the optimal rocket configuration and metadata about
/// how well it meets the requirements.
///
/// # Margin
///
/// The margin is the excess delta-v beyond the target. A positive
/// margin provides safety margin for:
/// - Gravity losses (typically 1,000-1,500 m/s)
/// - Atmospheric drag (typically 100-400 m/s)
/// - Navigation corrections
/// - Propellant reserves
///
/// # Example
///
/// ```ignore
/// let solution = optimizer.optimize(&problem)?;
///
/// println!("Total mass: {}", solution.rocket.total_mass());
/// println!("Delta-v margin: +{} m/s", solution.margin.as_mps());
/// println!("Payload fraction: {:.2}%", solution.payload_fraction_percent());
/// ```
#[derive(Debug, Clone)]
pub struct Solution {
    /// The optimized rocket configuration
    pub rocket: Rocket,

    /// Delta-v margin beyond target (positive = excess capacity)
    pub margin: Velocity,

    /// Number of iterations/configurations evaluated
    pub iterations: u64,
}

impl Solution {
    /// Create a new solution.
    pub fn new(rocket: Rocket, target_dv: Velocity, iterations: u64) -> Self {
        let actual_dv = rocket.total_delta_v();
        let margin = Velocity::mps(actual_dv.as_mps() - target_dv.as_mps());
        Self {
            rocket,
            margin,
            iterations,
        }
    }

    /// Payload fraction as a percentage.
    pub fn payload_fraction_percent(&self) -> f64 {
        self.rocket.payload_fraction().as_f64() * 100.0
    }

    /// Whether the solution meets or exceeds the target delta-v.
    pub fn meets_target(&self) -> bool {
        self.margin.as_mps() >= 0.0
    }

    /// Margin as a percentage of target delta-v.
    pub fn margin_percent(&self, target_dv: Velocity) -> f64 {
        (self.margin.as_mps() / target_dv.as_mps()) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;
    use crate::stage::Stage;
    use crate::units::Mass;

    fn simple_rocket() -> Rocket {
        let db = EngineDatabase::default();
        let raptor = db.get("Raptor-2").unwrap().clone();

        let stage1 = Stage::with_structural_ratio(raptor.clone(), 9, Mass::kg(1_000_000.0), 0.05);
        let stage2 = Stage::with_structural_ratio(raptor, 1, Mass::kg(100_000.0), 0.08);

        Rocket::new(vec![stage1, stage2], Mass::kg(50_000.0))
    }

    #[test]
    fn solution_construction() {
        let rocket = simple_rocket();
        let target_dv = Velocity::mps(8_000.0);
        let solution = Solution::new(rocket, target_dv, 100);

        // Should have positive margin (rocket has ~9,200 m/s)
        assert!(solution.meets_target());
        assert!(solution.margin.as_mps() > 0.0);
    }

    #[test]
    fn solution_margin_percent() {
        let rocket = simple_rocket();
        let target_dv = Velocity::mps(8_000.0);
        let solution = Solution::new(rocket, target_dv, 100);

        let margin_pct = solution.margin_percent(target_dv);
        assert!(margin_pct > 0.0);
    }

    #[test]
    fn solution_payload_fraction() {
        let rocket = simple_rocket();
        let target_dv = Velocity::mps(8_000.0);
        let solution = Solution::new(rocket, target_dv, 100);

        let payload_pct = solution.payload_fraction_percent();
        assert!(payload_pct > 1.0); // At least 1%
        assert!(payload_pct < 10.0); // Less than 10%
    }
}
