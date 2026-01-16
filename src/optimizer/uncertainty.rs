//! Uncertainty modeling for Monte Carlo analysis.
//!
//! Real rocket parameters have manufacturing tolerances and measurement
//! uncertainties. This module provides types to model these variations
//! and sample perturbed parameters for Monte Carlo simulation.
//!
//! # Uncertainty Model
//!
//! Parameters are modeled with percentage uncertainties, assuming
//! a normal (Gaussian) distribution. The uncertainty percentage
//! represents a 1-sigma (68%) confidence interval.
//!
//! For example, an ISP uncertainty of ±2% means:
//! - 68% of samples fall within ±2% of nominal
//! - 95% of samples fall within ±4% of nominal (2-sigma)
//! - 99.7% of samples fall within ±6% of nominal (3-sigma)
//!
//! # Example
//!
//! ```
//! use tsi::optimizer::Uncertainty;
//!
//! // Typical uncertainties for a well-characterized engine
//! let uncertainty = Uncertainty::default();
//! assert!((uncertainty.isp_percent - 1.0).abs() < 0.01);
//!
//! // Custom uncertainty for early development
//! let high_uncertainty = Uncertainty::new(3.0, 5.0, 2.0);
//! ```
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

use crate::engine::Engine;
use crate::units::{Force, Isp, Mass, Ratio};

/// Uncertainty specification for Monte Carlo analysis.
///
/// All values are expressed as percentages (1-sigma).
/// A value of 2.0 means ±2% uncertainty at 1-sigma.
///
/// # Default Values
///
/// The defaults represent typical uncertainties for a
/// well-characterized production engine:
///
/// - ISP: ±1% (combustion is well-understood)
/// - Thrust: ±2% (chamber pressure varies)
/// - Structural: ±5% (manufacturing tolerances)
#[derive(Debug, Clone, Copy)]
pub struct Uncertainty {
    /// ISP uncertainty as percentage (1-sigma)
    pub isp_percent: f64,

    /// Thrust uncertainty as percentage (1-sigma)
    pub thrust_percent: f64,

    /// Structural mass ratio uncertainty as percentage (1-sigma)
    pub structural_percent: f64,
}

impl Default for Uncertainty {
    /// Default uncertainties for well-characterized engines.
    ///
    /// These values are conservative for production hardware.
    /// Development engines may have higher uncertainties.
    fn default() -> Self {
        Self {
            isp_percent: 1.0,
            thrust_percent: 2.0,
            structural_percent: 5.0,
        }
    }
}

impl Uncertainty {
    /// Create custom uncertainty specification.
    ///
    /// All values are percentages (1-sigma).
    ///
    /// # Arguments
    ///
    /// * `isp_percent` - ISP uncertainty (typically 1-3%)
    /// * `structural_percent` - Structural ratio uncertainty (typically 3-10%)
    /// * `thrust_percent` - Thrust uncertainty (typically 1-3%)
    ///
    /// # Example
    ///
    /// ```
    /// use tsi::optimizer::Uncertainty;
    ///
    /// // High uncertainty for development engine
    /// let dev_uncertainty = Uncertainty::new(3.0, 10.0, 3.0);
    /// ```
    pub fn new(isp_percent: f64, structural_percent: f64, thrust_percent: f64) -> Self {
        Self {
            isp_percent,
            structural_percent,
            thrust_percent,
        }
    }

    /// Create zero uncertainty (for deterministic analysis).
    pub fn none() -> Self {
        Self {
            isp_percent: 0.0,
            thrust_percent: 0.0,
            structural_percent: 0.0,
        }
    }

    /// Check if any uncertainty is specified.
    pub fn is_zero(&self) -> bool {
        self.isp_percent == 0.0 && self.thrust_percent == 0.0 && self.structural_percent == 0.0
    }
}

