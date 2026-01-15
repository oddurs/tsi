//! Rocket engine representation with performance characteristics.
//!
//! This module defines the `Engine` struct which stores the key parameters
//! that define a rocket engine's performance.

use serde::{Deserialize, Serialize};

use crate::units::{Force, Isp, Mass, Ratio};

use super::Propellant;

/// A rocket engine with performance characteristics.
///
/// Engines are the heart of any rocket. Their key parameters determine
/// what the rocket can achieve:
///
/// - **Thrust**: How much force the engine produces (affects TWR)
/// - **Isp**: How efficiently propellant is used (affects delta-v)
/// - **Mass**: How heavy the engine is (affects mass ratio)
/// - **Propellant**: What fuel/oxidizer combination is used
///
/// # Sea Level vs Vacuum Performance
///
/// Most parameters vary with atmospheric pressure:
///
/// - At sea level, atmospheric back-pressure reduces effective exhaust velocity
/// - In vacuum, exhaust can expand fully, improving performance
///
/// Example: Merlin-1D produces 845 kN / 282s Isp at sea level,
/// but 914 kN / 311s Isp in vacuum - about 8-10% improvement.
///
/// # Examples
///
/// ```
/// use tsi::engine::{Engine, Propellant};
/// use tsi::units::{Force, Isp, Mass};
///
/// let merlin = Engine::new(
///     "Merlin-1D",
///     Force::kilonewtons(845.0),   // Sea level thrust
///     Force::kilonewtons(914.0),   // Vacuum thrust
///     Isp::seconds(282.0),         // Sea level Isp
///     Isp::seconds(311.0),         // Vacuum Isp
///     Mass::kg(470.0),             // Engine dry mass
///     Propellant::LoxRp1,
/// );
///
/// assert_eq!(merlin.isp_vac().as_seconds(), 311.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Engine {
    /// Engine name (e.g., "Merlin-1D", "Raptor-2")
    pub name: String,

    /// Sea level thrust in Newtons (stored as raw f64 for serde)
    #[serde(rename = "thrust_sl")]
    thrust_sl_n: f64,

    /// Vacuum thrust in Newtons
    #[serde(rename = "thrust_vac")]
    thrust_vac_n: f64,

    /// Sea level specific impulse in seconds
    #[serde(rename = "isp_sl")]
    isp_sl_s: f64,

    /// Vacuum specific impulse in seconds
    #[serde(rename = "isp_vac")]
    isp_vac_s: f64,

    /// Dry mass of the engine in kg
    #[serde(rename = "dry_mass")]
    dry_mass_kg: f64,

    /// Propellant type used by this engine
    pub propellant: Propellant,
}

impl Engine {
    /// Create a new engine with the given parameters.
    ///
    /// For upper-stage-only engines (like RL-10), set sea level values to zero.
    pub fn new(
        name: impl Into<String>,
        thrust_sl: Force,
        thrust_vac: Force,
        isp_sl: Isp,
        isp_vac: Isp,
        dry_mass: Mass,
        propellant: Propellant,
    ) -> Self {
        Self {
            name: name.into(),
            thrust_sl_n: thrust_sl.as_newtons(),
            thrust_vac_n: thrust_vac.as_newtons(),
            isp_sl_s: isp_sl.as_seconds(),
            isp_vac_s: isp_vac.as_seconds(),
            dry_mass_kg: dry_mass.as_kg(),
            propellant,
        }
    }

    /// Sea level thrust.
    ///
    /// Returns zero for vacuum-only engines like RL-10.
    pub fn thrust_sl(&self) -> Force {
        Force::newtons(self.thrust_sl_n)
    }

    /// Vacuum thrust - typically 5-10% higher than sea level.
    pub fn thrust_vac(&self) -> Force {
        Force::newtons(self.thrust_vac_n)
    }

    /// Sea level specific impulse.
    ///
    /// Returns zero for vacuum-only engines.
    pub fn isp_sl(&self) -> Isp {
        Isp::seconds(self.isp_sl_s)
    }

    /// Vacuum specific impulse - the "headline" Isp number.
    ///
    /// This is the efficiency you get in space where most delta-v is produced.
    pub fn isp_vac(&self) -> Isp {
        Isp::seconds(self.isp_vac_s)
    }

    /// Dry mass of the engine (without propellant).
    ///
    /// Multiple engines multiply this: 9 Merlins = 9 × 470 kg = 4,230 kg.
    pub fn dry_mass(&self) -> Mass {
        Mass::kg(self.dry_mass_kg)
    }

