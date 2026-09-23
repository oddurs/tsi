//! What if Saturn V's first stage had Raptors?
//!
//! Keep the real S-II and S-IVB and the Apollo spacecraft on top, and ask
//! for a first stage that does exactly what the S-IC did, first with its own
//! F-1 engines, then with SpaceX's Raptor-2.
//!
//! ```sh
//! cargo run --example saturn_v_with_raptors
//! ```

use tsiolkovsky::prelude::*;

/// A stage from published propellant and dry mass; the structure is the dry
/// mass less tsi's engine masses.
fn stage(
    engine: &str,
    count: u32,
    propellant_t: f64,
    dry_t: f64,
) -> Result<Stage, Box<dyn std::error::Error>> {
    let engine = EngineDatabase::builtin()
        .get(engine)
        .ok_or("unknown engine")?
        .clone();
    let structure = Mass::tonnes(dry_t) - engine.dry_mass() * count;
    Ok(Stage::new(
        engine,
        count,
        Mass::tonnes(propellant_t),
        structure,
    )?)
}

/// Design a first stage that gives `delta_v` to everything above it.
fn first_stage(
    engine: &str,
    above: Mass,
    delta_v: Velocity,
) -> Result<Rocket, Box<dyn std::error::Error>> {
    let problem = Problem::builder()
        .payload(above)
        .target(delta_v)
        .pin(
            0,
            EngineDatabase::builtin()
                .get(engine)
                .ok_or("unknown engine")?
                .clone(),
        )
        .stages(1)
        .constraints(
            Constraints::default()
                .with_structural_ratio(0.041) // the real S-IC's
                .with_min_liftoff_twr(1.15) // Saturn V lifted off at about 1.18
                .with_max_engines(60),
        )
        .build()?;
    Ok(AnalyticalOptimizer.optimize(&problem)?.into_rocket())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Saturn V as flown on Apollo 11 (NASA SP-4206, appendix)
    let s_ic = stage("f-1", 5, 2_160.0, 131.0)?;
    let s_ii = stage("j-2", 5, 443.0, 36.0)?;
    let s_ivb = stage("j-2", 1, 107.0, 13.5)?;
    let apollo = Mass::tonnes(45.0);
    let saturn = Rocket::new(vec![s_ic, s_ii.clone(), s_ivb.clone()], apollo)?;

    let upper_stack = saturn.mass_above_stage(0);
    let job = saturn.stage_delta_v(0);
    println!("Saturn V: {} at liftoff", saturn.total_mass());
    println!("  the S-IC gives {job} to {upper_stack} of upper stages and spacecraft\n");

    println!("A first stage for that job:");
    println!("                 engines   stage mass    liftoff mass   burn time");
    for (label, engine) in [("F-1 (1960s)", "f-1"), ("Raptor-2 (2020s)", "raptor-2")] {
        let booster = first_stage(engine, upper_stack, job)?;
        let s = &booster.stages()[0];
        println!(
            "{label:<17} {:>3} ×   {:>12}   {:>12}   {}",
            s.engine_count(),
            s.wet_mass().to_string(),
            booster.total_mass().to_string(),
            s.burn_time()
        );
    }

    let f1 = first_stage("f-1", upper_stack, job)?;
    let raptor = first_stage("raptor-2", upper_stack, job)?;
    let saving = f1.total_mass() - raptor.total_mass();
    let real_s_ic = saturn.stages()[0].wet_mass();
    println!();
    println!(
        "With F-1s, tsi's design comes within {:.1}% of the real S-IC ({}), with the",
        ((f1.stages()[0].wet_mass().as_kg() / real_s_ic.as_kg()) - 1.0).abs() * 100.0,
        real_s_ic
    );
    println!("same five engines. So the Raptor numbers are a fair comparison.");
    println!();
    println!(
        "Raptors save {} of propellant and structure on the pad, {:.0}% of the",
        saving,
        saving.as_kg() / f1.total_mass().as_kg() * 100.0
    );
    println!("whole rocket. Two things do it: methane and a high-pressure staged-combustion");
    println!("cycle give Raptor about 45 s more Isp than the F-1, and it weighs a fifth as");
    println!("much for a third of the thrust. It takes a lot more of them, though.");
    Ok(())
}
