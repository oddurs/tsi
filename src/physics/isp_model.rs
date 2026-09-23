//! How specific impulse varies over a stage's burn.
//!
//! The theory is documented on the public type below, where rustdoc shows it.

use crate::units::Ratio;

/// Ambient pressure, as a fraction of sea level, averaged over a first
/// stage's burn the way the rocket equation weights it (by 1/mass).
///
/// See [`IspModel`] for the derivation.
pub const ASCENT_MEAN_PRESSURE_RATIO: f64 = 0.2;

/// How a stage's specific impulse is evaluated over its burn.
///
/// # Why Isp depends on altitude
///
/// A nozzle turns hot, high-pressure gas into a fast, low-pressure jet. The
/// thrust it makes has two parts: the momentum of the exhaust, and the
/// difference between the pressure at the nozzle exit and the pressure of the
/// air around it:
///
/// ```text
/// F = ṁ·vₑ + (pₑ − pₐ)·Aₑ
/// ```
///
/// At sea level the ambient pressure pₐ pushes back on the exit plane and
/// subtracts from thrust. In vacuum pₐ = 0 and nothing is subtracted. Isp is
/// thrust per unit of propellant flow (F / ṁg₀), and the flow rate doesn't
/// change with altitude, so Isp goes up the same way thrust does. A Merlin-1D
/// makes 282 s at sea level and 311 s in vacuum.
///
/// Upper stages light above almost all of the atmosphere, so their vacuum Isp
/// is the right number. A first stage starts at sea level and ends 60-80 km up,
/// so the Isp it actually delivers is somewhere between the two.
///
/// # The ascent-averaged model
///
/// The rocket equation adds up velocity burn by burn: each kilogram of
/// propellant dm adds c·dm/m, where m is the mass at that moment. Written for
/// a whole burn,
///
/// ```text
/// Δv = ∫ c(t) · ṁ/m(t) dt
/// ```
///
/// so the effective exhaust velocity is an average of c weighted by 1/m, not
/// a plain time average. The last kilogram burned counts R times as much as
/// the first (R is the stage's mass ratio, including everything above it).
/// That favours the end of the burn, which happens high up in thin air.
///
/// Isp is close to linear in ambient pressure
/// ([`Engine::isp_at`](crate::engine::Engine::isp_at)), so what we need is the
/// same 1/m-weighted average of the pressure ratio p/p₀. Suppose the booster
/// burns at a constant rate, climbs to burnout altitude h_b along roughly
/// h(t) = h_b·(t/T)², and flies through an atmosphere with scale height
/// H ≈ 8 km. With u = t/T running from 0 to 1:
///
/// ```text
///          ∫ e^(−(h_b/H)·u²) · w(u) du                        1
/// ⟨p/p₀⟩ = ────────────────────────────     w(u) = ─────────────────────
///                ∫ w(u) du                         1 − (1 − 1/R)·u
/// ```
///
/// For h_b = 65 km and R = 3.5, typical of a first stage carrying its upper
/// stage (Falcon 9's is 3.6, Saturn V's S-IC 3.8), this gives 0.21. It stays
/// between 0.17 and 0.27 across burnout heights of 50-80 km and mass ratios
/// of 2.5-4.5. A plain time average would give 0.31, overweighting the dense
/// early air. tsi uses [`ASCENT_MEAN_PRESSURE_RATIO`] = 0.2. For a Merlin-1D
/// that gives 305 s, against 282 s at sea level and 311 s in vacuum.
///
/// This is a first-order model. It ignores throttling, the real shape of the
/// trajectory, and flow separation in over-expanded nozzles.
///
/// Upper stages always use [`IspModel::Vacuum`]. This choice only affects the
/// first stage, which is the one that flies through the atmosphere.
///
/// # Example
///
/// ```
/// use tsi::engine::EngineDatabase;
/// use tsi::physics::IspModel;
///
/// let db = EngineDatabase::load_embedded().expect("failed to load database");
/// let merlin = db.get("merlin-1d").expect("engine not found");
///
/// let vac = merlin.isp_for(IspModel::Vacuum).as_seconds();
/// let ascent = merlin.isp_for(IspModel::AscentAveraged).as_seconds();
///
/// assert_eq!(vac, 311.0);
/// assert!((ascent - 305.2).abs() < 0.1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IspModel {
    /// Vacuum Isp for the whole burn. Right for upper stages, and for first
    /// stages on airless bodies such as the Moon (or nearly airless ones such
    /// as Mars).
    Vacuum,

    /// Isp averaged over a nominal ascent from sea level, evaluated at
    /// [`ASCENT_MEAN_PRESSURE_RATIO`]. This is the default for Earth launches.
    AscentAveraged,
}

impl IspModel {
    /// Mean ambient pressure ratio (0 = vacuum, 1 = sea level) seen under this model.
    pub fn mean_pressure_ratio(self) -> Ratio {
        match self {
            IspModel::Vacuum => Ratio::new(0.0),
            IspModel::AscentAveraged => Ratio::new(ASCENT_MEAN_PRESSURE_RATIO),
        }
    }

    /// Short human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            IspModel::Vacuum => "vacuum",
            IspModel::AscentAveraged => "ascent-averaged",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 1/m-weighted mean pressure ratio from the model in the
    /// [`IspModel`] docs.
    fn weighted_mean_pressure(burnout_km: f64, mass_ratio: f64) -> f64 {
        let scale_height_km = 8.0;
        let n = 20_000;
        let (mut num, mut den) = (0.0, 0.0);
        for i in 0..n {
            let u = (i as f64 + 0.5) / n as f64;
            let w = 1.0 / (1.0 - (1.0 - 1.0 / mass_ratio) * u);
            num += (-(burnout_km / scale_height_km) * u * u).exp() * w;
            den += w;
        }
        num / den
    }

    #[test]
    fn derivation_matches_constant() {
        let mean = weighted_mean_pressure(65.0, 3.5);
        assert!((mean - 0.21).abs() < 0.005, "{mean}");
        assert!((mean - ASCENT_MEAN_PRESSURE_RATIO).abs() < 0.02, "{mean}");
    }

    #[test]
    fn range_quoted_in_docs_holds() {
        for h_b in [50.0, 65.0, 80.0] {
            for r in [2.5, 3.5, 4.5] {
                let mean = weighted_mean_pressure(h_b, r);
                assert!((0.165..0.275).contains(&mean), "h_b {h_b} R {r}: {mean}");
            }
        }
    }

    #[test]
    fn mass_weighting_lowers_the_mean() {
        // With R → 1 the weighting vanishes and we get the plain time average.
        let time_average = weighted_mean_pressure(65.0, 1.000_001);
        assert!((time_average - 0.31).abs() < 0.01, "{time_average}");
        assert!(weighted_mean_pressure(65.0, 3.5) < time_average);
    }

    #[test]
    fn vacuum_sees_no_atmosphere() {
        assert_eq!(IspModel::Vacuum.mean_pressure_ratio().as_f64(), 0.0);
    }
}
