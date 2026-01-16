use anyhow::{bail, Result};

use crate::engine::EngineDatabase;
use crate::optimizer::{
    AnalyticalOptimizer, BruteForceOptimizer, Constraints, MonteCarloRunner, Optimizer, Problem,
    Uncertainty,
};
use crate::output::terminal;
use crate::physics::{burn_time, delta_v, twr, G0};
use crate::units::{format_thousands_f64, Force, Isp, Mass, Ratio, Velocity};

use super::args::{
    CalculateArgs, CalculateOutputFormat, EnginesArgs, OptimizeArgs, OptimizeOutputFormat,
    OptimizerChoice, OutputFormat, UncertaintyLevel,
};

pub fn calculate(args: CalculateArgs) -> Result<()> {
    // Validate inputs first - collect all errors
    let mut errors = Vec::new();

    if let Some(isp) = args.isp {
        if isp <= 0.0 {
            errors.push("--isp must be positive".to_string());
        }
    }
    if let Some(ratio) = args.mass_ratio {
        if ratio <= 1.0 {
            errors.push("--mass-ratio must be greater than 1.0 (wet > dry)".to_string());
        }
    }
    if let Some(wet) = args.wet_mass {
        if wet <= 0.0 {
            errors.push("--wet-mass must be positive".to_string());
        }
    }
    if let Some(dry) = args.dry_mass {
        if dry <= 0.0 {
            errors.push("--dry-mass must be positive".to_string());
        }
    }
    if let (Some(wet), Some(dry)) = (args.wet_mass, args.dry_mass) {
        if wet <= dry {
            errors.push("--wet-mass must be greater than --dry-mass".to_string());
        }
    }
    if let Some(prop) = args.propellant_mass {
        if prop <= 0.0 {
            errors.push("--propellant-mass must be positive".to_string());
        }
    }
    if let Some(thrust) = args.thrust {
        if thrust <= 0.0 {
            errors.push("--thrust must be positive".to_string());
        }
    }
    if args.structural_ratio < 0.0 || args.structural_ratio >= 1.0 {
        errors.push("--structural-ratio must be between 0 and 1".to_string());
    }
    if args.engine_count == 0 {
        errors.push("--engine-count must be at least 1".to_string());
    }

    if !errors.is_empty() {
        let mut msg = "Invalid arguments:\n".to_string();
        for e in &errors {
            msg.push_str(&format!("  - {}\n", e));
        }
        bail!("{}", msg.trim_end());
    }

    let db = EngineDatabase::default();

    // Determine Isp and thrust from either --engine or explicit values
    let (isp, thrust, engine_name, propellant_name) = if let Some(ref engine_name) = args.engine {
        let engine = db.get(engine_name).ok_or_else(|| {
            let mut msg = format!("Unknown engine: '{}'", engine_name);
            let suggestions = db.suggest(engine_name);
            if !suggestions.is_empty() {
                msg.push_str("\n\nDid you mean:");
                for s in suggestions {
                    msg.push_str(&format!("\n  {}", s));
                }
            }
            msg.push_str("\n\nRun `tsi engines` to see all available engines.");
            anyhow::anyhow!(msg)
        })?;

        let isp = engine.isp_vac();
        let thrust = engine.thrust_vac() * args.engine_count;
        let name = if args.engine_count > 1 {
            format!("{} (×{})", engine.name, args.engine_count)
        } else {
            engine.name.clone()
        };
        (
            isp,
            Some(thrust),
            Some(name),
            Some(engine.propellant.name().to_string()),
        )
    } else if let Some(isp_s) = args.isp {
        let thrust = args.thrust.map(Force::newtons);
        (Isp::seconds(isp_s), thrust, None, None)
    } else {
        bail!("Must provide either --engine or --isp");
    };

    // Calculate mass ratio and related values
    if let Some(propellant_kg) = args.propellant_mass {
        // Engine-based calculation with propellant mass
        let propellant = Mass::kg(propellant_kg);
        let structural = Mass::kg(propellant_kg * args.structural_ratio);
        let engine_mass = if let Some(ref name) = args.engine {
            let engine = db.get(name).unwrap();
            engine.dry_mass() * args.engine_count
        } else {
            Mass::kg(0.0)
        };
        let dry_mass = structural + engine_mass;
        let wet_mass = dry_mass + propellant;
        let mass_ratio = wet_mass / dry_mass;

        let dv = delta_v(isp, mass_ratio);

        match args.output {
            CalculateOutputFormat::Compact => {
                // Compact one-line output
                let mut parts = vec![format!("Δv: {}", dv)];
                if let Some(thrust) = thrust {
                    let time = burn_time(propellant, thrust, isp);
                    let twr_val = twr(thrust, wet_mass, G0);
                    parts.push(format!("Burn: {}s", time.as_seconds() as u32));
                    parts.push(format!("TWR: {:.2}", twr_val.as_f64()));
                }
                println!("{}", parts.join(" | "));
            }
            CalculateOutputFormat::Pretty => {
                // Pretty multi-line output
                if let Some(name) = engine_name {
                    println!("Engine:     {}", name);
                }
                if let Some(prop) = propellant_name {
                    println!(
                        "Propellant: {} kg ({})",
                        format_thousands_f64(propellant.as_kg()),
                        prop
                    );
                } else {
                    println!(
                        "Propellant: {} kg",
                        format_thousands_f64(propellant.as_kg())
                    );
                }
                println!("Dry mass:   {} kg", format_thousands_f64(dry_mass.as_kg()));
                println!("Δv:         {}", dv);

                if let Some(thrust) = thrust {
                    let time = burn_time(propellant, thrust, isp);
                    let twr_val = twr(thrust, wet_mass, G0);
                    println!("Burn time:  {}", time);
                    println!("TWR (vac):  {:.2}", twr_val.as_f64());
                }
            }
        }
    } else if let Some(ratio) = args.get_mass_ratio() {
        // Simple mass ratio calculation (original behavior)
        let mass_ratio = Ratio::new(ratio);
        let dv = delta_v(isp, mass_ratio);

        match args.output {
            CalculateOutputFormat::Compact => {
                let mut parts = vec![format!("Δv: {}", dv)];
                if let Some(thrust) = thrust {
                    let propellant = match args.get_propellant_mass() {
                        Some(p) => Mass::kg(p),
                        None => bail!(
                            "Burn time requires propellant mass. Provide --wet-mass/--dry-mass or --propellant-mass"
                        ),
                    };
                    let time = burn_time(propellant, thrust, isp);
                    parts.push(format!("Burn: {}s", time.as_seconds() as u32));
                }
                println!("{}", parts.join(" | "));
            }
            CalculateOutputFormat::Pretty => {
                println!("Δv:         {}", dv);
                println!("Mass ratio: {}", mass_ratio);

                // If thrust is provided, calculate burn time
                if let Some(thrust) = thrust {
                    let propellant = match args.get_propellant_mass() {
                        Some(p) => Mass::kg(p),
                        None => bail!(
                            "Burn time requires propellant mass. Provide --wet-mass/--dry-mass or --propellant-mass"
                        ),
                    };

                    let time = burn_time(propellant, thrust, isp);
                    println!("Burn time:  {}", time);
                }
            }
        }
    } else {
        bail!("Must provide --propellant-mass, --mass-ratio, or --wet-mass/--dry-mass");
    }

    Ok(())
}

