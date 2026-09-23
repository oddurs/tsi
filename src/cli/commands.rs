use std::io::{self, Write};
use std::sync::Mutex;

use anyhow::{bail, Result};
use clap::CommandFactory;
use clap_complete::generate;
use serde::Serialize;

use tsiolkovsky::engine::{Engine, EngineDatabase, Propellant};
use tsiolkovsky::optimizer::{
    AnalyticalOptimizer, BruteForceOptimizer, ConstraintError, Constraints, Infeasibility,
    MonteCarloResults, MonteCarloRunner, OptimizeError, Optimizer, Problem, ProblemError, Progress,
    Solution, Uncertainty,
};
use tsiolkovsky::physics::losses;
use tsiolkovsky::physics::{burn_time, delta_v, twr, G0};
use tsiolkovsky::units::{Force, Isp, Mass, Ratio, Velocity};

use crate::output::data::{CalculateReport, EnginesReport, Envelope};
use crate::output::{views, Look};

use super::args::{
    CalculateArgs, CalculateOutputFormat, Cli, CompletionsArgs, EnginesArgs, OptimizeArgs,
    OptimizeOutputFormat, OptimizerChoice, OutputFormat, UncertaintyLevel,
};

pub fn calculate(args: CalculateArgs, look: Look) -> Result<()> {
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

    let report = calculate_report(&args)?;
    match args.output {
        CalculateOutputFormat::Pretty => look.print(&views::calculate(&report)),
        CalculateOutputFormat::Compact => {
            let mut parts = vec![format!("Δv: {}", Velocity::mps(report.delta_v_mps))];
            if let Some(t) = report.burn_time_s {
                parts.push(format!("Burn: {}s", t as u32));
            }
            if let Some(twr) = report.twr_vacuum {
                parts.push(format!("TWR: {twr:.2}"));
            }
            println!("{}", parts.join(" | "));
        }
        CalculateOutputFormat::Json => {
            println!("{}", Envelope::new("calculate", &report).to_json()?);
        }
    }

    Ok(())
}

pub fn engines(args: EnginesArgs, look: Look) -> Result<()> {
    let db = EngineDatabase::builtin();
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
        OutputFormat::Pretty => {
            let filtered = args.propellant.is_some() || args.name.is_some();
            look.print(&views::engines(&engines, args.verbose, filtered));
        }
        OutputFormat::Json => {
            println!(
                "{}",
                Envelope::new("engines", EnginesReport { engines: &engines }).to_json()?
            );
        }
    }

    Ok(())
}

/// Work out one stage's performance from whichever inputs were given.
fn calculate_report(args: &CalculateArgs) -> Result<CalculateReport> {
    let db = EngineDatabase::builtin();
    let engine = match &args.engine {
        Some(name) => Some(db.get(name).ok_or_else(|| {
            let mut msg = format!("Unknown engine: '{name}'");
            let suggestions = db.suggest(name);
            if !suggestions.is_empty() {
                msg.push_str("\n\nDid you mean:");
                for s in suggestions {
                    msg.push_str(&format!("\n  {s}"));
                }
            }
            msg.push_str("\n\nRun `tsi engines` to see all available engines.");
            anyhow::anyhow!(msg)
        })?),
        None => None,
    };

    let (isp, thrust) = match (engine, args.isp) {
        (Some(e), _) => (e.isp_vac(), Some(e.thrust_vac() * args.engine_count)),
        (None, Some(isp)) => (Isp::seconds(isp), args.thrust.map(Force::newtons)),
        (None, None) => bail!("Must provide either --engine or --isp"),
    };

    // Masses: from propellant and structure, or from wet and dry, or just a ratio
    let (mass_ratio, propellant, dry, wet) = if let Some(p) = args.propellant_mass {
        let engines = engine.map_or(Mass::kg(0.0), |e| e.dry_mass() * args.engine_count);
        let dry = Mass::kg(p * args.structural_ratio) + engines;
        let wet = dry + Mass::kg(p);
        (wet / dry, Some(Mass::kg(p)), Some(dry), Some(wet))
    } else if let Some(ratio) = args.get_mass_ratio() {
        let propellant = args.get_propellant_mass().map(Mass::kg);
        let dry = args.dry_mass.map(Mass::kg);
        let wet = args.wet_mass.map(Mass::kg);
        (Ratio::new(ratio), propellant, dry, wet)
    } else {
        bail!("Must provide --propellant-mass, --mass-ratio, or --wet-mass/--dry-mass");
    };

    let burn = match (thrust, propellant) {
        (Some(t), Some(p)) => Some(burn_time(p, t, isp)),
        (Some(_), None) => bail!(
            "Burn time requires propellant mass. Provide --wet-mass/--dry-mass or --propellant-mass"
        ),
        _ => None,
    };
    let twr_vacuum = match (thrust, wet) {
        (Some(t), Some(w)) => Some(twr(t, w, G0).as_f64()),
        _ => None,
    };

    Ok(CalculateReport {
        engine: engine.map(|e| e.name().to_string()),
        engine_count: engine.map(|_| args.engine_count),
        propellant: engine.map(|e| e.propellant().name().to_string()),
        isp_s: isp.as_seconds(),
        mass_ratio: mass_ratio.as_f64(),
        delta_v_mps: delta_v(isp, mass_ratio).as_mps(),
        propellant_kg: propellant.map(|m| m.as_kg()),
        dry_mass_kg: dry.map(|m| m.as_kg()),
        wet_mass_kg: wet.map(|m| m.as_kg()),
        thrust_n: thrust.map(|t| t.as_newtons()),
        burn_time_s: burn.map(|t| t.as_seconds()),
        twr_vacuum,
    })
}

