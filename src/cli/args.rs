use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "tsi")]
#[command(about = "Rocket staging optimizer")]
#[command(version)]
#[command(after_help = "\
Examples:
  tsi calculate --engine raptor-2 --propellant-mass 100000
  tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
  tsi engines --propellant methane")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Calculate delta-v for a single stage
    #[command(after_help = "\
Examples:
  tsi calculate --engine raptor-2 --propellant-mass 100000
  tsi calculate --engine merlin-1d --engine-count 9 --propellant-mass 400000
  tsi calculate --isp 311 --mass-ratio 3.5
  tsi calculate --isp 350 --wet-mass 100000 --dry-mass 10000")]
    Calculate(CalculateArgs),

    /// Optimize staging for a rocket
    #[command(after_help = "\
Examples:
  tsi optimize --payload 5000 --target-dv 9400 --engine raptor-2
  tsi optimize --payload 1000 --target-dv 8000 --engine merlin-1d --min-twr 1.3
  tsi optimize --payload 10000 --target-dv 9400 --engine raptor-2 --output json")]
    Optimize(OptimizeArgs),

    /// List available rocket engines
    #[command(after_help = "\
Examples:
  tsi engines
  tsi engines --verbose
  tsi engines --propellant methane
  tsi engines --name raptor
  tsi engines --output json")]
    Engines(EnginesArgs),
}

#[derive(Args)]
pub struct CalculateArgs {
    /// Specific impulse in seconds (required if --engine not provided)
    #[arg(long)]
    pub isp: Option<f64>,

    /// Engine name from database (e.g., raptor-2, merlin-1d)
    #[arg(long)]
    pub engine: Option<String>,

    /// Number of engines (default: 1)
    #[arg(long, default_value = "1")]
    pub engine_count: u32,

    /// Mass ratio (wet mass / dry mass)
    #[arg(long, group = "mass_input")]
    pub mass_ratio: Option<f64>,

    /// Wet mass in kg (requires --dry-mass)
    #[arg(long, requires = "dry_mass")]
    pub wet_mass: Option<f64>,

    /// Dry mass in kg (requires --wet-mass)
    #[arg(long, requires = "wet_mass")]
    pub dry_mass: Option<f64>,

    /// Propellant mass in kg
    #[arg(long)]
    pub propellant_mass: Option<f64>,

    /// Thrust in Newtons (overrides engine thrust)
    #[arg(long)]
    pub thrust: Option<f64>,

    /// Structural mass ratio (structural mass / propellant mass)
    #[arg(long, default_value = "0.1")]
    pub structural_ratio: f64,

    /// Output format (default: pretty, compact: one-line summary)
    #[arg(short, long, value_enum, default_value = "pretty")]
    pub output: CalculateOutputFormat,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CalculateOutputFormat {
    /// Multi-line detailed output
    Pretty,
    /// One-line summary: Δv | Burn | TWR
    Compact,
}

impl CalculateArgs {
    /// Get the mass ratio, either directly or computed from wet/dry mass.
    pub fn get_mass_ratio(&self) -> Option<f64> {
        if let Some(ratio) = self.mass_ratio {
            Some(ratio)
        } else if let (Some(wet), Some(dry)) = (self.wet_mass, self.dry_mass) {
            Some(wet / dry)
        } else {
            None
        }
    }

    /// Get propellant mass for burn time calculation.
    pub fn get_propellant_mass(&self) -> Option<f64> {
        if let Some(prop) = self.propellant_mass {
            Some(prop)
        } else if let (Some(wet), Some(dry)) = (self.wet_mass, self.dry_mass) {
            Some(wet - dry)
        } else {
            None
        }
    }
}

#[derive(Args)]
pub struct EnginesArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value = "table")]
    pub output: OutputFormat,

    /// Filter by propellant type (e.g., loxch4, loxrp1, loxlh2)
    #[arg(short, long)]
    pub propellant: Option<String>,

    /// Filter by name (case-insensitive substring match)
    #[arg(short = 'n', long)]
    pub name: Option<String>,

    /// Show verbose output with sea-level values
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable table
    Table,
    /// JSON array
    Json,
}

/// Arguments for the optimize command.
#[derive(Args)]
pub struct OptimizeArgs {
    /// Payload mass in kg
    #[arg(short, long)]
    pub payload: f64,

