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
    let rocket = &solution.rocket;
    let stages = rocket.stages();

    print_header("tsi — Staging Optimization Complete");

    println!();
    print_summary(
        &format!("Target Δv:  {} m/s", format_thousands_f64(target_dv)),
        &format!("Payload:  {} kg", format_thousands_f64(payload_kg)),
    );
    print_summary(
        &format!(
            "Solution:   {}-stage",
            rocket.stage_count()
        ),
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
        let stage_twr = rocket.stage_twr(i);

        print_stage_box(
            stage_num,
            stage_name,
            &stage.engine().name,
            stage.engine_count(),
            stage.propellant_mass().as_kg(),
            stage.engine().propellant.name(),
            stage.dry_mass().as_kg(),
            stage_dv.as_mps(),
            &format!("{}", stage.burn_time()),
            stage_twr.as_f64(),
        );
    }

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
    println!();

    print_footer();
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
