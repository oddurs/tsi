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
//!
//! # Monte Carlo Output
//!
//! Also provides formatting for Monte Carlo results:
//! - Success probability with confidence indicator
//! - Percentile values (5th, 50th, 95th)
//! - ASCII histogram of delta-v distribution

use tsiolkovsky::optimizer::{MonteCarloResults, Solution};
use tsiolkovsky::units::{format_thousands_f64, Velocity};

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

/// Print the complete optimization solution.
///
/// `design_margin` is the delta-v margin (as a fraction) the rocket was
/// sized for, shown next to the margin it achieved. TWR is quoted against the
/// rocket's own surface gravity.
pub fn print_solution(solution: &Solution, design_margin: f64) {
    let gravity = solution.rocket().surface_gravity();
    let rocket = solution.rocket();
    let stages = rocket.stages();
    let target_dv = solution.target_delta_v().as_mps();

    print_header("tsi — Staging Optimization Complete");

    println!();
    print_summary(
        &format!("Target Δv:  {} m/s", format_thousands_f64(target_dv)),
        &format!(
            "Payload:  {} kg",
            format_thousands_f64(rocket.payload().as_kg())
        ),
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

        // The first stage's TWR that matters is at liftoff, with the whole
        // stack on top; for upper stages, it's at their own ignition.
        let (stage_twr, twr_label) = if i == 0 {
            (rocket.liftoff_twr_in(gravity).as_f64(), "at liftoff")
        } else {
            (rocket.stage_twr_in(i, gravity).as_f64(), "at ignition")
        };

        print_stage_box(
            stage_num,
            stage_name,
            stage.engine().name(),
            stage.engine_count(),
            stage.propellant_mass().as_kg(),
            stage.engine().propellant().name(),
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
    let designed = if design_margin > 0.0 {
        format!(", designed for +{:.1}%", design_margin * 100.0)
    } else {
        String::new()
    };
    println!(
        "  Delta-v margin:    {:+.0} m/s ({:+.1}%{})",
        solution.margin().as_mps(),
        solution.margin_percent(),
        designed
    );
    let booster_model = rocket.booster_isp();
    let booster_isp = stages[0].engine().isp_for(booster_model);
    println!(
        "  Booster Isp:       {:.0}s ({}); upper stages use vacuum Isp",
        booster_isp.as_seconds(),
        booster_model.label()
    );

    // Show gravity note if not Earth
    if (gravity - 9.80665).abs() > 0.01 {
        println!();
        println!("  Note: TWR calculated for g = {:.2} m/s²", gravity);
    }

    // Optimizer metadata
    println!();
    let runtime = if solution.runtime().as_millis() > 0 {
        format!(" in {}ms", solution.runtime().as_millis())
    } else {
        String::new()
    };
    println!(
        "  Optimizer: {} ({} configs{})",
        solution.optimizer(),
        solution.iterations(),
        runtime
    );

    println!();

    print_footer();
}

/// Print one stage as a box.
#[allow(clippy::too_many_arguments)]
fn print_stage_box(
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
    let twr_line = format!("TWR:        {:.2} {}", twr, twr_label);
    println!("  │  {:<width$}│", twr_line, width = BOX_WIDTH - 2);

    println!("  └{}┘", "─".repeat(BOX_WIDTH));
}

// ============================================================================
// Monte Carlo Output
// ============================================================================

/// Print Monte Carlo results summary.
///
/// Shows success probability, confidence intervals, and histogram.
pub fn print_monte_carlo_results(results: &MonteCarloResults) {
    println!();
    println!("  ┌{}┐", "─".repeat(BOX_WIDTH));
    println!(
        "  │  {:<width$}│",
        "MONTE CARLO ANALYSIS",
        width = BOX_WIDTH - 2
    );
    println!("  └{}┘", "─".repeat(BOX_WIDTH));
    println!();

    // Success probability with status indicator
    let success_pct = results.success_probability() * 100.0;
    let status = if success_pct >= 95.0 {
        "HIGH CONFIDENCE"
    } else if success_pct >= 80.0 {
        "ADEQUATE"
    } else if success_pct >= 50.0 {
        "MARGINAL"
    } else {
        "LOW CONFIDENCE"
    };

    let design = results.design().rocket();
    println!(
        "  Design stressed:      {}-stage, {} kg (the solution above)",
        design.stage_count(),
        format_thousands_f64(design.total_mass().as_kg())
    );
    println!("  Success probability:  {:.1}% ({}) ", success_pct, status);
    println!(
        "  Builds:               {} ({} too heavy to lift off)",
        results.total_runs(),
        results.failures()
    );
    println!(
        "  Seed:                 {} (repeat with --seed {})",
        results.seed(),
        results.seed()
    );
    println!(
        "  Runtime:              {}ms",
        results.runtime().as_millis()
    );
    println!();

    // Delta-v statistics
    println!("  Delta-v Statistics:");
    println!(
        "    Mean:         {} m/s",
        format_thousands_f64(results.mean_delta_v())
    );
    println!(
        "    Std Dev:      {} m/s",
        format_thousands_f64(results.std_delta_v())
    );
    println!();

    // Percentiles
    println!("  Confidence Intervals:");
    println!(
        "    5th %ile:     {} m/s  (worst case)",
        format_thousands_f64(results.delta_v_percentile(5.0))
    );
    println!(
        "    50th %ile:    {} m/s  (median)",
        format_thousands_f64(results.delta_v_percentile(50.0))
    );
    println!(
        "    95th %ile:    {} m/s  (best case)",
        format_thousands_f64(results.delta_v_percentile(95.0))
    );
    println!();

    // Required margin for 95% confidence
    let margin_95 = results.required_margin(0.95);
    if margin_95 > 0.0 {
        println!(
            "  For 95% confidence, add {} m/s margin",
            format_thousands_f64(margin_95)
        );
    }

    // Warning for low success probability
    if success_pct < 95.0 {
        println!();
        println!("  ⚠ WARNING: Success probability is below 95%");
        println!("    Consider increasing target delta-v margin");
    }

    // Print histogram
    if !results.delta_v_samples().is_empty() {
        println!();
        print_histogram(results.delta_v_samples(), results.target_delta_v().as_mps());
    }
}

/// Print an ASCII histogram of delta-v distribution.
fn print_histogram(samples: &[f64], target: f64) {
    const HISTOGRAM_WIDTH: usize = 40;
    const NUM_BINS: usize = 20;

    if samples.is_empty() {
        return;
    }

    // Find range
    let min = samples.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    if range < 1.0 {
        // No meaningful distribution to show
        return;
    }

    // Create bins
    let bin_width = range / NUM_BINS as f64;
    let mut bins = [0usize; NUM_BINS];

    for &sample in samples {
        let bin = ((sample - min) / bin_width).floor() as usize;
        let bin = bin.min(NUM_BINS - 1);
        bins[bin] += 1;
    }

    // Find max bin for scaling
    let max_bin = *bins.iter().max().unwrap_or(&1);

    println!("  Delta-v Distribution:");
    println!("  ┌{}┐", "─".repeat(HISTOGRAM_WIDTH + 14));

    for (i, &count) in bins.iter().enumerate() {
        let bin_start = min + i as f64 * bin_width;
        let bar_len = (count * HISTOGRAM_WIDTH).checked_div(max_bin).unwrap_or(0);

        // Mark the bin containing the target
        let marker = if bin_start <= target && target < bin_start + bin_width {
            "◄"
        } else {
            " "
        };

        println!(
            "  │ {:>5.0} │{}{}│",
            bin_start,
            "█".repeat(bar_len),
            " ".repeat(HISTOGRAM_WIDTH - bar_len + 1),
        );

        // Add target line marker below the relevant bin
        if marker == "◄" && i < NUM_BINS - 1 {
            println!(
                "  │       │{:─<width$}┼ target",
                "",
                width = HISTOGRAM_WIDTH + 1
            );
        }
    }

    println!("  └{}┘", "─".repeat(HISTOGRAM_WIDTH + 14));
    println!(
        "        {} m/s {:>width$} {} m/s",
        format_thousands_f64(min),
        "",
        format_thousands_f64(max),
        width = HISTOGRAM_WIDTH - 10
    );
}

// ============================================================================
// Atmospheric Losses Output
// ============================================================================

use tsiolkovsky::physics::losses::{self, LossEstimate};

/// Print estimated atmospheric and gravity losses.
///
/// Shows a breakdown of estimated losses for Earth-to-LEO ascent.
pub fn print_losses(estimate: &LossEstimate, total_dv: Velocity) {
    let total_dv = total_dv.as_mps();
    println!();
    println!("  ┌{}┐", "─".repeat(BOX_WIDTH));
    println!(
        "  │  {:<width$}│",
        "ESTIMATED LOSSES (Earth to LEO)",
        width = BOX_WIDTH - 2
    );
    println!("  └{}┘", "─".repeat(BOX_WIDTH));
    println!();

    println!(
        "  Gravity losses:   {:>7} m/s",
        format_thousands_f64(estimate.gravity.as_mps())
    );
    println!(
        "  Drag losses:      {:>7} m/s",
        format_thousands_f64(estimate.drag.as_mps())
    );
    println!(
        "  Steering losses:  {:>7} m/s",
        format_thousands_f64(estimate.steering.as_mps())
    );
    println!("  {}", "─".repeat(30));
    println!(
        "  Total losses:     {:>7} m/s",
        format_thousands_f64(estimate.total().as_mps())
    );
    println!();

    // Calculate effective delta-v available for orbital velocity
    let effective_dv = total_dv - estimate.total().as_mps();
    let orbital_v_leo = losses::leo_orbital_velocity().as_mps();

    println!(
        "  Ideal delta-v:    {:>7} m/s",
        format_thousands_f64(total_dv)
    );
    println!(
        "  After losses:     {:>7} m/s",
        format_thousands_f64(effective_dv)
    );
    println!(
        "  LEO orbital v:    {:>7} m/s",
        format_thousands_f64(orbital_v_leo)
    );

    if effective_dv >= orbital_v_leo {
        let margin = effective_dv - orbital_v_leo;
        println!(
            "  Margin:           {:>+7} m/s (sufficient)",
            format_thousands_f64(margin)
        );
    } else {
        let shortfall = orbital_v_leo - effective_dv;
        println!(
            "  Shortfall:        {:>7} m/s (insufficient)",
            format_thousands_f64(shortfall)
        );
        println!();
        println!("  ⚠ WARNING: Insufficient delta-v for LEO insertion");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Box must be wide enough for typical content but fit an 80-column terminal.
    const _: () = assert!(BOX_WIDTH >= 50 && BOX_WIDTH <= 80);

    #[test]
    fn histogram_handles_empty() {
        // Should not panic on empty samples
        print_histogram(&[], 9400.0);
    }

    #[test]
    fn histogram_handles_single_value() {
        // Should not panic on single value
        print_histogram(&[9400.0], 9400.0);
    }
}
