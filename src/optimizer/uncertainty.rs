//! Uncertainty modeling for Monte Carlo analysis.
//!
//! Real rocket parameters have manufacturing tolerances and measurement
//! uncertainties. This module describes those variations and draws perturbed
//! builds of a design from them.
//!
//! # Uncertainty Model
//!
//! Parameters are modeled with percentage uncertainties, assuming a normal
//! (Gaussian) distribution. The uncertainty percentage is one standard
//! deviation, so an Isp uncertainty of ±2% means:
//!
//! - 68% of builds fall within ±2% of nominal
//! - 95% within ±4% (2-sigma)
//! - 99.7% within ±6% (3-sigma)
//!
//! # Physical Basis
//!
//! | Parameter | Typical Range | Sources |
//! |-----------|---------------|---------|
//! | ISP | ±1-2% | Combustion efficiency, nozzle wear |
//! | Thrust | ±1-3% | Chamber pressure, propellant temp |
//! | Structural | ±3-10% | Weld quality, material variation |
//!
//! # References
//!
//! - NASA-STD-8729.1: "Planning, Developing, and Managing an Effective
//!   Reliability and Maintainability Program"
//! - AIAA S-120A-2015: "Mass Properties Control for Space Systems"

use rand::Rng;
use rand_distr::{Distribution, Normal};

use crate::stage::{Rocket, Stage};

/// Smallest multiplicative factor a perturbation may produce. A normal
/// distribution has tails that reach zero and below; an engine can't have
/// negative Isp, so draws are truncated here. With realistic uncertainties
/// (a few percent) the truncation never happens.
const MIN_FACTOR: f64 = 0.01;

/// Uncertainty specification for Monte Carlo analysis.
///
/// Each value is a percentage, one standard deviation: 2.0 means ±2% at
/// 1-sigma. Build one from a preset and adjust it by name, so that the three
/// numbers can't be swapped by accident:
///
/// ```
/// use tsiolkovsky::optimizer::Uncertainty;
///
/// // Typical uncertainties for a well-characterized engine
/// let production = Uncertainty::default();
/// assert_eq!(production.isp_percent(), 1.0);
///
/// // An engine still in development
/// let development = Uncertainty::default()
///     .with_isp_percent(3.0)
///     .with_structural_percent(10.0);
/// assert_eq!(development.thrust_percent(), 2.0);
/// ```
///
/// # Presets
///
/// | Preset | Isp | Thrust | Structure |
/// |--------|-----|--------|-----------|
/// | [`none`](Self::none) | 0% | 0% | 0% |
/// | [`low`](Self::low) | 0.5% | 1% | 3% |
/// | [`default`](Self::default) | 1% | 2% | 5% |
/// | [`high`](Self::high) | 2% | 3% | 8% |
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Uncertainty {
    isp_percent: f64,
    thrust_percent: f64,
    structural_percent: f64,
}

impl Default for Uncertainty {
    /// Uncertainties for well-characterized production hardware: Isp ±1%,
    /// thrust ±2%, structural mass ±5%.
    fn default() -> Self {
        Self {
            isp_percent: 1.0,
            thrust_percent: 2.0,
            structural_percent: 5.0,
        }
    }
}

/// An uncertainty that isn't a usable standard deviation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum UncertaintyError {
    /// A percentage is negative, infinite or not a number.
    #[error("{parameter} uncertainty must be zero or a positive percentage, got {value}")]
    InvalidPercent { parameter: &'static str, value: f64 },
}

impl Uncertainty {
    /// No uncertainty: every build is the nominal design.
    pub fn none() -> Self {
        Self {
            isp_percent: 0.0,
            thrust_percent: 0.0,
            structural_percent: 0.0,
        }
    }

    /// Mature, flight-proven hardware: Isp ±0.5%, thrust ±1%, structure ±3%.
    pub fn low() -> Self {
        Self {
            isp_percent: 0.5,
            thrust_percent: 1.0,
            structural_percent: 3.0,
        }
    }

