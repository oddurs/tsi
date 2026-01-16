//! Terminal output formatting with box drawing characters.
//!
//! Provides pretty-printed output for optimization results using
//! Unicode box drawing characters for a clean, professional look.
//!
//! # Box Drawing Characters
//!
//! Uses the following Unicode characters for boxes:
//! - `═` Double horizontal line
//! - `─` Single horizontal line
//! - `│` Single vertical line
//! - `┌┐└┘` Corners
//! - `├┤` T-junctions

use crate::optimizer::Solution;
use crate::units::{format_thousands_f64, Velocity};

/// Width of the output box (interior content width)
const BOX_WIDTH: usize = 61;

/// Print a double-line header.
pub fn print_header(title: &str) {
    println!();
    println!("{}", "═".repeat(BOX_WIDTH + 2));
    println!("  {}", title);
    println!("{}", "═".repeat(BOX_WIDTH + 2));
}

/// Print a double-line footer.
pub fn print_footer() {
    println!("{}", "═".repeat(BOX_WIDTH + 2));
    println!();
}

/// Print a summary line with two columns.
pub fn print_summary(left: &str, right: &str) {
    println!("  {}    {}", left, right);
}

/// Print a stage box.
#[allow(clippy::too_many_arguments)]
pub fn print_stage_box(
    stage_num: usize,
    stage_name: &str,
    engine_name: &str,
    engine_count: u32,
    propellant_kg: f64,
    propellant_type: &str,
    dry_mass_kg: f64,
    delta_v_mps: f64,
    burn_time: &str,
    twr: f64,
) {
    println!("  ┌{}┐", "─".repeat(BOX_WIDTH));

    // Stage header
    let header = format!("STAGE {} ({})", stage_num, stage_name);
    println!("  │  {:<width$}│", header, width = BOX_WIDTH - 2);

    // Engine
    let engine = format!("Engine:     {} (×{})", engine_name, engine_count);
    println!("  │  {:<width$}│", engine, width = BOX_WIDTH - 2);

    // Propellant
    let prop = format!(
        "Propellant: {} kg ({})",
        format_thousands_f64(propellant_kg),
        propellant_type
    );
    println!("  │  {:<width$}│", prop, width = BOX_WIDTH - 2);

    // Dry mass
    let dry = format!("Dry mass:   {} kg", format_thousands_f64(dry_mass_kg));
    println!("  │  {:<width$}│", dry, width = BOX_WIDTH - 2);

    // Delta-v
    let dv = format!("Δv:         {} m/s", format_thousands_f64(delta_v_mps));
    println!("  │  {:<width$}│", dv, width = BOX_WIDTH - 2);

    // Burn time
    let bt = format!("Burn time:  {}", burn_time);
    println!("  │  {:<width$}│", bt, width = BOX_WIDTH - 2);

    // TWR
    let twr_line = format!("TWR:        {:.2}", twr);
    println!("  │  {:<width$}│", twr_line, width = BOX_WIDTH - 2);

    println!("  └{}┘", "─".repeat(BOX_WIDTH));
}

/// Print the complete optimization solution.
pub fn print_solution(target_dv: f64, payload_kg: f64, solution: &Solution) {
    print_solution_with_options(target_dv, payload_kg, solution, 9.80665, false);
}

