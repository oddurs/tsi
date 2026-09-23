//! Rocket engine representation with performance characteristics.
//!
//! This module defines the `Engine` struct which stores the key parameters
//! that define a rocket engine's performance.

use serde::{Deserialize, Serialize};

use crate::physics::IspModel;
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
/// # Validation
///
/// Every engine is checked when it is made, whether by [`Engine::new`] or by
/// loading a database: masses, thrusts and Isps must be positive and finite,
/// and an engine can't do better at sea level than in vacuum. An engine with
/// no sea-level rating (zero sea-level thrust and Isp) is an upper-stage
/// engine.
///
/// # Examples
///
/// ```
/// use tsiolkovsky::engine::{Engine, Propellant};
/// use tsiolkovsky::units::{Force, Isp, Mass};
///
/// let merlin = Engine::new(
///     "Merlin-1D",
///     Force::kilonewtons(845.0),   // Sea level thrust
///     Force::kilonewtons(914.0),   // Vacuum thrust
///     Isp::seconds(282.0),         // Sea level Isp
///     Isp::seconds(311.0),         // Vacuum Isp
///     Mass::kg(470.0),             // Engine dry mass
///     Propellant::LoxRp1,
/// )
/// .expect("a physically possible engine");
///
/// assert_eq!(merlin.isp_vac().as_seconds(), 311.0);
///
/// // An engine that beats its own vacuum Isp at sea level is rejected
/// let impossible = Engine::new(
///     "Perpetuum",
///     Force::kilonewtons(900.0),
///     Force::kilonewtons(900.0),
///     Isp::seconds(400.0),
///     Isp::seconds(300.0),
///     Mass::kg(500.0),
///     Propellant::LoxRp1,
/// );
/// assert!(impossible.is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EngineSpec", into = "EngineSpec")]
pub struct Engine {
    name: String,
    thrust_sl_n: f64,
    thrust_vac_n: f64,
    isp_sl_s: f64,
    isp_vac_s: f64,
    dry_mass_kg: f64,
    propellant: Propellant,
}

/// An engine as written in a TOML database or JSON: plain numbers in SI units.
#[derive(Serialize, Deserialize)]
struct EngineSpec {
    name: String,
    /// Sea level thrust in N (0 for vacuum-only engines)
    thrust_sl: f64,
    /// Vacuum thrust in N
    thrust_vac: f64,
    /// Sea level Isp in s (0 for vacuum-only engines)
    isp_sl: f64,
    /// Vacuum Isp in s
    isp_vac: f64,
    /// Dry mass in kg
    dry_mass: f64,
    propellant: Propellant,
}

impl TryFrom<EngineSpec> for Engine {
    type Error = EngineError;

    fn try_from(spec: EngineSpec) -> Result<Self, Self::Error> {
        Engine::new(
            spec.name,
            Force::newtons(spec.thrust_sl),
            Force::newtons(spec.thrust_vac),
            Isp::seconds(spec.isp_sl),
            Isp::seconds(spec.isp_vac),
            Mass::kg(spec.dry_mass),
            spec.propellant,
        )
    }
}

impl From<Engine> for EngineSpec {
    fn from(e: Engine) -> Self {
        EngineSpec {
            name: e.name,
            thrust_sl: e.thrust_sl_n,
            thrust_vac: e.thrust_vac_n,
            isp_sl: e.isp_sl_s,
            isp_vac: e.isp_vac_s,
            dry_mass: e.dry_mass_kg,
            propellant: e.propellant,
        }
    }
}

/// Why an engine's data is physically impossible.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum EngineError {
    /// The engine has no name.
    #[error("engine name is empty")]
    EmptyName,

    /// A quantity that must be positive and finite isn't.
    #[error("{engine}: {quantity} must be a positive number, got {value}")]
    NotPositive {
        engine: String,
        quantity: &'static str,
        value: f64,
    },

    /// A sea-level figure is negative or not a number.
    #[error("{engine}: {quantity} must be zero (no sea-level rating) or positive, got {value}")]
    InvalidSeaLevel {
        engine: String,
        quantity: &'static str,
        value: f64,
    },

    /// The engine does better at sea level than in vacuum.
    #[error(
        "{engine}: sea-level {quantity} ({sea_level}) exceeds vacuum ({vacuum}); \
        air pressure can only reduce an engine's performance"
    )]
    SeaLevelExceedsVacuum {
        engine: String,
        quantity: &'static str,
        sea_level: f64,
        vacuum: f64,
    },

    /// Only one of sea-level thrust and Isp is given.
    #[error(
        "{engine}: give both sea-level thrust and Isp, or neither (for a \
        vacuum-only engine)"
    )]
    PartialSeaLevelRating { engine: String },
}

