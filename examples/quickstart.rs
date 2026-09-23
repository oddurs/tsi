//! The example from the README: design a rocket, then ask how often it
//! would work if you actually built it.
//!
//! ```sh
//! cargo run --example quickstart
//! ```

use tsiolkovsky::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = EngineDatabase::builtin();
    let problem = Problem::builder()
        .payload(Mass::tonnes(5.0))
        .target(Velocity::mps(9_400.0))
        .engines([
            db.get("merlin-1d").unwrap().clone(),
            db.get("rl-10c").unwrap().clone(),
        ])
        .constraints(Constraints::default().with_margin(0.02))
        .build()?;

    let solution = AnalyticalOptimizer.optimize(&problem)?;
    for stage in solution.rocket().stages() {
        println!("{} × {}", stage.engine_count(), stage.engine().name());
    }

    // How often would it work, built with real manufacturing errors?
    let results = MonteCarloRunner::new(Uncertainty::default()).run_design(&solution, 10_000)?;
    println!(
        "{:.1}% of builds reach orbit",
        results.success_probability() * 100.0
    );
    Ok(())
}
