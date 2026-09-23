use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use anyhow::{bail, Result};
use clap::CommandFactory;
use clap_complete::generate;
use serde::Serialize;

use tsiolkovsky::engine::{Engine, EngineDatabase, Propellant};
use tsiolkovsky::optimizer::{
    AnalyticalOptimizer, BruteForceOptimizer, Constraints, Infeasibility, MonteCarloResults,
    MonteCarloRunner, OptimizeError, Optimizer, Problem, Progress, Solution, Uncertainty,
};
use tsiolkovsky::physics::losses;
use tsiolkovsky::physics::{burn_time, delta_v, twr, G0};
use tsiolkovsky::units::{format_thousands_f64, Force, Isp, Mass, Ratio, Velocity};

use crate::output::{diagram, terminal};

use super::args::{
    CalculateArgs, CalculateOutputFormat, Cli, CompletionsArgs, EnginesArgs, OptimizeArgs,
    OptimizeOutputFormat, OptimizerChoice, OutputFormat, UncertaintyLevel,
};

pub fn calculate(args: CalculateArgs) -> Result<()> {
    // Validate inputs first - collect all errors
    let mut errors = Vec::new();

    if let Some(isp) = args.isp {
        if !positive(isp) {
            errors.push("--isp must be a positive number".to_string());
        }
    }
    if let Some(ratio) = args.mass_ratio {
        if !(ratio.is_finite() && ratio > 1.0) {
            errors.push("--mass-ratio must be greater than 1.0 (wet > dry)".to_string());
        }
    }
    if let Some(wet) = args.wet_mass {
        if !positive(wet) {
            errors.push("--wet-mass must be positive".to_string());
        }
    }
    if let Some(dry) = args.dry_mass {
        if !positive(dry) {
            errors.push("--dry-mass must be positive".to_string());
        }
    }
    if let (Some(wet), Some(dry)) = (args.wet_mass, args.dry_mass) {
        if wet <= dry {
            errors.push("--wet-mass must be greater than --dry-mass".to_string());
        }
    }
    if let Some(prop) = args.propellant_mass {
        if !positive(prop) {
            errors.push("--propellant-mass must be positive".to_string());
        }
    }
    if let Some(thrust) = args.thrust {
        if !positive(thrust) {
            errors.push("--thrust must be a positive number".to_string());
        }
    }
    if !(args.structural_ratio >= 0.0 && args.structural_ratio < 1.0) {
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
            format!("{} (×{})", engine.name(), args.engine_count)
        } else {
            engine.name().to_string()
        };
        (
            isp,
            Some(thrust),
            Some(name),
            Some(engine.propellant().name().to_string()),
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
                if !e.propellant().matches(prop_filter) {
                    return false;
                }
            }
            // Filter by name
            if let Some(ref name_filter) = args.name {
                if !e
                    .name()
                    .to_lowercase()
                    .contains(&name_filter.to_lowercase())
                {
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
                        engine.name(),
                        engine.propellant().name(),
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
                        engine.name(),
                        engine.propellant().name(),
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

    if !positive(args.payload) {
        errors.push("--payload must be a positive number".to_string());
    }
    if !positive(args.target_dv) {
        errors.push("--target-dv must be a positive number".to_string());
    }
    if !(args.min_twr.is_finite() && args.min_twr >= 1.0) {
        errors.push("--min-twr must be >= 1.0 for liftoff".to_string());
    }
    if !positive(args.min_upper_twr) {
        errors.push("--min-upper-twr must be positive".to_string());
    }
    if args.max_stages == 0 {
        errors.push("--max-stages must be at least 1".to_string());
    }
    if args.stages == Some(0) {
        errors.push("--stages must be at least 1".to_string());
    }
    if args.max_engines == 0 {
        errors.push("--max-engines must be at least 1".to_string());
    }
    if args.structural_ratio.iter().any(|&r| !(r > 0.0 && r < 1.0)) {
        errors.push("--structural-ratio values must be between 0 and 1".to_string());
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

    // Parse custom engines first (so they can be referenced by name)
    let mut custom_engines: Vec<Engine> = Vec::new();
    for spec in &args.custom_engine {
        let engine = parse_custom_engine(spec)?;
        custom_engines.push(engine);
    }

    // Helper to look up and validate an engine (checks custom engines first)
    let lookup_engine = |name: &str| -> Result<Engine> {
        // Check custom engines first
        if let Some(engine) = custom_engines
            .iter()
            .find(|e| e.name().eq_ignore_ascii_case(name))
        {
            return Ok(engine.clone());
        }

        // Then check database
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
            if !custom_engines.is_empty() {
                msg.push_str("\n\nCustom engines defined:");
                for e in &custom_engines {
                    msg.push_str(&format!("\n  {}", e.name()));
                }
            }
            anyhow::anyhow!(msg)
        })
    };

    for engine_name in &engine_names {
        engines.push(lookup_engine(engine_name)?);
    }

    // Build constraints. The launch body sets gravity and whether the first
    // stage flies through an atmosphere.
    let max_stages = args
        .stages
        .map_or(args.max_stages, |n| n.max(args.max_stages));
    let constraints = Constraints::default()
        .with_min_liftoff_twr(args.min_twr)
        .with_min_stage_twr(args.min_upper_twr)
        .with_max_stages(max_stages)
        .with_structural_ratios(args.structural_ratio.iter().copied())
        .with_max_engines(args.max_engines)
        .with_margin(args.margin)
        .with_surface_gravity(args.gravity.as_mps2())
        .with_booster_isp(args.gravity.booster_isp());

    // Build problem
    let mut builder = Problem::builder()
        .payload(Mass::kg(args.payload))
        .target(Velocity::mps(args.target_dv))
        .engines(engines)
        .constraints(constraints);
    if let Some(n) = args.stages {
        builder = builder.stages(n);
    }
    for (index, name) in [(0, &args.stage1_engine), (1, &args.stage2_engine)] {
        if let Some(name) = name {
            builder = builder.pin(index, lookup_engine(name)?);
        }
    }
    let problem = builder
        .build()
        .map_err(|e| anyhow::anyhow!("Invalid problem: {e}"))?;

    if args.sea_level {
        eprintln!(
            "warning: --sea-level is deprecated and has no effect; \
            liftoff TWR always uses sea-level thrust"
        );
    }

    // Select optimizer
    let show_progress = !args.quiet && args.output == OptimizeOutputFormat::Pretty;
    let solution = match select_optimizer(&args) {
        SelectedOptimizer::Analytical => AnalyticalOptimizer.optimize(&problem),
        SelectedOptimizer::BruteForce => {
            let mut optimizer = BruteForceOptimizer::default();
            if show_progress {
                eprintln!("  Optimizer: BruteForce (parallel)");
                optimizer = optimizer.with_progress(StderrProgress::default());
            }
            optimizer.optimize(&problem)
        }
    }
    .map_err(|e| anyhow::anyhow!(explain(&e)))?;

    // Stress the design with Monte Carlo analysis if requested
    let mc_results = match args.monte_carlo {
        Some(iterations) => {
            let mut runner = MonteCarloRunner::new(uncertainty_from_level(args.uncertainty));
            if show_progress {
                runner = runner.with_progress(StderrProgress::default());
            }
            if let Some(seed) = args.seed {
                runner = runner.with_seed(seed);
            }
            Some(runner.run_design(&solution, iterations)?)
        }
        None => None,
    };

    // Output results
    match args.output {
        OptimizeOutputFormat::Pretty => {
            terminal::print_solution(&solution, args.margin);
            if args.diagram {
                diagram::print_rocket_diagram(solution.rocket(), args.payload);
            }
            if args.show_losses {
                let losses = losses::ascent_losses(solution.rocket());
                terminal::print_losses(&losses, solution.rocket().total_delta_v());
            }
            if let Some(ref mc) = mc_results {
                terminal::print_monte_carlo_results(mc);
            }
        }
        OptimizeOutputFormat::Json => {
            let output = OptimizeJson {
                schema_version: JSON_SCHEMA_VERSION,
                design_margin_percent: args.margin * 100.0,
                solution: &solution,
                monte_carlo: mc_results.as_ref(),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

/// Version of the `tsi optimize --output json` format. Raised whenever a
/// field is removed or changes meaning; adding fields doesn't raise it.
const JSON_SCHEMA_VERSION: u32 = 1;

/// The JSON document `tsi optimize --output json` prints.
#[derive(Serialize)]
struct OptimizeJson<'a> {
    schema_version: u32,
    design_margin_percent: f64,
    #[serde(flatten)]
    solution: &'a Solution,
    #[serde(skip_serializing_if = "Option::is_none")]
    monte_carlo: Option<&'a MonteCarloResults>,
}

/// Turn an optimization error into a message with advice about which flags
/// to change. The library reports causes; suggesting flags is the CLI's job.
fn explain(error: &OptimizeError) -> String {
    let OptimizeError::Infeasible(cause) = error else {
        return error.to_string();
    };
    let suggestions: Vec<String> = match cause {
        Infeasibility::StructuralLimit { .. } => vec![
            "Lower --structural-ratio".into(),
            "Allow more stages with --max-stages".into(),
            "Use an engine with higher ISP".into(),
            "Reduce target delta-v".into(),
        ],
        Infeasibility::EngineLimit {
            stage,
            required_twr,
            ..
        } => vec![
            "Allow more engines with --max-engines".into(),
            format!(
                "Lower {} (currently {:.2})",
                twr_flag(*stage),
                required_twr.as_f64()
            ),
            "Use an engine with higher thrust".into(),
        ],
        Infeasibility::EnginesTooHeavy {
            stage,
            required_twr,
            ..
        } => vec![
            format!(
                "Lower {} (currently {:.2})",
                twr_flag(*stage),
                required_twr.as_f64()
            ),
            "Use an engine with a better thrust-to-weight ratio".into(),
        ],
        Infeasibility::BoosterTooSmall { .. } => vec![
            "Increase target delta-v, or use a single stage with --stages 1".into(),
            "Use a first-stage engine with more thrust".into(),
        ],
        Infeasibility::SearchMissed { .. } => vec!["Use --optimizer analytical".into()],
        _ => vec![],
    };
    let mut message = error.to_string();
    if !suggestions.is_empty() {
        message.push_str("\n\nSuggestions:");
        for s in suggestions {
            message.push_str("\n  - ");
            message.push_str(&s);
        }
    }
    message
}

/// The flag that sets the minimum TWR for a stage.
fn twr_flag(stage: usize) -> &'static str {
    if stage == 0 {
        "--min-twr"
    } else {
        "--min-upper-twr"
    }
}

/// Progress on stderr, in the same shape tsi has always printed: a phase
/// line and "Searching... NN%" for brute force, "Monte Carlo: NN% (done/total)"
/// for Monte Carlo.
#[derive(Default)]
struct StderrProgress {
    total: AtomicU64,
    shown: AtomicU64,
    monte_carlo: AtomicBool,
}

impl Progress for StderrProgress {
    fn start(&self, phase: &str, total: u64) {
        self.total.store(total.max(1), Ordering::Relaxed);
        self.shown.store(u64::MAX, Ordering::Relaxed);
        let monte_carlo = phase == "Monte Carlo";
        self.monte_carlo.store(monte_carlo, Ordering::Relaxed);
        if !monte_carlo {
            eprintln!("  {phase}");
        }
    }

    fn advance(&self, done: u64) {
        let total = self.total.load(Ordering::Relaxed);
        let percent = done.min(total) * 100 / total;
        // Redraw only when the whole-number percentage changes.
        if self.shown.swap(percent, Ordering::Relaxed) != percent {
            if self.monte_carlo.load(Ordering::Relaxed) {
                eprint!("\rMonte Carlo: {percent}% ({done}/{total})");
            } else {
                eprint!("\r  Searching... {percent}%");
            }
            let _ = io::stderr().flush();
        }
    }

    fn finish(&self) {
        if self.monte_carlo.load(Ordering::Relaxed) {
            let total = self.total.load(Ordering::Relaxed);
            eprintln!("\rMonte Carlo: 100% ({total}/{total})");
        } else {
            eprintln!();
        }
    }
}

/// Convert CLI uncertainty level to Uncertainty struct.
fn uncertainty_from_level(level: UncertaintyLevel) -> Uncertainty {
    match level {
        UncertaintyLevel::None => Uncertainty::none(),
        UncertaintyLevel::Low => Uncertainty::low(),
        UncertaintyLevel::Default => Uncertainty::default(),
        UncertaintyLevel::High => Uncertainty::high(),
    }
}

/// Which optimizer to use.
enum SelectedOptimizer {
    Analytical,
    BruteForce,
}

/// Select the optimizer. The analytical optimizer handles every problem the
/// CLI can express, so it is the automatic choice; brute force is there as an
/// independent cross-check.
fn select_optimizer(args: &OptimizeArgs) -> SelectedOptimizer {
    match args.optimizer {
        OptimizerChoice::Analytical | OptimizerChoice::Auto => SelectedOptimizer::Analytical,
        OptimizerChoice::BruteForce => SelectedOptimizer::BruteForce,
    }
}

/// A finite number greater than zero. NaN fails every comparison, so
/// checking `x <= 0.0` alone would let it through.
fn positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

/// Parse a custom engine specification string.
///
/// Format: name:thrust_kn:isp_s:mass_kg:propellant
///
/// Example: "MyEngine:2000:350:1500:loxch4"
fn parse_custom_engine(spec: &str) -> Result<Engine> {
    let parts: Vec<&str> = spec.split(':').collect();

    if parts.len() != 5 {
        bail!(
            "Invalid custom engine format: '{}'\n\n\
            Expected format: name:thrust_kn:isp_s:mass_kg:propellant\n\
            Example: MyEngine:2000:350:1500:loxch4\n\n\
            Propellant types: loxrp1, loxlh2, loxch4, n2o4udmh, solid",
            spec
        );
    }

    let name = parts[0].to_string();
    if name.is_empty() {
        bail!("Custom engine name cannot be empty");
    }

    let thrust_kn: f64 = parts[1].parse().map_err(|_| {
        anyhow::anyhow!(
            "Invalid thrust value '{}' in custom engine '{}'.\n\
            Thrust should be in kilonewtons (e.g., 2000 for 2000 kN)",
            parts[1],
            name
        )
    })?;
    if !positive(thrust_kn) {
        bail!("Thrust must be positive for custom engine '{}'", name);
    }

    let isp_s: f64 = parts[2].parse().map_err(|_| {
        anyhow::anyhow!(
            "Invalid ISP value '{}' in custom engine '{}'.\n\
            ISP should be in seconds (e.g., 350 for 350s)",
            parts[2],
            name
        )
    })?;
    if !positive(isp_s) {
        bail!("ISP must be positive for custom engine '{}'", name);
    }

    let mass_kg: f64 = parts[3].parse().map_err(|_| {
        anyhow::anyhow!(
            "Invalid mass value '{}' in custom engine '{}'.\n\
            Mass should be in kilograms (e.g., 1500 for 1500 kg)",
            parts[3],
            name
        )
    })?;
    if !positive(mass_kg) {
        bail!("Mass must be positive for custom engine '{}'", name);
    }

    let propellant = parse_propellant(parts[4]).map_err(|_| {
        anyhow::anyhow!(
            "Unknown propellant '{}' in custom engine '{}'.\n\n\
            Valid propellant types:\n\
              loxrp1   - LOX/RP-1 (kerosene)\n\
              loxlh2   - LOX/LH2 (hydrogen)\n\
              loxch4   - LOX/CH4 (methane)\n\
              n2o4udmh - N2O4/UDMH (hypergolic)\n\
              solid    - Solid propellant",
            parts[4],
            name
        )
    })?;

    // Create engine with vacuum values (assume vacuum-optimized for custom engines)
    // Use 90% of vacuum thrust for sea level (rough approximation)
    let thrust_vac = Force::kilonewtons(thrust_kn);
    let thrust_sl = Force::kilonewtons(thrust_kn * 0.9);
    let isp_vac = Isp::seconds(isp_s);
    let isp_sl = Isp::seconds(isp_s * 0.85); // Rough sea-level approximation

    Engine::new(
        name,
        thrust_sl,
        thrust_vac,
        isp_sl,
        isp_vac,
        Mass::kg(mass_kg),
        propellant,
    )
    .map_err(|e| anyhow::anyhow!("Invalid custom engine: {e}"))
}

/// Parse propellant type from string.
fn parse_propellant(s: &str) -> Result<Propellant> {
    match s.to_lowercase().as_str() {
        "loxrp1" | "lox-rp1" | "kerosene" => Ok(Propellant::LoxRp1),
        "loxlh2" | "lox-lh2" | "hydrogen" => Ok(Propellant::LoxLh2),
        "loxch4" | "lox-ch4" | "methane" => Ok(Propellant::LoxCh4),
        "n2o4udmh" | "hypergolic" => Ok(Propellant::N2o4Udmh),
        "solid" => Ok(Propellant::Solid),
        _ => bail!("Unknown propellant type: {}", s),
    }
}

/// Generate shell completions or man page.
pub fn completions(args: CompletionsArgs) -> Result<()> {
    if args.man {
        // Generate man page
        let cmd = Cli::command();
        let man = clap_mangen::Man::new(cmd);
        man.render(&mut io::stdout())?;
    } else if let Some(shell) = args.shell {
        // Generate shell completions
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut io::stdout());
    } else {
        bail!(
            "Please specify a shell or --man.\n\n\
            Usage:\n  \
            tsi completions bash     # Bash completions\n  \
            tsi completions zsh      # Zsh completions\n  \
            tsi completions fish     # Fish completions\n  \
            tsi completions --man    # Man page"
        );
    }

    Ok(())
}
