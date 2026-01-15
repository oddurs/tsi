//! Core physics calculations for rocket performance analysis.
//!
//! This module provides the fundamental equations of astronautics:
//!
//! - [`delta_v`] - The Tsiolkovsky rocket equation
//! - [`required_mass_ratio`] - Inverse of the rocket equation
//! - [`twr`] - Thrust-to-weight ratio calculation
//! - [`burn_time`] - Engine burn duration
//!
//! # Constants
//!
//! - [`G0`] - Standard gravity (9.80665 m/s²), used to convert between
//!   Isp (seconds) and exhaust velocity (m/s).
//!
//! # Example: Analyzing a Rocket Stage
//!
//! ```
//! use tsi::physics::{delta_v, twr, burn_time, G0};
//! use tsi::units::{Mass, Force, Isp, Ratio};
//!
//! // Define a stage: 100 tonnes propellant, 10 tonnes dry mass
//! let propellant = Mass::kg(100_000.0);
//! let dry_mass = Mass::kg(10_000.0);
//! let wet_mass = propellant + dry_mass;
//! let mass_ratio = wet_mass / dry_mass;
//!
//! // Engine: 2,000 kN thrust, 350s Isp
//! let thrust = Force::kilonewtons(2_000.0);
//! let isp = Isp::seconds(350.0);
//!
//! // Calculate performance
//! let dv = delta_v(isp, mass_ratio);
//! let initial_twr = twr(thrust, wet_mass, G0);
//! let burn = burn_time(propellant, thrust, isp);
//!
//! println!("Delta-v: {}", dv);           // ~7,900 m/s
//! println!("Initial TWR: {:.2}", initial_twr.as_f64()); // ~1.82
//! println!("Burn time: {}", burn);       // ~2m 51s
//! ```

mod thrust;
mod tsiolkovsky;

pub use thrust::{burn_time, twr};
pub use tsiolkovsky::{delta_v, required_mass_ratio};

/// Standard gravitational acceleration at Earth's surface.
///
/// This value (9.80665 m/s²) is used as the reference for specific impulse
/// calculations. Isp in seconds multiplied by g₀ gives exhaust velocity in m/s.
///
/// # Historical Note
///
/// This specific value was adopted as the standard in 1901 by the General
/// Conference on Weights and Measures. It represents the gravitational
/// acceleration at sea level at 45° latitude.
pub const G0: f64 = 9.80665;
