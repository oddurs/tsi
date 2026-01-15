//! Physics calculations including Tsiolkovsky equation.

mod thrust;
mod tsiolkovsky;

pub use thrust::{burn_time, twr};
pub use tsiolkovsky::{delta_v, required_mass_ratio};

/// Standard gravity (m/s²)
pub const G0: f64 = 9.80665;