    /// Target delta-v in m/s
    #[arg(short = 'd', long)]
    pub target_dv: f64,

    /// Engine name from database (comma-separated for multiple)
    #[arg(short, long)]
    pub engine: String,

    /// Force specific engine for first stage (overrides --engine for stage 1)
    #[arg(long)]
    pub stage1_engine: Option<String>,

    /// Force specific engine for second stage (overrides --engine for stage 2)
    #[arg(long)]
    pub stage2_engine: Option<String>,

    /// Minimum thrust-to-weight ratio at liftoff
    #[arg(long, default_value = "1.2")]
    pub min_twr: f64,

    /// Minimum TWR for upper stages (can be < 1.0 in vacuum)
    #[arg(long, default_value = "0.5")]
    pub min_upper_twr: f64,

    /// Maximum number of stages
    #[arg(long, default_value = "2")]
    pub max_stages: u32,

    /// Structural mass ratio (structural / propellant)
    #[arg(long, default_value = "0.08")]
    pub structural_ratio: f64,

    /// Use sea-level thrust/ISP for first stage TWR calculation
    #[arg(long)]
    pub sea_level: bool,

    /// Surface gravity (affects TWR calculation)
    #[arg(long, value_enum, default_value = "earth")]
    pub gravity: Gravity,

    /// Optimizer algorithm (auto-selects if not specified)
    #[arg(long, value_enum, default_value = "auto")]
    pub optimizer: OptimizerChoice,

    /// Hide progress indicator (useful for scripts)
    #[arg(long)]
    pub quiet: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "pretty")]
    pub output: OptimizeOutputFormat,

    /// Run Monte Carlo uncertainty analysis with N iterations
    #[arg(long, value_name = "N")]
    pub monte_carlo: Option<u64>,

    /// Uncertainty level for Monte Carlo (low, default, high, or custom ISP%)
    #[arg(long, value_enum, default_value = "default")]
    pub uncertainty: UncertaintyLevel,

    /// Show ASCII rocket diagram
    #[arg(long)]
    pub diagram: bool,

    /// Show estimated atmospheric and gravity losses
    #[arg(long)]
    pub show_losses: bool,

    /// Define a custom engine inline (can be used multiple times)
    ///
    /// Format: name:thrust_kn:isp_s:mass_kg:propellant
    ///
    /// Example: --custom-engine "MyEngine:2000:350:1500:loxch4"
    ///
    /// Propellant types: loxrp1, loxlh2, loxch4, n2o4udmh, solid
    #[arg(long, value_name = "SPEC")]
    pub custom_engine: Vec<String>,
}

/// Uncertainty level for Monte Carlo analysis.
#[derive(Clone, Copy, ValueEnum, Default)]
pub enum UncertaintyLevel {
    /// No uncertainty (0%)
    None,
    /// Low uncertainty (ISP ±0.5%, thrust ±1%, structural ±3%)
    Low,
    /// Default uncertainty (ISP ±1%, thrust ±2%, structural ±5%)
    #[default]
    Default,
    /// High uncertainty (ISP ±2%, thrust ±3%, structural ±8%)
    High,
}

/// Optimizer algorithm to use.
#[derive(Clone, Copy, ValueEnum, Default)]
pub enum OptimizerChoice {
    /// Auto-select based on problem complexity (default)
    #[default]
    Auto,
    /// Analytical optimizer (fast, 2-stage single-engine only)
    Analytical,
    /// Brute force grid search (slower, handles any configuration)
    BruteForce,
}

/// Surface gravity for different planetary bodies.
#[derive(Clone, Copy, ValueEnum)]
pub enum Gravity {
    /// Earth: 9.80665 m/s²
    Earth,
    /// Mars: 3.72076 m/s²
    Mars,
    /// Moon: 1.62 m/s²
    Moon,
}

impl Gravity {
    /// Get the surface gravity in m/s².
    pub fn as_mps2(&self) -> f64 {
        match self {
            Gravity::Earth => 9.80665,
            Gravity::Mars => 3.72076,
            Gravity::Moon => 1.62,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OptimizeOutputFormat {
    /// Detailed pretty-printed output
    Pretty,
    /// JSON output
    Json,
}