    /// Development hardware: Isp ±2%, thrust ±3%, structure ±8%.
    pub fn high() -> Self {
        Self {
            isp_percent: 2.0,
            thrust_percent: 3.0,
            structural_percent: 8.0,
        }
    }

    /// Set the Isp uncertainty (percent, 1-sigma).
    pub fn with_isp_percent(mut self, percent: f64) -> Self {
        self.isp_percent = percent;
        self
    }

    /// Set the thrust uncertainty (percent, 1-sigma).
    pub fn with_thrust_percent(mut self, percent: f64) -> Self {
        self.thrust_percent = percent;
        self
    }

    /// Set the structural mass uncertainty (percent, 1-sigma).
    pub fn with_structural_percent(mut self, percent: f64) -> Self {
        self.structural_percent = percent;
        self
    }

    /// Isp uncertainty, percent 1-sigma.
    pub fn isp_percent(&self) -> f64 {
        self.isp_percent
    }

    /// Thrust uncertainty, percent 1-sigma.
    pub fn thrust_percent(&self) -> f64 {
        self.thrust_percent
    }

    /// Structural mass uncertainty, percent 1-sigma.
    pub fn structural_percent(&self) -> f64 {
        self.structural_percent
    }

    /// Check that every percentage is a usable standard deviation.
    pub fn validate(&self) -> Result<(), UncertaintyError> {
        for (parameter, value) in [
            ("Isp", self.isp_percent),
            ("thrust", self.thrust_percent),
            ("structural", self.structural_percent),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                return Err(UncertaintyError::InvalidPercent { parameter, value });
            }
        }
        Ok(())
    }

    /// Check if every uncertainty is zero.
    pub fn is_zero(&self) -> bool {
        self.isp_percent == 0.0 && self.thrust_percent == 0.0 && self.structural_percent == 0.0
    }
}

/// Draws perturbed builds of a design.
///
/// Crate-private so that `rand`'s types stay out of the public API; the
/// public face of this is [`MonteCarloRunner`](super::MonteCarloRunner).
#[derive(Debug, Clone)]
pub(crate) struct ParameterSampler {
    uncertainty: Uncertainty,
}

impl ParameterSampler {
    /// A sampler for a validated uncertainty.
    pub(crate) fn new(uncertainty: Uncertainty) -> Self {
        Self { uncertainty }
    }

    /// Build a rocket again with as-built errors.
    ///
    /// Each stage gets one Isp factor (scaling sea-level and vacuum Isp
    /// together: the errors share causes, and perturbing them separately could
    /// make an engine better at sea level than in vacuum), one thrust factor
    /// likewise, and one factor on its structural mass. Propellant loads,
    /// engine counts and payload are unchanged: this is the same design,
    /// built imperfectly.
    pub(crate) fn perturb_rocket<R: Rng>(&self, rocket: &Rocket, rng: &mut R) -> Rocket {
        let stages = rocket
            .stages()
            .iter()
            .map(|stage| {
                let isp = self.factor(self.uncertainty.isp_percent, rng);
                let thrust = self.factor(self.uncertainty.thrust_percent, rng);
                let structure = self.factor(self.uncertainty.structural_percent, rng);
                Stage::from_parts(
                    stage.engine().scaled(isp, thrust),
                    stage.engine_count(),
                    stage.propellant_mass(),
                    stage.structural_mass() * structure,
                )
            })
            .collect();
        rocket.with_stages(stages)
    }

