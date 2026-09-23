//! Why does Falcon 9 have nine engines?
//!
//! Build Falcon 9 from its published stage masses, then try it with seven to
//! ten Merlins. The rocket equation doesn't care how many engines there are;
//! the pad does.
//!
//! ```sh
//! cargo run --example falcon9
//! ```

use tsiolkovsky::prelude::*;

/// Falcon 9 Block 5, expendable: published propellant and dry masses, with
/// tsi's engine masses taken out of the dry mass to leave the structure.
fn falcon_9(first_stage_engines: u32) -> Result<Rocket, Box<dyn std::error::Error>> {
    let db = EngineDatabase::builtin();
    let merlin = db.get("merlin-1d").ok_or("no Merlin-1D")?.clone();
    let mvac = db.get("merlin-vacuum").ok_or("no Merlin Vacuum")?.clone();

    // Nine engines' worth of structure, whatever we bolt on
    let s1_structure = Mass::kg(22_200.0) - merlin.dry_mass() * 9;
    let s2_structure = Mass::kg(4_000.0) - mvac.dry_mass();

    let s1 = Stage::new(
        merlin,
        first_stage_engines,
        Mass::kg(411_000.0),
        s1_structure,
    )?;
    let s2 = Stage::new(mvac, 1, Mass::kg(111_500.0), s2_structure)?;
    Ok(Rocket::new(vec![s1, s2], Mass::tonnes(22.8))?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let f9 = falcon_9(9)?;
    println!("Falcon 9, expendable, 22.8 t to low Earth orbit");
    println!("  liftoff mass     {}", f9.total_mass());
    println!("  ideal delta-v    {}", f9.total_delta_v());
    println!(
        "  booster delta-v  {} (carrying the second stage)",
        f9.stage_delta_v(0)
    );
    println!();

    println!("Merlins   liftoff TWR   with one engine out");
    for n in 7..=10 {
        let rocket = falcon_9(n)?;
        let twr = rocket.liftoff_twr().as_f64();
        let engine_out = rocket.liftoff_twr().as_f64() * f64::from(n - 1) / f64::from(n);
        let verdict = if engine_out >= 1.2 {
            "still climbs briskly"
        } else if engine_out > 1.0 {
            "barely climbs"
        } else {
            "can't hold itself up"
        };
        println!("   {n:>2}         {twr:.2}          {engine_out:.2}  {verdict}");
    }
    println!();
    println!("Eight engines would lift it (TWR 1.21), just. Nine give 1.36: a brisk");
    println!("climb that wastes less delta-v fighting gravity, and room to lose an");
    println!("engine. On CRS-1 in 2012 one failed 79 seconds into flight, and Dragon");
    println!("still reached its orbit.");
    println!();

    // Ask the optimizer the same question with no hint.
    let db = EngineDatabase::builtin();
    let problem = Problem::builder()
        .payload(f9.payload())
        .target(f9.total_delta_v())
        .pin(0, db.get("merlin-1d").ok_or("no Merlin-1D")?.clone())
        .pin(
            1,
            db.get("merlin-vacuum").ok_or("no Merlin Vacuum")?.clone(),
        )
        .stages(2)
        .constraints(
            Constraints::default()
                .with_structural_ratio(0.04)
                .with_min_liftoff_twr(1.2),
        )
        .build()?;
    let design = AnalyticalOptimizer.optimize(&problem)?;
    let rocket = design.rocket();
    println!(
        "Given Falcon 9's job and its engines, tsi designs a {} rocket with {} Merlins",
        rocket.total_mass(),
        rocket.stages()[0].engine_count()
    );
    println!(
        "on the booster ({:+.1}% against the real thing).",
        (rocket.total_mass().as_kg() / f9.total_mass().as_kg() - 1.0) * 100.0
    );
    Ok(())
}