impl Engine {
    /// Create a new engine, checking that its data is physically possible.
    ///
    /// For upper-stage-only engines (like RL-10), set sea level values to zero.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] for an empty name, a non-positive or
    /// non-finite vacuum thrust, vacuum Isp or dry mass, a negative sea-level
    /// value, sea-level performance better than vacuum, or a sea-level rating
    /// with only one of thrust and Isp.
    pub fn new(
        name: impl Into<String>,
        thrust_sl: Force,
        thrust_vac: Force,
        isp_sl: Isp,
        isp_vac: Isp,
        dry_mass: Mass,
        propellant: Propellant,
    ) -> Result<Self, EngineError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(EngineError::EmptyName);
        }
        let positive = |quantity: &'static str, value: f64| {
            if value.is_finite() && value > 0.0 {
                Ok(value)
            } else {
                Err(EngineError::NotPositive {
                    engine: name.clone(),
                    quantity,
                    value,
                })
            }
        };
        let sea_level = |quantity: &'static str, value: f64| {
            if value.is_finite() && value >= 0.0 {
                Ok(value)
            } else {
                Err(EngineError::InvalidSeaLevel {
                    engine: name.clone(),
                    quantity,
                    value,
                })
            }
        };
        let thrust_vac_n = positive("vacuum thrust", thrust_vac.as_newtons())?;
        let isp_vac_s = positive("vacuum Isp", isp_vac.as_seconds())?;
        let dry_mass_kg = positive("dry mass", dry_mass.as_kg())?;
        let thrust_sl_n = sea_level("sea-level thrust", thrust_sl.as_newtons())?;
        let isp_sl_s = sea_level("sea-level Isp", isp_sl.as_seconds())?;

        if (thrust_sl_n == 0.0) != (isp_sl_s == 0.0) {
            return Err(EngineError::PartialSeaLevelRating { engine: name });
        }
        for (quantity, sl, vac) in [
            ("thrust", thrust_sl_n, thrust_vac_n),
            ("Isp", isp_sl_s, isp_vac_s),
        ] {
            if sl > vac {
                return Err(EngineError::SeaLevelExceedsVacuum {
                    engine: name,
                    quantity,
                    sea_level: sl,
                    vacuum: vac,
                });
            }
        }

        Ok(Self {
            name,
            thrust_sl_n,
            thrust_vac_n,
            isp_sl_s,
            isp_vac_s,
            dry_mass_kg,
            propellant,
        })
    }

    /// The same engine with its Isp and thrust scaled, for Monte Carlo
    /// analysis. Sea-level and vacuum values scale together, so a valid engine
    /// stays valid for any positive factors.
    pub(crate) fn scaled(&self, isp_factor: f64, thrust_factor: f64) -> Engine {
        debug_assert!(isp_factor > 0.0 && thrust_factor > 0.0);
        Engine {
            name: self.name.clone(),
            thrust_sl_n: self.thrust_sl_n * thrust_factor,
            thrust_vac_n: self.thrust_vac_n * thrust_factor,
            isp_sl_s: self.isp_sl_s * isp_factor,
            isp_vac_s: self.isp_vac_s * isp_factor,
            dry_mass_kg: self.dry_mass_kg,
            propellant: self.propellant,
        }
    }

    /// Engine name, e.g. "Merlin-1D".
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Propellant combination this engine burns.
    pub fn propellant(&self) -> Propellant {
        self.propellant
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
    /// use tsiolkovsky::engine::EngineDatabase;
    /// use tsiolkovsky::units::Ratio;
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

    /// Effective Isp over a burn flown under the given [`IspModel`].
    ///
    /// Vacuum gives [`isp_vac`](Self::isp_vac). Ascent-averaged evaluates
    /// [`isp_at`](Self::isp_at) at the mean pressure a first stage sees on its
    /// way to orbit (see [`crate::physics::ASCENT_MEAN_PRESSURE_RATIO`]).
    ///
    /// An upper-stage-only engine (no sea-level rating) has no meaningful
    /// ascent-averaged Isp: its nozzle can't run at sea level at all. It gets
    /// zero, so a stage built around one delivers no delta-v rather than a
    /// plausible-looking number. The optimizers never put such an engine on
    /// an Earth first stage.
    pub fn isp_for(&self, model: IspModel) -> Isp {
        match model {
            IspModel::Vacuum => self.isp_vac(),
            IspModel::AscentAveraged if self.is_upper_stage_only() => Isp::seconds(0.0),
            IspModel::AscentAveraged => self.isp_at(model.mean_pressure_ratio()),
        }
    }

    /// The thrust that matters for TWR under the given [`IspModel`]: sea-level
    /// thrust for a first stage leaving Earth's surface, vacuum thrust
    /// otherwise.
    ///
    /// Only the liftoff moment is checked, and at liftoff an Earth-launched
    /// booster is pushing against a full atmosphere.
    pub fn thrust_for(&self, model: IspModel) -> Force {
        match model {
            IspModel::Vacuum => self.thrust_vac(),
            IspModel::AscentAveraged => self.thrust_sl(),
        }
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
        .unwrap()
    }

    #[test]
    fn engine_accessors() {
        let e = merlin_1d();
        assert_eq!(e.name(), "Merlin-1D");
        assert_eq!(e.thrust_sl().as_newtons(), 845_000.0);
        assert_eq!(e.thrust_vac().as_newtons(), 914_000.0);
        assert_eq!(e.isp_sl().as_seconds(), 282.0);
        assert_eq!(e.isp_vac().as_seconds(), 311.0);
        assert_eq!(e.dry_mass().as_kg(), 470.0);
        assert_eq!(e.propellant(), Propellant::LoxRp1);
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
        )
        .unwrap();
        assert!(rl10.is_upper_stage_only());
    }
}
