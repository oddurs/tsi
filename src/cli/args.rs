use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "tsi")]
#[command(about = "Rocket staging optimizer")]
#[command(version)]
#[command(after_help = "\
Examples:
  tsi calculate --engine raptor-2 --propellant-mass 100000
  tsi engines --propellant methane
  tsi engines --verbose")]
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
