//! Rocket propellant types and their characteristics.
//!
//! Different propellant combinations offer different trade-offs between
//! performance (Isp), density, storability, and cost.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Propellant type used by a rocket engine.
///
/// Each propellant combination has distinct characteristics that affect
/// rocket design and performance.
///
/// # Comparison
///
/// | Type | Isp | Density | Notes |
/// |------|-----|---------|-------|
/// | LOX/RP-1 | Medium | High | Proven, cost-effective, dense |
/// | LOX/LH2 | Highest | Low | Best Isp, but very low density |
/// | LOX/CH4 | High | Medium | Good balance, reusability-friendly |
/// | N2O4/UDMH | Low | High | Storable, toxic, used in space |
/// | Solid | Low | Very High | Simple, high thrust, not restartable |
///
/// # Trade-offs
///
/// - **LOX/RP-1** (kerosene): High density means smaller tanks and better
///   mass ratio. Falcon 9 uses this. Moderate Isp (~310s vacuum).
///
/// - **LOX/LH2** (hydrogen): Highest Isp (~450s) but hydrogen's low density
///   requires huge tanks, hurting mass ratio. Used in upper stages (RS-25, RL-10).
///
/// - **LOX/CH4** (methane): Emerging favorite for reusable rockets. Good Isp
///   (~350s), doesn't coke like kerosene, easier to handle than hydrogen.
///   Used in Raptor, BE-4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Propellant {
    /// LOX/RP-1 (kerosene) - The workhorse propellant
    ///
    /// Used by: Falcon 9 (Merlin), Atlas V (RD-180), Saturn V first stage (F-1)
    LoxRp1,

    /// LOX/LH2 (liquid hydrogen) - Maximum efficiency
    ///
    /// Used by: Space Shuttle (RS-25), SLS, Centaur upper stage (RL-10)
    LoxLh2,

    /// LOX/CH4 (methane) - The reusability champion
    ///
    /// Used by: Starship (Raptor), New Glenn (BE-4)
    LoxCh4,

    /// N2O4/UDMH (hypergolic) - Storable but toxic
    ///
    /// Used by: Proton, some spacecraft thrusters. Ignites on contact.
    N2o4Udmh,

    /// Solid propellant - Simple and powerful
    ///
    /// Used by: Space Shuttle SRBs, Ariane 5 boosters, ICBMs
    Solid,
}

impl Propellant {
    /// Human-readable name for display.
    pub fn name(&self) -> &'static str {
        match self {
            Propellant::LoxRp1 => "LOX/RP-1",
            Propellant::LoxLh2 => "LOX/LH2",
            Propellant::LoxCh4 => "LOX/CH4",
            Propellant::N2o4Udmh => "N2O4/UDMH",
            Propellant::Solid => "Solid",
        }
    }

    /// Typical bulk density in kg/m³.
    ///
    /// This is an average considering both oxidizer and fuel mixed at
    /// typical ratios. Actual density varies with mixture ratio and temperature.
    ///
    /// Higher density means smaller tanks for the same propellant mass,
    /// which improves structural mass fraction.
    pub fn density(&self) -> f64 {
        match self {
            Propellant::LoxRp1 => 1030.0,   // Dense, good for first stages
            Propellant::LoxLh2 => 360.0,    // Very low, needs huge tanks
            Propellant::LoxCh4 => 830.0,    // Between kerosene and hydrogen
            Propellant::N2o4Udmh => 1180.0, // Dense storable
            Propellant::Solid => 1800.0,    // Highest density
        }
    }

    /// Check if this propellant matches a filter string (case-insensitive).
    ///
    /// Matches against:
    /// - Enum name (e.g., "loxch4")
    /// - Display name (e.g., "LOX/CH4")
    /// - Common aliases (e.g., "methane", "methalox")
    pub fn matches(&self, filter: &str) -> bool {
        let filter = filter.to_lowercase();
        let filter = filter.trim();

        // Match against enum name (lowercase)
        let enum_name = match self {
            Propellant::LoxRp1 => "loxrp1",
            Propellant::LoxLh2 => "loxlh2",
            Propellant::LoxCh4 => "loxch4",
            Propellant::N2o4Udmh => "n2o4udmh",
            Propellant::Solid => "solid",
        };
        if filter == enum_name {
            return true;
        }

        // Match against display name (lowercase)
        if self.name().to_lowercase() == filter {
            return true;
        }

        // Match against common aliases (what users might type)
        let aliases: &[&str] = match self {
            Propellant::LoxRp1 => &["kerosene", "rp1", "rp-1", "lox/rp1", "lox/rp-1"],
            Propellant::LoxLh2 => &["hydrogen", "lh2", "hydrolox", "lox/lh2"],
            Propellant::LoxCh4 => &["methane", "ch4", "methalox", "lox/ch4"],
            Propellant::N2o4Udmh => &["hypergolic", "udmh", "n2o4"],
            Propellant::Solid => &["srb"],
        };
        aliases.contains(&filter)
    }

    /// List all available propellant types.
    pub fn all() -> &'static [Propellant] {
        &[
            Propellant::LoxRp1,
            Propellant::LoxLh2,
            Propellant::LoxCh4,
            Propellant::N2o4Udmh,
            Propellant::Solid,
        ]
    }
}

impl fmt::Display for Propellant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn propellant_display() {
        assert_eq!(format!("{}", Propellant::LoxRp1), "LOX/RP-1");
        assert_eq!(format!("{}", Propellant::LoxCh4), "LOX/CH4");
    }

    #[test]
    fn propellant_density() {
        assert!(Propellant::LoxLh2.density() < Propellant::LoxRp1.density());
        assert!(Propellant::Solid.density() > Propellant::LoxRp1.density());
    }

    #[test]
    fn propellant_serialization() {
        let p = Propellant::LoxCh4;
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"LoxCh4\"");

        let parsed: Propellant = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, p);
    }
}
