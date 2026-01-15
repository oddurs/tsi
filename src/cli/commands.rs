use anyhow::{bail, Result};

use crate::physics::{burn_time, delta_v};
use crate::units::{Force, Isp, Mass, Ratio};

use super::args::CalculateArgs;

pub fn calculate(args: CalculateArgs) -> Result<()> {
    let isp = Isp::seconds(args.isp);

    let mass_ratio = match args.get_mass_ratio() {
        Some(r) => Ratio::new(r),
        None => bail!("Must provide either --mass-ratio or both --wet-mass and --dry-mass"),
    };

    let dv = delta_v(isp, mass_ratio);

    println!("Δv:         {}", dv);
    println!("Mass ratio: {}", mass_ratio);

    // If thrust is provided, calculate burn time
    if let Some(thrust_n) = args.thrust {
        let thrust = Force::newtons(thrust_n);

        let propellant = match args.get_propellant_mass() {
            Some(p) => Mass::kg(p),
            None => bail!(
                "Burn time requires propellant mass. Provide --wet-mass/--dry-mass or --propellant-mass"
            ),
        };

        let time = burn_time(propellant, thrust, isp);
        println!("Burn time:  {}", time);
    }

    Ok(())
}