pub fn engines(args: EnginesArgs) -> Result<()> {
    let db = EngineDatabase::default();
    let all_engines = db.list();

    // Apply filters
    let engines: Vec<_> = all_engines
        .iter()
        .filter(|e| {
            // Filter by propellant
            if let Some(ref prop_filter) = args.propellant {
                if !e.propellant.matches(prop_filter) {
                    return false;
                }
            }
            // Filter by name
            if let Some(ref name_filter) = args.name {
                if !e.name.to_lowercase().contains(&name_filter.to_lowercase()) {
                    return false;
                }
            }
            true
        })
        .collect();

    if engines.is_empty() {
        let mut msg = "No engines found".to_string();
        if args.propellant.is_some() || args.name.is_some() {
            msg.push_str(" matching the filter");
        }
        msg.push_str(".\nRun `tsi engines` to see all available engines.");
        bail!("{}", msg);
    }

    match args.output {
        OutputFormat::Table => {
            if args.verbose {
                // Verbose output with sea-level values
                println!(
                    "{:<16} {:<12} {:>10} {:>10} {:>8} {:>8} {:>10}",
                    "NAME",
                    "PROPELLANT",
                    "THRUST(vac)",
                    "THRUST(sl)",
                    "ISP(vac)",
                    "ISP(sl)",
                    "MASS"
                );
                println!("{}", "-".repeat(84));
                for engine in &engines {
                    let thrust_sl = if engine.thrust_sl().as_newtons() > 0.0 {
                        format!(
                            "{} kN",
                            format_thousands_f64(engine.thrust_sl().as_kilonewtons())
                        )
                    } else {
                        "-".to_string()
                    };
                    let isp_sl = if engine.isp_sl().as_seconds() > 0.0 {
                        format!("{}s", engine.isp_sl().as_seconds() as u32)
                    } else {
                        "-".to_string()
                    };
                    println!(
                        "{:<16} {:<12} {:>8} kN {:>10} {:>7}s {:>8} {:>10} kg",
                        engine.name,
                        engine.propellant.name(),
                        format_thousands_f64(engine.thrust_vac().as_kilonewtons()),
                        thrust_sl,
                        engine.isp_vac().as_seconds() as u32,
                        isp_sl,
                        format_thousands_f64(engine.dry_mass().as_kg()),
                    );
                }
            } else {
                // Standard output
                println!(
                    "{:<16} {:<12} {:>12} {:>10} {:>10}",
                    "NAME", "PROPELLANT", "THRUST(vac)", "ISP(vac)", "MASS"
                );
                println!("{}", "-".repeat(62));
                for engine in &engines {
                    println!(
                        "{:<16} {:<12} {:>10} kN {:>8}s {:>10} kg",
                        engine.name,
                        engine.propellant.name(),
                        format_thousands_f64(engine.thrust_vac().as_kilonewtons()),
                        engine.isp_vac().as_seconds() as u32,
                        format_thousands_f64(engine.dry_mass().as_kg()),
                    );
                }
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&engines)?;
            println!("{}", json);
        }
    }

    Ok(())
}