    /// Interpolate Isp at a given atmospheric pressure ratio.
    ///
    /// This provides a simple linear interpolation between sea level and vacuum.
    /// Real nozzle performance is more complex, but this is a reasonable
    /// approximation for trajectory analysis.
    ///
    /// # Arguments
    ///
    /// * `pressure_ratio` - 0.0 = vacuum, 1.0 = sea level (1 atm)
    ///
    /// # Example
    ///
    /// ```
    /// use tsi::engine::EngineDatabase;
    /// use tsi::units::Ratio;
    ///
    /// let db = EngineDatabase::load_embedded().expect("failed to load database");
    /// let merlin = db.get("merlin-1d").expect("engine not found");
    ///
    /// // At 50% sea level pressure (roughly 5 km altitude)
    /// let mid_isp = merlin.isp_at(Ratio::new(0.5));
    /// assert!(mid_isp.as_seconds() > 282.0); // Better than sea level
    /// assert!(mid_isp.as_seconds() < 311.0); // Not as good as vacuum
    /// ```
    pub fn isp_at(&self, pressure_ratio: Ratio) -> Isp {
        let p = pressure_ratio.as_f64().clamp(0.0, 1.0);
        // Linear interpolation: vacuum + p × (sea_level - vacuum)
        // At p=0 (vacuum): isp_vac
        // At p=1 (sea level): isp_sl
        let isp = self.isp_vac_s + p * (self.isp_sl_s - self.isp_vac_s);
        Isp::seconds(isp)
    }

    /// Interpolate thrust at a given atmospheric pressure ratio.
    ///
    /// Similar to [`isp_at`](Self::isp_at), provides linear interpolation.
    ///
    /// # Arguments
    ///
    /// * `pressure_ratio` - 0.0 = vacuum, 1.0 = sea level
    pub fn thrust_at(&self, pressure_ratio: Ratio) -> Force {
        let p = pressure_ratio.as_f64().clamp(0.0, 1.0);
        let thrust = self.thrust_vac_n + p * (self.thrust_sl_n - self.thrust_vac_n);
        Force::newtons(thrust)
    }

    /// Check if this is an upper-stage-only engine.
    ///
    /// Upper stage engines (like RL-10, Merlin Vacuum) have vacuum-optimized
    /// nozzles that would be damaged or perform poorly at sea level.
    /// They have no sea level performance data.
    ///
    /// These engines cannot be used for first stages or booster applications.
    pub fn is_upper_stage_only(&self) -> bool {
        self.thrust_sl_n == 0.0 || self.isp_sl_s == 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merlin_1d() -> Engine {
        Engine::new(
            "Merlin-1D",
            Force::newtons(845_000.0),
            Force::newtons(914_000.0),
            Isp::seconds(282.0),
            Isp::seconds(311.0),
            Mass::kg(470.0),
            Propellant::LoxRp1,
        )
    }

    #[test]
    fn engine_accessors() {
        let e = merlin_1d();
        assert_eq!(e.name, "Merlin-1D");
        assert_eq!(e.thrust_sl().as_newtons(), 845_000.0);
        assert_eq!(e.thrust_vac().as_newtons(), 914_000.0);
        assert_eq!(e.isp_sl().as_seconds(), 282.0);
        assert_eq!(e.isp_vac().as_seconds(), 311.0);
        assert_eq!(e.dry_mass().as_kg(), 470.0);
        assert_eq!(e.propellant, Propellant::LoxRp1);
    }

    #[test]
    fn isp_interpolation() {
        let e = merlin_1d();

        // Vacuum
        assert_eq!(e.isp_at(Ratio::new(0.0)).as_seconds(), 311.0);

        // Sea level
        assert_eq!(e.isp_at(Ratio::new(1.0)).as_seconds(), 282.0);

        // Mid-point
        let mid = e.isp_at(Ratio::new(0.5)).as_seconds();
        assert!((mid - 296.5).abs() < 0.1);
    }

    #[test]
    fn thrust_interpolation() {
        let e = merlin_1d();

        // Vacuum
        assert_eq!(e.thrust_at(Ratio::new(0.0)).as_newtons(), 914_000.0);

        // Sea level
        assert_eq!(e.thrust_at(Ratio::new(1.0)).as_newtons(), 845_000.0);
    }

    #[test]
    fn upper_stage_detection() {
        let e = merlin_1d();
        assert!(!e.is_upper_stage_only());

        let rl10 = Engine::new(
            "RL-10C",
            Force::newtons(0.0),
            Force::newtons(106_000.0),
            Isp::seconds(0.0),
            Isp::seconds(453.0),
            Mass::kg(190.0),
            Propellant::LoxLh2,
        );
        assert!(rl10.is_upper_stage_only());
    }
}