    /// Draw a multiplicative factor N(1, percent/100), truncated below at
    /// [`MIN_FACTOR`], or exactly 1 for zero uncertainty.
    fn factor<R: Rng>(&self, percent: f64, rng: &mut R) -> f64 {
        if percent == 0.0 {
            return 1.0;
        }
        // A validated uncertainty always makes a valid distribution; if one
        // slips through, no perturbation is safer than a panic.
        Normal::new(1.0, percent / 100.0).map_or(1.0, |n| n.sample(rng).max(MIN_FACTOR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;
    use crate::units::Mass;
    use proptest::prelude::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn rocket() -> Rocket {
        let db = EngineDatabase::default();
        let raptor = db.get("raptor-2").unwrap().clone();
        Rocket::new(
            vec![
                Stage::with_structural_ratio(raptor.clone(), 2, Mass::kg(150_000.0), 0.08).unwrap(),
                Stage::with_structural_ratio(raptor, 1, Mass::kg(30_000.0), 0.08).unwrap(),
            ],
            Mass::kg(5_000.0),
        )
        .unwrap()
    }

    #[test]
    fn presets() {
        assert!(Uncertainty::none().is_zero());
        assert_eq!(Uncertainty::low().structural_percent(), 3.0);
        assert_eq!(Uncertainty::default().thrust_percent(), 2.0);
        assert_eq!(Uncertainty::high().isp_percent(), 2.0);
    }

    #[test]
    fn named_setters_change_only_their_field() {
        let u = Uncertainty::default().with_structural_percent(8.0);
        assert_eq!(u.isp_percent(), 1.0);
        assert_eq!(u.thrust_percent(), 2.0);
        assert_eq!(u.structural_percent(), 8.0);
    }

    #[test]
    fn validation_rejects_negative_and_nan() {
        assert!(Uncertainty::default().validate().is_ok());
        assert!(Uncertainty::default()
            .with_isp_percent(-1.0)
            .validate()
            .is_err());
        assert!(Uncertainty::default()
            .with_thrust_percent(f64::NAN)
            .validate()
            .is_err());
    }

    #[test]
    fn isp_factor_has_the_right_spread() {
        let sampler = ParameterSampler::new(Uncertainty::default());
        let mut rng = StdRng::seed_from_u64(1);
        let samples: Vec<f64> = (0..20_000).map(|_| sampler.factor(1.0, &mut rng)).collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let var =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (samples.len() - 1) as f64;
        assert!((mean - 1.0).abs() < 0.001, "mean {mean}");
        assert!((var.sqrt() - 0.01).abs() < 0.0005, "std {}", var.sqrt());
    }

    #[test]
    fn zero_uncertainty_builds_the_nominal_rocket() {
        let sampler = ParameterSampler::new(Uncertainty::none());
        let nominal = rocket();
        let built = sampler.perturb_rocket(&nominal, &mut StdRng::seed_from_u64(1));
        assert_eq!(built.total_mass(), nominal.total_mass());
        assert_eq!(built.total_delta_v(), nominal.total_delta_v());
    }

    #[test]
    fn perturbation_keeps_the_design() {
        let sampler = ParameterSampler::new(Uncertainty::high());
        let nominal = rocket();
        let built = sampler.perturb_rocket(&nominal, &mut StdRng::seed_from_u64(2));
        for (a, b) in built.stages().iter().zip(nominal.stages()) {
            assert_eq!(a.engine_count(), b.engine_count());
            assert_eq!(a.propellant_mass(), b.propellant_mass());
            assert_eq!(a.engine().name(), b.engine().name());
        }
        assert_eq!(built.payload(), nominal.payload());
        assert_eq!(built.booster_isp(), nominal.booster_isp());
        assert_eq!(built.surface_gravity(), nominal.surface_gravity());
    }

    proptest! {
        /// A perturbed engine is never better at sea level than in vacuum,
        /// and never has a negative Isp, however wide the uncertainty.
        #[test]
        fn perturbed_engines_stay_physical(
            isp_percent in 0.0..200.0_f64,
            thrust_percent in 0.0..200.0_f64,
            seed in any::<u64>(),
        ) {
            let sampler = ParameterSampler::new(
                Uncertainty::none()
                    .with_isp_percent(isp_percent)
                    .with_thrust_percent(thrust_percent),
            );
            let built = sampler.perturb_rocket(&rocket(), &mut StdRng::seed_from_u64(seed));
            for stage in built.stages() {
                let e = stage.engine();
                prop_assert!(e.isp_sl().as_seconds() <= e.isp_vac().as_seconds());
                prop_assert!(e.thrust_sl().as_newtons() <= e.thrust_vac().as_newtons());
                prop_assert!(e.isp_sl().as_seconds() > 0.0);
            }
        }
    }
}
