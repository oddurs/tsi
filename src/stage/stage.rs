//! Rocket stage representation and performance calculations.
//!
//! A stage is a complete propulsion unit: engine(s), propellant tanks, and structure.
//! This module calculates stage performance including delta-v, TWR, and burn time.

use crate::engine::Engine;
use crate::physics::{burn_time, delta_v, twr};
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
/// The structural ratio (ε) relates structural mass to propellant mass:
///
/// ```text
/// ε = Structural Mass / Propellant Mass
/// ```
///
/// Typical values:
/// - 0.05-0.08: Excellent (modern composites, Falcon 9)
/// - 0.08-0.12: Good (traditional aluminum-lithium)
/// - 0.12-0.15: Acceptable (older designs, safety margins)
///
/// Lower is better: less structure per kg of propellant means better mass ratio.
///
/// # Examples
///
/// ```
/// use tsi::stage::Stage;
/// use tsi::engine::EngineDatabase;
/// use tsi::units::Mass;
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
/// );
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

impl Stage {
    /// Create a new stage with explicit structural mass.
    pub fn new(
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

    /// Create a stage with structural mass as a ratio of propellant mass.
    ///
    /// This is the common way to define stages when you know the structural
    /// efficiency but not the exact structural mass.
    ///
    /// # Arguments
    ///
    /// * `structural_ratio` - Structural mass / propellant mass (typically 0.05-0.15)
    pub fn with_structural_ratio(
        engine: Engine,
        engine_count: u32,
        propellant_mass: Mass,
        structural_ratio: f64,
    ) -> Self {
        let structural_mass = Mass::kg(propellant_mass.as_kg() * structural_ratio);
        Self::new(engine, engine_count, propellant_mass, structural_mass)
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
        let wet = self.wet_mass() + payload;
        let dry = self.dry_mass() + payload;
        let ratio = wet / dry;
        delta_v(self.isp_vac(), ratio)
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
        let stage = Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1);

        // Structural: 10,000 kg, Engine: 1,600 kg
        assert!((stage.dry_mass().as_kg() - 11_600.0).abs() < 1.0);
        assert!((stage.wet_mass().as_kg() - 111_600.0).abs() < 1.0);
    }

    #[test]
    fn stage_delta_v() {
        let stage = Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1);

        let dv = stage.delta_v();
        // ~7,771 m/s expected
        assert!(dv.as_mps() > 7_500.0);
        assert!(dv.as_mps() < 8_000.0);
    }

    #[test]
    fn stage_multiple_engines() {
        let stage = Stage::with_structural_ratio(get_merlin(), 9, Mass::kg(400_000.0), 0.1);

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
        let stage = Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1);

        let twr = stage.twr_vac();
        // ~2.24 expected
        assert!(twr.as_f64() > 2.0);
        assert!(twr.as_f64() < 2.5);
    }

    #[test]
    fn stage_with_payload() {
        let stage = Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1);

        let payload = Mass::kg(10_000.0);
        let dv_no_payload = stage.delta_v();
        let dv_with_payload = stage.delta_v_with_payload(payload);

        // Delta-v should be lower with payload
        assert!(dv_with_payload.as_mps() < dv_no_payload.as_mps());
    }

    #[test]
    fn stage_burn_time() {
        let stage = Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.1);

        let time = stage.burn_time();
        // ~140 seconds expected
        assert!(time.as_seconds() > 130.0);
        assert!(time.as_seconds() < 150.0);
    }
}