/// Optimize staging for a rocket.
pub fn optimize(args: OptimizeArgs) -> Result<()> {
    // Validate inputs
    let mut errors = Vec::new();

    if args.payload <= 0.0 {
        errors.push("--payload must be positive".to_string());
    }
    if args.target_dv <= 0.0 {
        errors.push("--target-dv must be positive".to_string());
    }
    if args.min_twr < 1.0 {
        errors.push("--min-twr must be >= 1.0 for liftoff".to_string());
    }
    if args.min_upper_twr <= 0.0 {
        errors.push("--min-upper-twr must be positive".to_string());
    }
    if args.max_stages == 0 {
        errors.push("--max-stages must be at least 1".to_string());
    }
    if args.structural_ratio <= 0.0 || args.structural_ratio >= 1.0 {
        errors.push("--structural-ratio must be between 0 and 1".to_string());
    }

    if !errors.is_empty() {
        let mut msg = "Invalid arguments:\n".to_string();
        for e in &errors {
            msg.push_str(&format!("  - {}\n", e));
        }
        bail!("{}", msg.trim_end());
    }

    // Load engine database and look up engines (comma-separated)
    let db = EngineDatabase::default();
    let engine_names: Vec<&str> = args.engine.split(',').map(|s| s.trim()).collect();
    let mut engines = Vec::new();

    // Helper to look up and validate an engine
    let lookup_engine = |name: &str| -> Result<crate::engine::Engine> {
        db.get(name).cloned().ok_or_else(|| {
            let mut msg = format!("Unknown engine: '{}'", name);
            let suggestions = db.suggest(name);
            if !suggestions.is_empty() {
                msg.push_str("\n\nDid you mean:");
                for s in suggestions {
                    msg.push_str(&format!("\n  {}", s));
                }
            }
            msg.push_str("\n\nRun `tsi engines` to see all available engines.");
            anyhow::anyhow!(msg)
        })
    };

    for engine_name in &engine_names {
        engines.push(lookup_engine(engine_name)?);
    }

    // Add per-stage engines if specified
    if let Some(ref s1_engine) = args.stage1_engine {
        let engine = lookup_engine(s1_engine)?;
        if !engines.iter().any(|e| e.name == engine.name) {
            engines.push(engine);
        }
    }
    if let Some(ref s2_engine) = args.stage2_engine {
        let engine = lookup_engine(s2_engine)?;
        if !engines.iter().any(|e| e.name == engine.name) {
            engines.push(engine);
        }
    }

    // Build constraints
    let constraints = Constraints::new(
        Ratio::new(args.min_twr),
        Ratio::new(args.min_upper_twr),
        args.max_stages,
        Ratio::new(args.structural_ratio),
    );

    // Build problem
    let problem = Problem::new(
        Mass::kg(args.payload),
        Velocity::mps(args.target_dv),
        engines.clone(),
        constraints,
    )
    .with_stage_count(args.max_stages);

    // Select optimizer
    let show_progress = !args.quiet && args.output == OptimizeOutputFormat::Pretty;
    let solution = match select_optimizer(&args, &problem) {
        SelectedOptimizer::Analytical => {
            let optimizer = AnalyticalOptimizer;
            optimizer
                .optimize(&problem)
                .map_err(|e| anyhow::anyhow!("{}", e))?
        }
        SelectedOptimizer::BruteForce => {
            let optimizer = BruteForceOptimizer::default().with_progress(show_progress);
            optimizer
                .optimize(&problem)
                .map_err(|e| anyhow::anyhow!("{}", e))?
        }
    };

    // Run Monte Carlo analysis if requested
    let mc_results = if let Some(iterations) = args.monte_carlo {
        let uncertainty = uncertainty_from_level(args.uncertainty);
        let show_mc_progress = !args.quiet && args.output == OptimizeOutputFormat::Pretty;

        let runner = MonteCarloRunner::new(uncertainty).with_progress(show_mc_progress);
        Some(
            runner
                .run(&problem, iterations)
                .map_err(|e| anyhow::anyhow!("{}", e))?,
        )
    } else {
        None
    };

    // Output results
    match args.output {
        OptimizeOutputFormat::Pretty => {
            print_solution_pretty(&args, &solution);
            if let Some(ref mc) = mc_results {
                terminal::print_monte_carlo_results(mc);
            }
        }
        OptimizeOutputFormat::Json => {
            print_solution_json(&args, &solution, mc_results.as_ref())?;
        }
    }

    Ok(())
}