/// Optimize staging for a rocket.
pub fn optimize(args: OptimizeArgs, look: Look) -> Result<()> {
    // The library validates the problem; `explain_problem` names the flags.
    // Load engine database and look up engines (comma-separated)
    let db = EngineDatabase::builtin();
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
        .map_err(|e| anyhow::anyhow!(explain_problem(&e)))?;

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
                optimizer = optimizer.with_progress(StderrProgress::search());
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
                runner = runner.with_progress(StderrProgress::monte_carlo());
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
            let mut doc = views::optimize(&solution, args.margin);
            if args.diagram {
                doc.extend(views::diagram(solution.rocket(), look.ascii));
            }
            if args.show_losses {
                let estimate = losses::ascent_losses(solution.rocket());
                doc.extend(views::losses(&estimate, solution.rocket()));
            }
            if let Some(ref mc) = mc_results {
                doc.extend(views::monte_carlo(
                    mc,
                    &uncertainty_from_level(args.uncertainty),
                ));
            }
            look.print(&doc);
        }
        OptimizeOutputFormat::Json => {
            let output = OptimizeJson {
                design_margin_percent: args.margin * 100.0,
                solution: &solution,
                monte_carlo: mc_results.as_ref(),
            };
            println!("{}", Envelope::new("optimize", output).to_json()?);
        }
    }

    Ok(())
}

/// The JSON document `tsi optimize --output json` prints: the solution's own
/// versioned report, plus the margin the problem asked for.
#[derive(Serialize)]
struct OptimizeJson<'a> {
    design_margin_percent: f64,
    #[serde(flatten)]
    solution: &'a Solution,
    #[serde(skip_serializing_if = "Option::is_none")]
    monte_carlo: Option<&'a MonteCarloResults>,
}

