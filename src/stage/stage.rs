//! Rocket stage representation and performance calculations.
//!
//! A stage is a complete propulsion unit: engine(s), propellant tanks, and structure.
//! This module calculates stage performance including delta-v, TWR, and burn time.

use crate::engine::Engine;
use crate::physics::{burn_time, delta_v, twr, IspModel};
use crate::units::{Force, Isp, Mass, Ratio, Time, Velocity};

/// A rocket stage with engine(s), propellant, and structure.
///
/// Stages are the building blocks of rockets. Each stage is designed to
/// operate in a specific flight regime:
///
/// - **First stages**: High thrust, often multiple engines, designed for liftoff
/// - **Upper stages**: Optimized for vacuum, often single engine, high Isp
///
/// # Mass Breakdown
///
/// A stage's mass is composed of:
///
/// ```text
/// Wet Mass = Propellant + Structural Mass + Engine Mass
/// Dry Mass = Structural Mass + Engine Mass
/// Mass Ratio = Wet Mass / Dry Mass
/// ```
///
/// The mass ratio directly determines delta-v through the rocket equation.
///
/// # Structural Ratio
///
/// tsi describes a stage's structure with its **structural ratio**: the mass
/// of everything that isn't propellant or engines (tanks, interstage,
/// plumbing, avionics) per kilogram of propellant:
///
/// ```text
/// structural ratio = structural mass / propellant mass
/// ```
///
/// Engines are left out because tsi counts them separately, engine by engine:
/// their mass is fixed, not proportional to the propellant, and the optimizer
/// needs to see that.
///
/// Textbooks usually quote the **structural coefficient** ε instead, which is
/// the whole dry mass (engines included) over dry plus propellant:
///
/// ```text
/// ε = dry mass / (dry mass + propellant mass)
/// ```
///
/// [`Stage::structural_coefficient`] gives ε for any stage. For a stage whose
/// engines weigh E, the two are related by
/// ε = (ratio · m_p + E) / ((1 + ratio) · m_p + E).
///
/// ## Real stages
///
/// | Stage | Propellant | Structural ratio | ε (textbook) |
/// |-------|-----------|------------------|--------------|
/// | Falcon 9 first stage | 411 t | 4.4% | 5.1% |
/// | Falcon 9 second stage | 111.5 t | 3.2% | 3.5% |
/// | Saturn V S-IC | 2,160 t | 4.1% | 5.7% |
/// | Saturn V S-II | 443 t | 6.1% | 7.5% |
/// | Saturn V S-IVB | 107 t | 10.9% | 11.2% |
///
/// Computed from published stage masses (SpaceX Falcon 9 specifications;
/// NASA SP-4206, *Stages to Saturn*, appendix) with tsi's engine masses taken
/// out. Hydrogen stages need big, insulated tanks for their low-density
/// propellant, which is why the S-II and S-IVB are structurally heavier.
///
/// Lower is better: less structure per kg of propellant means a better mass
/// ratio.
///
/// # Examples
///
/// ```
/// use tsiolkovsky::stage::Stage;
/// use tsiolkovsky::engine::EngineDatabase;
/// use tsiolkovsky::units::Mass;
///
/// let db = EngineDatabase::load_embedded().expect("failed to load database");
/// let raptor = db.get("raptor-2").expect("engine not found");
///
/// // Create a stage with 100 tonnes propellant, 10% structural ratio
/// let stage = Stage::with_structural_ratio(
///     raptor.clone(),
///     1,                      // Single engine
///     Mass::kg(100_000.0),    // 100 tonnes propellant
///     0.10,                   // 10% structural ratio
/// )
/// .expect("a valid stage");
///
/// println!("Delta-v: {}", stage.delta_v());
/// println!("Burn time: {}", stage.burn_time());
/// println!("TWR: {:.2}", stage.twr_vac().as_f64());
/// ```
#[derive(Debug, Clone)]
pub struct Stage {
    /// The engine type used by this stage
    engine: Engine,
    /// Number of engines (e.g., 9 for Falcon 9 first stage)
    engine_count: u32,
    /// Mass of propellant loaded
    propellant_mass: Mass,
    /// Structural mass (tanks, interstage, plumbing - excludes engines)
    structural_mass: Mass,
}

/// Why a stage's numbers can't describe a real stage.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum StageError {
    /// A stage needs at least one engine.
    #[error("a stage needs at least one engine")]
    NoEngines,

    /// Propellant mass must be positive and finite.
    #[error("propellant mass must be a positive number, got {0}")]
    InvalidPropellant(Mass),

    /// Structural mass must be zero or positive, and finite.
    #[error("structural mass must be zero or a positive number, got {0}")]
    InvalidStructuralMass(Mass),

    /// Structural ratio must be zero or positive, and finite.
    #[error("structural ratio must be zero or a positive number, got {0}")]
    InvalidStructuralRatio(f64),
}

