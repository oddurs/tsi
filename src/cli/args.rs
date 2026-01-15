use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tsi")]
#[command(about = "Rocket staging optimizer")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Calculate delta-v for a single stage
    Calculate(CalculateArgs),
}

#[derive(Args)]
pub struct CalculateArgs {
    /// Specific impulse in seconds
    #[arg(long)]
    pub isp: f64,

    /// Mass ratio (wet mass / dry mass)
    #[arg(long, group = "mass_input")]
    pub mass_ratio: Option<f64>,

    /// Wet mass in kg (requires --dry-mass)
    #[arg(long, requires = "dry_mass")]
    pub wet_mass: Option<f64>,

    /// Dry mass in kg (requires --wet-mass)
    #[arg(long, requires = "wet_mass")]
    pub dry_mass: Option<f64>,

    /// Thrust in Newtons (enables burn time calculation)
    #[arg(long)]
    pub thrust: Option<f64>,

    /// Propellant mass in kg (for burn time, defaults to wet - dry)
    #[arg(long)]
    pub propellant_mass: Option<f64>,
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