/// Samples perturbed parameter values based on uncertainty.
///
/// Uses normal distributions to generate random variations
/// around nominal values. Thread-safe and can be used with
/// rayon for parallel Monte Carlo.
///
/// # Example
///
/// ```
/// use tsi::optimizer::{Uncertainty, ParameterSampler};
/// use tsi::units::Isp;
///
/// let sampler = ParameterSampler::new(Uncertainty::default());
/// let nominal_isp = Isp::seconds(350.0);
///
/// // Generate 1000 perturbed ISP values
/// let samples: Vec<Isp> = (0..1000)
///     .map(|_| sampler.perturb_isp(nominal_isp))
///     .collect();
///
/// // Mean should be close to nominal
/// let mean: f64 = samples.iter().map(|i| i.as_seconds()).sum::<f64>() / 1000.0;
/// assert!((mean - 350.0).abs() < 5.0);
/// ```
#[derive(Debug, Clone)]
pub struct ParameterSampler {
    uncertainty: Uncertainty,
}

impl ParameterSampler {
    /// Create a new sampler with the given uncertainty specification.
    pub fn new(uncertainty: Uncertainty) -> Self {
        Self { uncertainty }
    }

    /// Perturb an ISP value based on uncertainty.
    ///
    /// Samples from a normal distribution centered on the nominal
    /// value with standard deviation based on the ISP uncertainty.
    pub fn perturb_isp(&self, nominal: Isp) -> Isp {
        if self.uncertainty.isp_percent == 0.0 {
            return nominal;
        }
        let factor = self.sample_factor(self.uncertainty.isp_percent);
        Isp::seconds(nominal.as_seconds() * factor)
    }

    /// Perturb a thrust value based on uncertainty.
    pub fn perturb_thrust(&self, nominal: Force) -> Force {
        if self.uncertainty.thrust_percent == 0.0 {
            return nominal;
        }
        let factor = self.sample_factor(self.uncertainty.thrust_percent);
        Force::newtons(nominal.as_newtons() * factor)
    }

    /// Perturb a structural ratio based on uncertainty.
    ///
    /// The result is clamped to valid range (0.01 to 0.5).
    pub fn perturb_structural_ratio(&self, nominal: Ratio) -> Ratio {
        if self.uncertainty.structural_percent == 0.0 {
            return nominal;
        }
        let factor = self.sample_factor(self.uncertainty.structural_percent);
        let perturbed = nominal.as_f64() * factor;
        // Clamp to reasonable range
        Ratio::new(perturbed.clamp(0.01, 0.5))
    }

    /// Perturb a mass value based on a percentage uncertainty.
    pub fn perturb_mass(&self, nominal: Mass, percent: f64) -> Mass {
        if percent == 0.0 {
            return nominal;
        }
        let factor = self.sample_factor(percent);
        Mass::kg(nominal.as_kg() * factor)
    }

    /// Create a perturbed copy of an engine.
    ///
    /// Perturbs ISP and thrust values while keeping other
    /// parameters (name, propellant, dry mass) unchanged.
    pub fn perturb_engine(&self, engine: &Engine) -> Engine {
        Engine::new(
            engine.name.clone(),
            self.perturb_thrust(engine.thrust_sl()),
            self.perturb_thrust(engine.thrust_vac()),
            self.perturb_isp(engine.isp_sl()),
            self.perturb_isp(engine.isp_vac()),
            engine.dry_mass(),
            engine.propellant,
        )
    }

    /// Sample a multiplicative factor from normal distribution.
    ///
    /// Returns a value centered on 1.0 with standard deviation
    /// equal to percent/100. For example, 2% uncertainty gives
    /// a normal distribution N(1.0, 0.02).
    fn sample_factor(&self, percent: f64) -> f64 {
        let mut rng = rand::thread_rng();
        let sigma = percent / 100.0;
        let normal = Normal::new(1.0, sigma).expect("invalid distribution parameters");
        normal.sample(&mut rng)
    }

    /// Sample a factor using a provided RNG (for reproducibility).
    pub fn sample_factor_with_rng<R: Rng>(&self, percent: f64, rng: &mut R) -> f64 {
        let sigma = percent / 100.0;
        let normal = Normal::new(1.0, sigma).expect("invalid distribution parameters");
        normal.sample(rng)
    }

