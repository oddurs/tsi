//! Why do rockets burn hydrogen up high and kerosene down low?
//!
//! Hydrogen has the best Isp of any practical propellant: 450 s against
//! kerosene's 310-350 s. So why isn't every stage hydrogen? Design the same
//! rocket three ways and look past the delta-v.
//!
//! ```sh
//! cargo run --example hydrogen_upper_stage
//! ```

use tsiolkovsky::prelude::*;

/// Design a two-stage rocket for 5 t to low Earth orbit with the given
/// booster and upper-stage engines and structural ratios.
fn design(
    booster: &str,
    upper: &str,
    structural_ratios: [f64; 2],
) -> Result<Solution, Box<dyn std::error::Error>> {
    let db = EngineDatabase::builtin();
    let problem = Problem::builder()
        .payload(Mass::tonnes(5.0))
        .target(Velocity::mps(9_400.0))
        .pin(0, db.get(booster).ok_or("unknown engine")?.clone())
        .pin(1, db.get(upper).ok_or("unknown engine")?.clone())
        .stages(2)
        .constraints(
            Constraints::default()
                .with_structural_ratios(structural_ratios)
                .with_max_engines(20),
        )
        .build()?;
    Ok(AnalyticalOptimizer.optimize(&problem)?)
}

/// Propellant tank volume of a stage, in cubic metres.
fn tank_volume(stage: &Stage) -> f64 {
    stage.propellant_mass().as_kg() / stage.engine().propellant().density()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Structural ratios from real stages (see the table on `Stage`): dense
    // kerosene stages run 3-5%, hydrogen needs big insulated tanks, 6-11%.
    let kerosene = [0.045, 0.035];
    let designs = [
        (
            "Kerosene all the way",
            "merlin-1d",
            "merlin-vacuum",
            kerosene,
        ),
        ("Hydrogen upper stage", "merlin-1d", "rl-10c", [0.045, 0.10]),
        ("Hydrogen all the way", "rs-25", "rl-10c", [0.10, 0.10]),
    ];

    println!("5 t to low Earth orbit (9,400 m/s), three ways\n");
    println!("                         liftoff mass   booster tanks   booster Isp");
    let mut masses = Vec::new();
    let mut volumes = Vec::new();
    for (name, booster, upper, ratios) in designs {
        let solution = design(booster, upper, ratios)?;
        let rocket = solution.rocket();
        let first = &rocket.stages()[0];
        let isp = first.engine().isp_for(rocket.booster_isp());
        println!(
            "{name:<24} {:>12}   {:>9.0} m³   {:>5.0} s of {:.0} s",
            rocket.total_mass().to_string(),
            tank_volume(first),
            isp.as_seconds(),
            first.engine().isp_vac().as_seconds(),
        );
        masses.push(rocket.total_mass().as_kg());
        volumes.push(tank_volume(first));
    }

    let upper_saving = 1.0 - masses[1] / masses[0];
    println!();
    println!("Up high, hydrogen wins outright: even with tanks three times heavier,");
    println!(
        "the hydrogen upper stage makes the whole rocket {:.0}% lighter. Every kilogram",
        upper_saving * 100.0
    );
    println!("saved at the top saves several at the bottom.");
    println!();
    println!("Down low, the mass column still favours hydrogen, and that is where ideal");
    println!("delta-v stops telling the whole story:");
    let db = EngineDatabase::builtin();
    let per_kg = |name: &str| -> Result<f64, Box<dyn std::error::Error>> {
        let e = db.get(name).ok_or("unknown engine")?;
        Ok(e.thrust_sl().as_newtons() / e.dry_mass().as_kg())
    };
    println!("  - The RS-25 gives up a fifth of its Isp at sea level; a Merlin, a tenth.");
    println!(
        "  - Hydrogen is so light that the hydrogen booster needs {:.1}x the kerosene",
        volumes[2] / volumes[0]
    );
    println!(
        "    booster's tank volume while lifting {:.0}% less mass. A fatter rocket pays",
        (1.0 - masses[2] / masses[0]) * 100.0
    );
    println!("    in drag, which ideal delta-v leaves out.");
    println!(
        "  - Hydrogen engines make little thrust for their weight: an RS-25 makes {:.0} N",
        per_kg("rs-25")?
    );
    println!(
        "    at sea level per kg of engine, a Merlin {:.0}. The Space Shuttle and SLS",
        per_kg("merlin-1d")?
    );
    println!("    strap solid boosters to their hydrogen cores to get off the pad.");
    Ok(())
}
