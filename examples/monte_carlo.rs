//! How much margin is enough?
//!
//! A rocket designed to hit its target exactly works about half the time,
//! because half of all builds come out a little worse than the drawings.
//! Margin buys confidence, and it costs mass. This finds where the trade
//! stops being worth it.
//!
//! ```sh
//! cargo run --example monte_carlo
//! ```

use tsiolkovsky::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raptor = EngineDatabase::builtin()
        .get("raptor-2")
        .ok_or("no Raptor-2")?
        .clone();
    let base = Problem::builder()
        .payload(Mass::tonnes(5.0))
        .target(Velocity::mps(9_400.0))
        .engine(raptor)
        .stages(2)
        .build()?;
    let nominal_mass = AnalyticalOptimizer
        .optimize(&base)?
        .rocket()
        .total_mass()
        .as_kg();

    // Production hardware: Isp ±1%, thrust ±2%, structure ±5% (1-sigma)
    let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(1903);

    println!("5 t to 9,400 m/s with Raptor-2, 20,000 builds per design\n");
    println!("margin   success   extra mass");
    for margin in [0.0, 0.005, 0.01, 0.015, 0.02, 0.03, 0.04] {
        let problem = base
            .to_builder()
            .constraints(Constraints::default().with_margin(margin))
            .build()?;
        let design = AnalyticalOptimizer.optimize(&problem)?;
        let results = runner.run_design(&design, 20_000)?;
        let extra = design.rocket().total_mass().as_kg() / nominal_mass - 1.0;
        println!(
            "{:>5.1}%   {:>6.1}%   {:>+8.1}%",
            margin * 100.0,
            results.success_probability() * 100.0,
            extra * 100.0
        );
    }

    println!();
    println!("The first percent of margin buys most of the confidence. After about 2%,");
    println!("each extra percent of delta-v costs mass for a rounding error of success.");
    println!("(Mass jumps where the design needs one more engine.)");
    Ok(())
}