    /// Get the underlying uncertainty specification.
    pub fn uncertainty(&self) -> &Uncertainty {
        &self.uncertainty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncertainty_default() {
        let u = Uncertainty::default();
        assert!((u.isp_percent - 1.0).abs() < 0.001);
        assert!((u.thrust_percent - 2.0).abs() < 0.001);
        assert!((u.structural_percent - 5.0).abs() < 0.001);
    }

    #[test]
    fn uncertainty_none() {
        let u = Uncertainty::none();
        assert!(u.is_zero());
    }

    #[test]
    fn uncertainty_custom() {
        let u = Uncertainty::new(2.0, 8.0, 3.0);
        assert!((u.isp_percent - 2.0).abs() < 0.001);
        assert!((u.structural_percent - 8.0).abs() < 0.001);
        assert!((u.thrust_percent - 3.0).abs() < 0.001);
    }

    #[test]
    fn sampler_perturb_isp() {
        let sampler = ParameterSampler::new(Uncertainty::default());
        let nominal = Isp::seconds(350.0);

        // Generate many samples
        let samples: Vec<f64> = (0..10000)
            .map(|_| sampler.perturb_isp(nominal).as_seconds())
            .collect();

        // Mean should be close to nominal
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(
            (mean - 350.0).abs() < 1.0,
            "mean {} too far from nominal 350",
            mean
        );

        // Standard deviation should be close to 1% of nominal
        let variance: f64 =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (samples.len() - 1) as f64;
        let std_dev = variance.sqrt();
        let expected_std = 350.0 * 0.01; // 1% of nominal
        assert!(
            (std_dev - expected_std).abs() < 1.0,
            "std_dev {} too far from expected {}",
            std_dev,
            expected_std
        );
    }

    #[test]
    fn sampler_perturb_thrust() {
        let sampler = ParameterSampler::new(Uncertainty::default());
        let nominal = Force::newtons(2_000_000.0);

        // Generate many samples
        let samples: Vec<f64> = (0..10000)
            .map(|_| sampler.perturb_thrust(nominal).as_newtons())
            .collect();

        // Mean should be close to nominal
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(
            (mean - 2_000_000.0).abs() < 10_000.0,
            "mean {} too far from nominal",
            mean
        );
    }

    #[test]
    fn sampler_perturb_structural_ratio() {
        let sampler = ParameterSampler::new(Uncertainty::default());
        let nominal = Ratio::new(0.08);

        // Generate many samples
        let samples: Vec<f64> = (0..10000)
            .map(|_| sampler.perturb_structural_ratio(nominal).as_f64())
            .collect();

        // Mean should be close to nominal
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(
            (mean - 0.08).abs() < 0.005,
            "mean {} too far from nominal 0.08",
            mean
        );

        // All values should be clamped to valid range
        assert!(samples.iter().all(|&x| x >= 0.01 && x <= 0.5));
    }

    #[test]
    fn sampler_zero_uncertainty_returns_nominal() {
        let sampler = ParameterSampler::new(Uncertainty::none());
        let nominal_isp = Isp::seconds(350.0);
        let nominal_thrust = Force::newtons(2_000_000.0);
        let nominal_ratio = Ratio::new(0.08);

        // With zero uncertainty, should return exactly nominal
        assert_eq!(
            sampler.perturb_isp(nominal_isp).as_seconds(),
            nominal_isp.as_seconds()
        );
        assert_eq!(
            sampler.perturb_thrust(nominal_thrust).as_newtons(),
            nominal_thrust.as_newtons()
        );
        assert_eq!(
            sampler.perturb_structural_ratio(nominal_ratio).as_f64(),
            nominal_ratio.as_f64()
        );
    }

    #[test]
    fn sampler_perturb_engine() {
        let sampler = ParameterSampler::new(Uncertainty::default());

        // Create a mock engine using the constructor
        use crate::engine::{Engine, Propellant};
        let engine = Engine::new(
            "Test",
            Force::newtons(1_000_000.0),
            Force::newtons(1_100_000.0),
            Isp::seconds(300.0),
            Isp::seconds(350.0),
            Mass::kg(1000.0),
            Propellant::LoxCh4,
        );

        let perturbed = sampler.perturb_engine(&engine);

        // Name and propellant should be unchanged
        assert_eq!(perturbed.name, engine.name);
        assert_eq!(perturbed.propellant, engine.propellant);
        assert_eq!(perturbed.dry_mass().as_kg(), engine.dry_mass().as_kg());

        // ISP and thrust should be different (almost certainly)
        // Note: There's a tiny chance they could be identical, but very unlikely
    }
}