/// Print the complete optimization solution with gravity and sea-level options.
pub fn print_solution_with_options(
    target_dv: f64,
    payload_kg: f64,
    solution: &Solution,
    gravity: f64,
    sea_level: bool,
) {
    let rocket = &solution.rocket;
    let stages = rocket.stages();

    print_header("tsi — Staging Optimization Complete");

    println!();
    print_summary(
        &format!("Target Δv:  {} m/s", format_thousands_f64(target_dv)),
        &format!("Payload:  {} kg", format_thousands_f64(payload_kg)),
    );
    print_summary(
        &format!("Solution:   {}-stage", rocket.stage_count()),
        &format!(
            "Total mass:  {} kg",
            format_thousands_f64(rocket.total_mass().as_kg())
        ),
    );
    println!();

    // Print stages from top to bottom (reverse order for display)
    for (i, stage) in stages.iter().enumerate().rev() {
        let stage_num = i + 1;
        let stage_name = if i == 0 { "booster" } else { "upper" };
        let stage_dv = rocket.stage_delta_v(i);

        // Calculate TWR based on options
        let stage_twr = if i == 0 && sea_level {
            // Use sea-level thrust for first stage
            let sl_thrust = stage.engine().thrust_sl() * stage.engine_count();
            let mass_above = rocket.mass_above_stage(i);
            let total_mass = stage.wet_mass() + mass_above;
            sl_thrust.as_newtons() / (total_mass.as_kg() * gravity)
        } else {
            // Use vacuum thrust (default), adjusted for gravity
            let vac_thrust = stage.engine().thrust_vac() * stage.engine_count();
            let mass_above = rocket.mass_above_stage(i);
            let total_mass = stage.wet_mass() + mass_above;
            vac_thrust.as_newtons() / (total_mass.as_kg() * gravity)
        };

        let twr_label = if i == 0 && sea_level { "TWR (SL)" } else { "TWR (vac)" };

        print_stage_box_with_twr_label(
            stage_num,
            stage_name,
            &stage.engine().name,
            stage.engine_count(),
            stage.propellant_mass().as_kg(),
            stage.engine().propellant.name(),
            stage.dry_mass().as_kg(),
            stage_dv.as_mps(),
            &format!("{}", stage.burn_time()),
            stage_twr,
            twr_label,
        );
    }

    // Summary statistics
    let total_propellant: f64 = stages.iter().map(|s| s.propellant_mass().as_kg()).sum();
    let total_dry: f64 = stages.iter().map(|s| s.dry_mass().as_kg()).sum();
    let total_burn_time: f64 = stages.iter().map(|s| s.burn_time().as_seconds()).sum();

    println!();
    println!(
        "  Total propellant:  {} kg",
        format_thousands_f64(total_propellant)
    );
    println!(
        "  Total dry mass:    {} kg",
        format_thousands_f64(total_dry)
    );
    println!("  Total burn time:   {:.0}s", total_burn_time);
    println!();
    println!(
        "  Payload fraction:  {:.2}%",
        solution.payload_fraction_percent()
    );
    println!(
        "  Delta-v margin:    +{} m/s ({:.1}%)",
        format_thousands_f64(solution.margin.as_mps()),
        solution.margin_percent(Velocity::mps(target_dv))
    );

    // Show gravity note if not Earth
    if (gravity - 9.80665).abs() > 0.01 {
        println!();
        println!("  Note: TWR calculated for g = {:.2} m/s²", gravity);
    }

    // Optimizer metadata
    if !solution.optimizer_name.is_empty() {
        println!();
        let runtime = if solution.runtime.as_millis() > 0 {
            format!(" in {}ms", solution.runtime.as_millis())
        } else {
            String::new()
        };
        println!(
            "  Optimizer: {} ({} configs{})",
            solution.optimizer_name, solution.iterations, runtime
        );
    }

    println!();

    print_footer();
}

/// Print a stage box with custom TWR label.
#[allow(clippy::too_many_arguments)]
fn print_stage_box_with_twr_label(
    stage_num: usize,
    stage_name: &str,
    engine_name: &str,
    engine_count: u32,
    propellant_kg: f64,
    propellant_type: &str,
    dry_mass_kg: f64,
    delta_v_mps: f64,
    burn_time: &str,
    twr: f64,
    twr_label: &str,
) {
    println!("  ┌{}┐", "─".repeat(BOX_WIDTH));

    // Stage header
    let header = format!("STAGE {} ({})", stage_num, stage_name);
    println!("  │  {:<width$}│", header, width = BOX_WIDTH - 2);

    // Engine
    let engine = format!("Engine:     {} (×{})", engine_name, engine_count);
    println!("  │  {:<width$}│", engine, width = BOX_WIDTH - 2);

    // Propellant
    let prop = format!(
        "Propellant: {} kg ({})",
        format_thousands_f64(propellant_kg),
        propellant_type
    );
    println!("  │  {:<width$}│", prop, width = BOX_WIDTH - 2);

    // Dry mass
    let dry = format!("Dry mass:   {} kg", format_thousands_f64(dry_mass_kg));
    println!("  │  {:<width$}│", dry, width = BOX_WIDTH - 2);

    // Delta-v
    let dv = format!("Δv:         {} m/s", format_thousands_f64(delta_v_mps));
    println!("  │  {:<width$}│", dv, width = BOX_WIDTH - 2);

    // Burn time
    let bt = format!("Burn time:  {}", burn_time);
    println!("  │  {:<width$}│", bt, width = BOX_WIDTH - 2);

    // TWR with custom label
    let twr_line = format!("{}:   {:.2}", twr_label, twr);
    println!("  │  {:<width$}│", twr_line, width = BOX_WIDTH - 2);

    println!("  └{}┘", "─".repeat(BOX_WIDTH));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_width_is_reasonable() {
        // Box should be wide enough for typical content
        assert!(BOX_WIDTH >= 50);
        assert!(BOX_WIDTH <= 80);
    }
}
