//! Type-safe unit types for physical quantities.
//!
//! This module provides newtype wrappers for physical quantities used in
//! rocket calculations. Using distinct types prevents unit confusion errors
//! at compile time.
//!
//! # Available Types
//!
//! | Type | Purpose | Units |
//! |------|---------|-------|
//! | [`Mass`] | Rocket/propellant mass | kg, tonnes |
//! | [`Velocity`] | Delta-v, exhaust velocity | m/s, km/s |
//! | [`Force`] | Thrust | N, kN |
//! | [`Isp`] | Engine efficiency | seconds |
//! | [`Time`] | Burn duration | seconds, minutes |
//! | [`Ratio`] | Mass ratio, TWR | dimensionless |
//!
//! # Type Safety
//!
//! These types prevent common errors like adding mass to velocity:
//!
//! ```compile_fail
//! use tsi::units::{Mass, Velocity};
//!
//! let mass = Mass::kg(1000.0);
//! let velocity = Velocity::mps(3000.0);
//! let wrong = mass + velocity; // Compile error!
//! ```
//!
//! But meaningful operations are allowed:
//!
//! ```
//! use tsi::units::Mass;
//!
//! let wet = Mass::kg(10000.0);
//! let dry = Mass::kg(2000.0);
//! let ratio = wet / dry; // Returns Ratio
//! assert!((ratio.as_f64() - 5.0).abs() < 0.001);
//! ```

mod fmt;
mod force;
mod isp;
mod mass;
mod ratio;
mod time;
mod velocity;

pub use fmt::{format_thousands, format_thousands_f64};

pub use force::Force;
pub use isp::Isp;
pub use mass::Mass;
pub use ratio::Ratio;
pub use time::Time;
pub use velocity::Velocity;