/// Convert CLI uncertainty level to Uncertainty struct.
fn uncertainty_from_level(level: UncertaintyLevel) -> Uncertainty {
    match level {
        UncertaintyLevel::None => Uncertainty::none(),
        UncertaintyLevel::Low => Uncertainty::new(0.5, 3.0, 1.0),
        UncertaintyLevel::Default => Uncertainty::default(),
        UncertaintyLevel::High => Uncertainty::new(2.0, 8.0, 3.0),
    }
}

/// Which optimizer to use.
enum SelectedOptimizer {
    Analytical,
    BruteForce,
}

/// Select the appropriate optimizer based on user choice and problem complexity.
fn select_optimizer(args: &OptimizeArgs, problem: &Problem) -> SelectedOptimizer {
    match args.optimizer {
        OptimizerChoice::Analytical => SelectedOptimizer::Analytical,
        OptimizerChoice::BruteForce => SelectedOptimizer::BruteForce,
        OptimizerChoice::Auto => {
            // Auto-select based on problem complexity:
            // - Single engine + 2 stages → Analytical (fast)
            // - Multiple engines or != 2 stages → BruteForce
            let is_simple = problem.is_single_engine() && problem.stage_count == Some(2);

            if is_simple {
                SelectedOptimizer::Analytical
            } else {
                SelectedOptimizer::BruteForce
            }
        }
    }
}

fn print_solution_pretty(args: &OptimizeArgs, solution: &crate::optimizer::Solution) {
    terminal::print_solution_with_options(
        args.target_dv,
        args.payload,
        solution,
        args.gravity.as_mps2(),
        args.sea_level,
    );
}

fn print_solution_json(
    args: &OptimizeArgs,
    solution: &crate::optimizer::Solution,
    mc_results: Option<&crate::optimizer::MonteCarloResults>,
) -> Result<()> {
    let rocket = &solution.rocket;
    let stages = rocket.stages();

    let stages_json: Vec<_> = stages
        .iter()
        .enumerate()
        .map(|(i, stage)| {
            serde_json::json!({
                "stage": i + 1,
                "engine": stage.engine().name,
                "engine_count": stage.engine_count(),
                "propellant_kg": stage.propellant_mass().as_kg(),
                "dry_mass_kg": stage.dry_mass().as_kg(),
                "wet_mass_kg": stage.wet_mass().as_kg(),
                "delta_v_mps": rocket.stage_delta_v(i).as_mps(),
                "burn_time_s": stage.burn_time().as_seconds(),
                "twr": rocket.stage_twr(i).as_f64(),
            })
        })
        .collect();

    let mut output = serde_json::json!({
        "target_delta_v_mps": args.target_dv,
        "payload_kg": args.payload,
        "total_mass_kg": rocket.total_mass().as_kg(),
        "total_delta_v_mps": rocket.total_delta_v().as_mps(),
        "payload_fraction": rocket.payload_fraction().as_f64(),
        "margin_mps": solution.margin.as_mps(),
        "margin_percent": solution.margin_percent(Velocity::mps(args.target_dv)),
        "stages": stages_json,
        "metadata": {
            "optimizer": solution.optimizer_name,
            "iterations": solution.iterations,
            "runtime_ms": solution.runtime.as_millis(),
        },
    });

    // Add Monte Carlo results if available
    if let Some(mc) = mc_results {
        output["monte_carlo"] = serde_json::to_value(mc.to_json_summary())?;
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