impl Stage {
    /// Create a new stage with explicit structural mass.
    ///
    /// # Errors
    ///
    /// [`StageError`] for zero engines, a propellant mass that isn't positive,
    /// or a negative or non-finite structural mass.
    pub fn new(
        engine: Engine,
        engine_count: u32,
        propellant_mass: Mass,
        structural_mass: Mass,
    ) -> Result<Self, StageError> {
        if engine_count == 0 {
            return Err(StageError::NoEngines);
        }
        let p = propellant_mass.as_kg();
        if !(p.is_finite() && p > 0.0) {
            return Err(StageError::InvalidPropellant(propellant_mass));
        }
        let s = structural_mass.as_kg();
        if !(s.is_finite() && s >= 0.0) {
            return Err(StageError::InvalidStructuralMass(structural_mass));
        }
        Ok(Self::from_parts(
            engine,
            engine_count,
            propellant_mass,
            structural_mass,
        ))
    }

    /// Create a stage with structural mass as a ratio of propellant mass.
    ///
    /// This is the common way to define stages when you know the structural
    /// efficiency but not the exact structural mass.
    ///
    /// # Arguments
    ///
    /// * `structural_ratio` - Structural mass / propellant mass, engines
    ///   excluded (typically 0.03-0.11; see the table above)
    ///
    /// # Errors
    ///
    /// As [`Stage::new`], or [`StageError::InvalidStructuralRatio`].
    pub fn with_structural_ratio(
        engine: Engine,
        engine_count: u32,
        propellant_mass: Mass,
        structural_ratio: f64,
    ) -> Result<Self, StageError> {
        if !(structural_ratio.is_finite() && structural_ratio >= 0.0) {
            return Err(StageError::InvalidStructuralRatio(structural_ratio));
        }
        let structural_mass = Mass::kg(propellant_mass.as_kg() * structural_ratio);
        Self::new(engine, engine_count, propellant_mass, structural_mass)
    }

    /// Build a stage from numbers already known to be valid.
    pub(crate) fn from_parts(
        engine: Engine,
        engine_count: u32,
        propellant_mass: Mass,
        structural_mass: Mass,
    ) -> Self {
        Self {
            engine,
            engine_count,
            propellant_mass,
            structural_mass,
        }
    }

    /// Structural ratio: structural mass (engines excluded) / propellant mass.
    pub fn structural_ratio(&self) -> f64 {
        self.structural_mass.as_kg() / self.propellant_mass.as_kg()
    }

    /// The textbook structural coefficient ε = dry / (dry + propellant),
    /// engines included.
    ///
    /// ```
    /// use tsiolkovsky::engine::EngineDatabase;
    /// use tsiolkovsky::stage::Stage;
    /// use tsiolkovsky::units::Mass;
    ///
    /// let db = EngineDatabase::load_embedded().unwrap();
    /// let merlin = db.get("merlin-1d").unwrap().clone();
    ///
    /// // Falcon 9's first stage: 22.2 t dry, of which 4.2 t is engines
    /// let s1 = Stage::new(merlin, 9, Mass::kg(411_000.0), Mass::kg(17_970.0)).unwrap();
    /// assert!((s1.structural_ratio() - 0.044).abs() < 0.001);
    /// assert!((s1.structural_coefficient() - 0.051).abs() < 0.001);
    /// ```
    pub fn structural_coefficient(&self) -> f64 {
        let dry = self.dry_mass().as_kg();
        dry / (dry + self.propellant_mass.as_kg())
    }

    /// Get the engine used by this stage.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Get the number of engines.
    pub fn engine_count(&self) -> u32 {
        self.engine_count
    }

    /// Get the propellant mass.
    pub fn propellant_mass(&self) -> Mass {
        self.propellant_mass
    }

    /// Get the structural mass (tanks, interstage, etc. excluding engines).
    pub fn structural_mass(&self) -> Mass {
        self.structural_mass
    }

    /// Total mass of all engines on this stage.
    pub fn engine_mass(&self) -> Mass {
        self.engine.dry_mass() * self.engine_count
    }

    /// Dry mass: structural mass + engine mass.
    ///
    /// This is the mass remaining after propellant is exhausted.
    pub fn dry_mass(&self) -> Mass {
        self.structural_mass + self.engine_mass()
    }

    /// Wet mass: dry mass + propellant.
    ///
    /// This is the total stage mass at ignition.
    pub fn wet_mass(&self) -> Mass {
        self.dry_mass() + self.propellant_mass
    }

    /// Mass ratio (wet/dry) - the key input to the rocket equation.
    ///
    /// Higher mass ratio = more delta-v. A ratio of 10 means the stage
    /// is 90% propellant by mass.
    pub fn mass_ratio(&self) -> Ratio {
        self.wet_mass() / self.dry_mass()
    }

    /// Total vacuum thrust from all engines.
    pub fn thrust_vac(&self) -> Force {
        self.engine.thrust_vac() * self.engine_count
    }

    /// Total sea-level thrust from all engines.
    pub fn thrust_sl(&self) -> Force {
        self.engine.thrust_sl() * self.engine_count
    }

    /// Vacuum Isp (same regardless of engine count).
    ///
    /// Isp doesn't change with multiple engines - it's a property of
    /// the engine design, not the number of engines.
    pub fn isp_vac(&self) -> Isp {
        self.engine.isp_vac()
    }

