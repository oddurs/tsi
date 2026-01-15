use crate::units::{Force, Isp, Mass, Ratio, Time};

use super::G0;

/// Calculate thrust-to-weight ratio.
pub fn twr(thrust: Force, mass: Mass, gravity: f64) -> Ratio {
    Ratio::new(thrust.as_newtons() / (mass.as_kg() * gravity))
}

/// Calculate burn time from propellant mass, thrust, and Isp.
///
/// burn_time = propellant_mass / mass_flow_rate
/// mass_flow_rate = thrust / (Isp × g₀)
pub fn burn_time(propellant: Mass, thrust: Force, isp: Isp) -> Time {
    let mass_flow = thrust.as_newtons() / (isp.as_seconds() * G0);
    Time::seconds(propellant.as_kg() / mass_flow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn twr_calculation() {
        // 1,000,000 N thrust, 100,000 kg mass, Earth gravity
        let thrust = Force::newtons(1_000_000.0);
        let mass = Mass::kg(100_000.0);
        let ratio = twr(thrust, mass, G0);
        assert_relative_eq!(ratio.as_f64(), 1.0197, epsilon = 0.001);
    }

    #[test]
    fn twr_exactly_one() {
        // Thrust equals weight
        let mass = Mass::kg(1000.0);
        let weight = 1000.0 * G0;
        let thrust = Force::newtons(weight);
        let ratio = twr(thrust, mass, G0);
        assert_relative_eq!(ratio.as_f64(), 1.0, epsilon = 0.0001);
    }

    #[test]
    fn twr_high_thrust() {
        // 9 × Merlin-1D at liftoff
        let thrust = Force::newtons(845_000.0 * 9.0); // 7.6 MN
        let mass = Mass::kg(550_000.0); // Falcon 9 liftoff mass
        let ratio = twr(thrust, mass, G0);
        // Should be around 1.4
        assert!(ratio.as_f64() > 1.3);
        assert!(ratio.as_f64() < 1.5);
    }

    #[test]
    fn burn_time_calculation() {
        // 10,000 kg propellant, 100,000 N thrust, 300s Isp
        // mass_flow = 100,000 / (300 × 9.80665) = 33.99 kg/s
        // burn_time = 10,000 / 33.99 = 294.2 s
        let propellant = Mass::kg(10_000.0);
        let thrust = Force::newtons(100_000.0);
        let isp = Isp::seconds(300.0);
        let time = burn_time(propellant, thrust, isp);
        assert_relative_eq!(time.as_seconds(), 294.2, epsilon = 0.5);
    }

    #[test]
    fn burn_time_merlin() {
        // Single Merlin-1D: 845 kN thrust, 311s Isp (vac)
        // With 45,000 kg propellant
        let propellant = Mass::kg(45_000.0);
        let thrust = Force::newtons(845_000.0);
        let isp = Isp::seconds(311.0);
        let time = burn_time(propellant, thrust, isp);

        // mass_flow = 845000 / (311 × 9.80665) ≈ 277 kg/s
        // burn_time = 45000 / 277 ≈ 162 s
        assert_relative_eq!(time.as_seconds(), 162.0, epsilon = 2.0);
    }

    #[test]
    fn burn_time_high_isp() {
        // RL-10C: 106 kN thrust, 453s Isp
        let propellant = Mass::kg(20_000.0);
        let thrust = Force::newtons(106_000.0);
        let isp = Isp::seconds(453.0);
        let time = burn_time(propellant, thrust, isp);

        // Higher Isp means lower mass flow, longer burn
        // mass_flow = 106000 / (453 × 9.80665) ≈ 23.9 kg/s
        // burn_time = 20000 / 23.9 ≈ 838 s
        assert_relative_eq!(time.as_seconds(), 838.0, epsilon = 5.0);
    }
}
