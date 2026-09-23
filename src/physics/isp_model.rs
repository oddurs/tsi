//! How specific impulse varies over a stage's burn.
//!
//! The theory is documented on the public type below, where rustdoc shows it.

use crate::units::Ratio;

/// Time-averaged ambient pressure, as a fraction of sea level, seen by a
/// first stage during a typical ascent to orbit.
///
/// See [`IspModel`] for the derivation.
pub const ASCENT_MEAN_PRESSURE_RATIO: f64 = 0.3;

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
/// Engines burn propellant at a nearly constant rate, so the Isp that goes into
/// the rocket equation is the time average of Isp over the burn. Isp is close
/// to linear in ambient pressure ([`Engine::isp_at`](crate::engine::Engine::isp_at)),
/// so what we need is the time average of the pressure ratio p/p₀.
///
/// Suppose the booster climbs to burnout altitude h_b along roughly
/// h(t) = h_b·(t/T)², with the atmosphere falling off with scale height
/// H ≈ 8 km. Then:
///
/// ```text
///  p     1            1                              √π
/// ⟨─⟩ = ─ ∫ e^(−h/H) dt = ∫ e^(−(h_b/H)·u²) du  ≈  ────────────
///  p₀   T              0                           2·√(h_b / H)
/// ```
///
/// With h_b ≈ 65 km this comes to 0.31, and the answer moves only slowly with
/// h_b (0.28 at 80 km, 0.35 at 50 km). tsi uses
/// [`ASCENT_MEAN_PRESSURE_RATIO`] = 0.3. For a Merlin-1D that gives 302 s,
/// close to the ~300 s usually quoted for Falcon 9's first stage.
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
/// assert!((ascent - 302.3).abs() < 0.1);
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

    #[test]
    fn derivation_matches_constant() {
        // Numerically integrate the model in the IspModel docs and check the
        // constant is a fair rounding of it.
        let h_b = 65.0;
        let scale_height = 8.0;
        let n = 10_000;
        let mean: f64 = (0..n)
            .map(|i| {
                let u = (i as f64 + 0.5) / n as f64;
                (-(h_b / scale_height) * u * u).exp()
            })
            .sum::<f64>()
            / n as f64;
        assert!((mean - ASCENT_MEAN_PRESSURE_RATIO).abs() < 0.02, "{mean}");
    }

    #[test]
    fn vacuum_sees_no_atmosphere() {
        assert_eq!(IspModel::Vacuum.mean_pressure_ratio().as_f64(), 0.0);
    }
}