    /// Delta-v of this stage in vacuum (no payload).
    ///
    /// This is the maximum velocity change this stage can produce
    /// when carrying no additional mass on top.
    pub fn delta_v(&self) -> Velocity {
        delta_v(self.isp_vac(), self.mass_ratio())
    }

    /// Delta-v when carrying additional payload mass.
    ///
    /// The payload "eats into" the mass ratio, reducing available delta-v.
    /// This is why upper stages want to be as light as possible.
    pub fn delta_v_with_payload(&self, payload: Mass) -> Velocity {
        self.delta_v_with(payload, IspModel::Vacuum)
    }

    /// Delta-v carrying `payload`, with Isp evaluated under `isp_model`.
    ///
    /// A first stage flown from sea level delivers less than its vacuum
    /// delta-v because it spends the first part of its burn in thick air.
    /// See [`IspModel`] for how much less, and why.
    pub fn delta_v_with(&self, payload: Mass, isp_model: IspModel) -> Velocity {
        let wet = self.wet_mass() + payload;
        let dry = self.dry_mass() + payload;
        delta_v(self.engine.isp_for(isp_model), wet / dry)
    }

    /// Thrust-to-weight ratio at ignition in vacuum.
    ///
    /// Calculated at full propellant load (worst case for TWR).
    pub fn twr_vac(&self) -> Ratio {
        twr(self.thrust_vac(), self.wet_mass(), crate::physics::G0)
    }

    /// TWR at ignition in vacuum with additional payload.
    pub fn twr_vac_with_payload(&self, payload: Mass) -> Ratio {
        let total_mass = self.wet_mass() + payload;
        twr(self.thrust_vac(), total_mass, crate::physics::G0)
    }

    /// Thrust-to-weight ratio at ignition at sea level.
    ///
    /// Relevant for first stages that must lift off from Earth's surface.
    pub fn twr_sl(&self) -> Ratio {
        twr(self.thrust_sl(), self.wet_mass(), crate::physics::G0)
    }

    /// TWR at ignition at sea level with additional payload.
    pub fn twr_sl_with_payload(&self, payload: Mass) -> Ratio {
        let total_mass = self.wet_mass() + payload;
        twr(self.thrust_sl(), total_mass, crate::physics::G0)
    }

    /// Time to consume all propellant at full thrust.
    ///
    /// Assumes constant thrust and complete propellant consumption.
    /// Real rockets may throttle or retain reserves.
    pub fn burn_time(&self) -> Time {
        burn_time(self.propellant_mass, self.thrust_vac(), self.isp_vac())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;

    fn get_raptor() -> Engine {
        let db = EngineDatabase::default();
        db.get("Raptor-2").unwrap().clone()
    }

    fn get_merlin() -> Engine {
        let db = EngineDatabase::default();
        db.get("Merlin-1D").unwrap().clone()
    }

    #[test]
    fn stage_mass_calculations() {
        let stage =
            Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1).unwrap();

        // Structural: 10,000 kg, Engine: 1,600 kg
        assert!((stage.dry_mass().as_kg() - 11_600.0).abs() < 1.0);
        assert!((stage.wet_mass().as_kg() - 111_600.0).abs() < 1.0);
    }

    #[test]
    fn stage_delta_v() {
        let stage =
            Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1).unwrap();

        let dv = stage.delta_v();
        // ~7,771 m/s expected
        assert!(dv.as_mps() > 7_500.0);
        assert!(dv.as_mps() < 8_000.0);
    }

    #[test]
    fn stage_multiple_engines() {
        let stage =
            Stage::with_structural_ratio(get_merlin(), 9, Mass::kg(400_000.0), 0.1).unwrap();

        // 9 engines = 9 * 470 kg = 4,230 kg
        // Structural = 40,000 kg
        // Dry = 44,230 kg
        assert!((stage.engine_mass().as_kg() - 4_230.0).abs() < 1.0);
        assert!((stage.dry_mass().as_kg() - 44_230.0).abs() < 1.0);

        // Thrust = 9 * 914 kN = 8,226 kN
        assert!((stage.thrust_vac().as_kilonewtons() - 8_226.0).abs() < 1.0);
    }

    #[test]
    fn stage_twr() {
        let stage =
            Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1).unwrap();

        let twr = stage.twr_vac();
        // ~2.24 expected
        assert!(twr.as_f64() > 2.0);
        assert!(twr.as_f64() < 2.5);
    }

    #[test]
    fn stage_with_payload() {
        let stage =
            Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1).unwrap();

        let payload = Mass::kg(10_000.0);
        let dv_no_payload = stage.delta_v();
        let dv_with_payload = stage.delta_v_with_payload(payload);

        // Delta-v should be lower with payload
        assert!(dv_with_payload.as_mps() < dv_no_payload.as_mps());
    }

    #[test]
    fn stage_burn_time() {
        let stage =
            Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1).unwrap();

        let time = stage.burn_time();
        // ~140 seconds expected
        assert!(time.as_seconds() > 130.0);
        assert!(time.as_seconds() < 150.0);
    }
}