/// Turn an optimization error into a message with advice about which flags
/// to change. The library reports causes; suggesting flags is the CLI's job.
fn explain(error: &OptimizeError) -> String {
    let cause = match error {
        OptimizeError::Infeasible(cause) => cause,
        OptimizeError::TooManyCombinations { .. } => {
            return with_suggestions(
                error.to_string(),
                &[
                    "Pin engines to stages with --stage1-engine and --stage2-engine".into(),
                    "Offer fewer engines with --engine".into(),
                    "Allow fewer stages with --max-stages".into(),
                ],
            )
        }
        _ => return error.to_string(),
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
    with_suggestions(error.to_string(), &suggestions)
}

fn with_suggestions(mut message: String, suggestions: &[String]) -> String {
    if !suggestions.is_empty() {
        message.push_str("\n\nSuggestions:");
        for s in suggestions {
            message.push_str("\n  - ");
            message.push_str(s);
        }
    }
    message
}

/// Turn a problem the library rejected into a message about the flag that
/// set it. The library checks; the CLI knows which flag is which.
fn explain_problem(error: &ProblemError) -> String {
    let message = match error {
        ProblemError::InvalidPayload(m) => {
            format!("--payload must be a positive number, got {}", m.as_kg())
        }
        ProblemError::InvalidDeltaV(v) => {
            format!("--target-dv must be a positive number, got {}", v.as_mps())
        }
        ProblemError::InvalidStageCount { requested, max } => {
            format!("--stages {requested} is out of range: allowed 1 to {max} (raise --max-stages)")
        }
        ProblemError::Constraint(c) => match c {
            ConstraintError::InvalidLiftoffTwr(r) => format!(
                "--min-twr must be at least 1.0 to leave the pad, got {}",
                r.as_f64()
            ),
            ConstraintError::InvalidStageTwr(r) => {
                format!("--min-upper-twr must be positive, got {}", r.as_f64())
            }
            ConstraintError::ZeroStages => "--max-stages must be at least 1".into(),
            ConstraintError::ZeroEngines => "--max-engines must be at least 1".into(),
            ConstraintError::InvalidStructuralRatio(r) => format!(
                "--structural-ratio values must be between 0 and 1, got {}",
                r.as_f64()
            ),
            ConstraintError::InvalidMargin(r) => {
                format!(
                    "--margin must be zero or positive, got {}%",
                    r.as_f64() * 100.0
                )
            }
            other => other.to_string(),
        },
        other => other.to_string(),
    };
    format!("Invalid arguments:\n  - {message}")
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
///
/// Rayon workers report from many threads at once, so drawing happens under a
/// lock and only ever moves forward.
struct StderrProgress {
    style: ProgressStyle,
    state: Mutex<ProgressState>,
}

#[derive(Clone, Copy)]
enum ProgressStyle {
    Search,
    MonteCarlo,
}

#[derive(Default)]
struct ProgressState {
    total: u64,
    /// Highest percentage drawn so far in this phase
    shown: Option<u64>,
}

impl StderrProgress {
    fn search() -> Self {
        Self::new(ProgressStyle::Search)
    }

    fn monte_carlo() -> Self {
        Self::new(ProgressStyle::MonteCarlo)
    }

    fn new(style: ProgressStyle) -> Self {
        Self {
            style,
            state: Mutex::new(ProgressState::default()),
        }
    }

    fn draw(&self, percent: u64, done: u64, total: u64) {
        match self.style {
            ProgressStyle::Search => eprint!("\r  Searching... {percent}%"),
            ProgressStyle::MonteCarlo => eprint!("\rMonte Carlo: {percent}% ({done}/{total})"),
        }
        let _ = io::stderr().flush();
    }
}

impl Progress for StderrProgress {
    fn start(&self, phase: &str, total: u64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        *state = ProgressState { total, shown: None };
        if let ProgressStyle::Search = self.style {
            eprintln!("  {phase}");
        }
    }

    fn advance(&self, done: u64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.total == 0 {
            return;
        }
        let percent = done.min(state.total) * 100 / state.total;
        if state.shown.is_none_or(|shown| percent > shown) {
            state.shown = Some(percent);
            self.draw(percent, done, state.total);
        }
    }

    fn finish(&self) {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.total > 0 && state.shown != Some(100) {
            self.draw(100, state.total, state.total);
        }
        eprintln!();
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

    let thrust_kn: f64 = parts[1].parse().map_err(|_| {
        anyhow::anyhow!(
            "Invalid thrust value '{}' in custom engine '{}'.\n\
            Thrust should be in kilonewtons (e.g., 2000 for 2000 kN)",
            parts[1],
            name
        )
    })?;

    let isp_s: f64 = parts[2].parse().map_err(|_| {
        anyhow::anyhow!(
            "Invalid ISP value '{}' in custom engine '{}'.\n\
            ISP should be in seconds (e.g., 350 for 350s)",
            parts[2],
            name
        )
    })?;

    let mass_kg: f64 = parts[3].parse().map_err(|_| {
        anyhow::anyhow!(
            "Invalid mass value '{}' in custom engine '{}'.\n\
            Mass should be in kilograms (e.g., 1500 for 1500 kg)",
            parts[3],
            name
        )
    })?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn too_many_combinations_comes_with_advice() {
        let message = explain(&OptimizeError::TooManyCombinations {
            stage_count: 3,
            combinations: 216_000,
            limit: 200_000,
        });
        assert!(message.contains("--stage1-engine"), "{message}");
        assert!(message.contains("--engine"), "{message}");
    }

    #[test]
    fn problem_errors_name_their_flag() {
        let message = explain_problem(&ProblemError::Constraint(
            ConstraintError::InvalidLiftoffTwr(Ratio::new(0.5)),
        ));
        assert!(message.contains("--min-twr"), "{message}");
        let message = explain_problem(&ProblemError::InvalidPayload(Mass::kg(f64::NAN)));
        assert!(message.contains("--payload"), "{message}");
    }

    #[test]
    fn progress_only_moves_forward_and_ignores_advance_before_start() {
        let progress = StderrProgress::search();
        progress.advance(5); // before start: no total, nothing to divide by
        progress.start("test", 100);
        progress.advance(60);
        progress.advance(40); // a slower thread reporting late
        let shown = progress.state.lock().unwrap().shown;
        assert_eq!(shown, Some(60));
    }
}
