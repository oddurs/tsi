//! Integration tests for CLI commands.

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn tsi() -> Command {
    Command::cargo_bin("tsi").unwrap()
}

// ============================================================================
// Help and version
// ============================================================================

#[test]
fn help_displays() {
    tsi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rocket staging optimizer"));
}

#[test]
fn version_displays() {
    tsi()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn no_args_shows_help() {
    // clap shows help on stderr with exit code 2 when no subcommand is provided
    tsi()
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Usage:"))
        .stderr(predicate::str::contains("calculate"))
        .stderr(predicate::str::contains("engines"));
}

// ============================================================================
// Calculate command
// ============================================================================

#[test]
fn calculate_with_isp_and_mass_ratio() {
    tsi()
        .args(["calculate", "--isp", "311", "--mass-ratio", "3.5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Δv:"))
        .stdout(predicate::str::contains("m/s"));
}

#[test]
fn calculate_with_engine() {
    tsi()
        .args([
            "calculate",
            "--engine",
            "raptor-2",
            "--propellant-mass",
            "100000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Raptor-2"))
        .stdout(predicate::str::contains("Δv:"))
        .stdout(predicate::str::contains("LOX/CH4"));
}

#[test]
fn calculate_with_multiple_engines() {
    tsi()
        .args([
            "calculate",
            "--engine",
            "merlin-1d",
            "--engine-count",
            "9",
            "--propellant-mass",
            "400000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Merlin-1D (×9)"))
        .stdout(predicate::str::contains("Δv:"));
}

#[test]
fn calculate_with_wet_dry_mass() {
    tsi()
        .args([
            "calculate",
            "--isp",
            "311",
            "--wet-mass",
            "550000",
            "--dry-mass",
            "26000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Δv:"));
}

#[test]
fn calculate_missing_isp_and_engine_fails() {
    tsi()
        .args(["calculate", "--mass-ratio", "3.5"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Must provide either --engine or --isp",
        ));
}

#[test]
fn calculate_missing_mass_input_fails() {
    tsi()
        .args(["calculate", "--isp", "311"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Must provide"));
}

#[test]
fn calculate_unknown_engine_fails() {
    tsi()
        .args([
            "calculate",
            "--engine",
            "not-a-real-engine",
            "--propellant-mass",
            "1000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown engine"))
        .stderr(predicate::str::contains("tsi engines"));
}

#[test]
fn calculate_output_has_thousands_separators() {
    tsi()
        .args([
            "calculate",
            "--engine",
            "raptor-2",
            "--propellant-mass",
            "100000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("100,000 kg"))
        .stdout(predicate::str::contains("11,600 kg"));
}

// ============================================================================
// Engines command
// ============================================================================

#[test]
fn engines_lists_available() {
    tsi()
        .args(["engines"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Merlin-1D"))
        .stdout(predicate::str::contains("Raptor-2"))
        .stdout(predicate::str::contains("RS-25"));
}

#[test]
fn engines_shows_propellant_types() {
    tsi()
        .args(["engines"])
        .assert()
        .success()
        .stdout(predicate::str::contains("LOX/RP-1"))
        .stdout(predicate::str::contains("LOX/CH4"))
        .stdout(predicate::str::contains("LOX/LH2"));
}

#[test]
fn engines_json_output() {
    tsi()
        .args(["engines", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

#[test]
fn engines_json_is_valid() {
    let output = tsi()
        .args(["engines", "--output", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.is_array());
    assert!(json.as_array().unwrap().len() >= 10);
}

#[test]
fn engines_table_has_headers() {
    tsi()
        .args(["engines"])
        .assert()
        .success()
        .stdout(predicate::str::contains("NAME"))
        .stdout(predicate::str::contains("PROPELLANT"))
        .stdout(predicate::str::contains("THRUST"))
        .stdout(predicate::str::contains("ISP"))
        .stdout(predicate::str::contains("MASS"));
}

// ============================================================================
// Case insensitivity
// ============================================================================

#[test]
fn engine_name_case_insensitive() {
    // Should work with various cases
    tsi()
        .args([
            "calculate",
            "--engine",
            "RAPTOR-2",
            "--propellant-mass",
            "100000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Raptor-2"));

    tsi()
        .args([
            "calculate",
            "--engine",
            "Merlin-1D",
            "--propellant-mass",
            "100000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Merlin-1D"));
}

// ============================================================================
// Validation and error messages
// ============================================================================

#[test]
fn unknown_engine_suggests_alternatives() {
    tsi()
        .args([
            "calculate",
            "--engine",
            "raptor",
            "--propellant-mass",
            "100000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Did you mean:"))
        .stderr(predicate::str::contains("Raptor-2"));
}

#[test]
fn validation_reports_multiple_errors() {
    tsi()
        .args([
            "calculate",
            "--isp",
            "300",
            "--mass-ratio",
            "0.5",
            "--structural-ratio",
            "2.0",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid arguments:"))
        .stderr(predicate::str::contains("--mass-ratio"))
        .stderr(predicate::str::contains("--structural-ratio"));
}

#[test]
fn compact_output_one_line() {
    tsi()
        .args([
            "calculate",
            "--engine",
            "raptor-2",
            "--propellant-mass",
            "100000",
            "--output",
            "compact",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Δv:"))
        .stdout(predicate::str::contains("|"))
        .stdout(predicate::str::contains("TWR:"));
}

#[test]
fn engines_filter_by_propellant() {
    tsi()
        .args(["engines", "--propellant", "methane"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Raptor-2"))
        .stdout(predicate::str::contains("BE-4"))
        .stdout(predicate::str::contains("LOX/CH4"));
}

#[test]
fn engines_filter_by_name() {
    tsi()
        .args(["engines", "--name", "raptor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Raptor-2"))
        .stdout(predicate::str::contains("Raptor-Vacuum"));
}

#[test]
fn engines_verbose_shows_sl_values() {
    tsi()
        .args(["engines", "--verbose"])
        .assert()
        .success()
        .stdout(predicate::str::contains("THRUST(sl)"))
        .stdout(predicate::str::contains("ISP(sl)"));
}

// ============================================================================
// Optimize command
// ============================================================================

#[test]
fn optimize_basic() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Staging Optimization Complete"))
        .stdout(predicate::str::contains("STAGE 1"))
        .stdout(predicate::str::contains("STAGE 2"))
        .stdout(predicate::str::contains("Payload fraction"));
}

#[test]
fn optimize_json_output() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_mass_kg\""))
        .stdout(predicate::str::contains("\"stages\""))
        .stdout(predicate::str::contains("\"margin_mps\""));
}

#[test]
fn optimize_json_is_valid() {
    let output = tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--output",
            "json",
        ])
        .output()
        .expect("failed to run");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("invalid JSON output");
    assert!(json["total_mass_kg"].is_number());
    assert!(json["stages"].is_array());
    assert_eq!(json["stages"].as_array().unwrap().len(), 2);
}

#[test]
fn optimize_unknown_engine_fails() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "not-an-engine",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown engine"));
}

#[test]
fn optimize_missing_payload_fails() {
    tsi()
        .args(["optimize", "--target-dv", "9400", "--engine", "raptor-2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--payload"));
}

#[test]
fn optimize_with_custom_twr() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--min-twr",
            "1.3",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("STAGE 1"));
}

#[test]
fn optimize_with_merlin() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "10000",
            "--target-dv",
            "8000",
            "--engine",
            "merlin-1d",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Merlin-1D"));
}

#[test]
fn optimize_engine_case_insensitive() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "RAPTOR-2",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Raptor-2"));
}

// ============================================================================
// Optimizer selection
// ============================================================================

#[test]
fn optimize_with_analytical_flag() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--optimizer",
            "analytical",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("STAGE 1"));
}

#[test]
fn optimize_with_brute_force_flag() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--optimizer",
            "brute-force",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("STAGE 1"));
}

#[test]
fn optimize_multi_engine_comma_separated() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9000",
            "--engine",
            "raptor-2,merlin-1d",
            "--optimizer",
            "brute-force",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Staging Optimization Complete"));
}

#[test]
fn optimize_json_includes_metadata() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"metadata\""))
        .stdout(predicate::str::contains("\"optimizer\""))
        .stdout(predicate::str::contains("\"iterations\""))
        .stdout(predicate::str::contains("\"runtime_ms\""));
}

#[test]
fn optimize_quiet_flag() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--optimizer",
            "brute-force",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("STAGE 1"));
}

#[test]
fn optimize_per_stage_engines() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--stage2-engine",
            "raptor-vacuum",
            "--optimizer",
            "brute-force",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("STAGE 1"));
}

// ============================================================================
// Monte Carlo Analysis
// ============================================================================

#[test]
fn optimize_monte_carlo_basic() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--monte-carlo",
            "50",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("MONTE CARLO ANALYSIS"))
        .stdout(predicate::str::contains("Success probability"));
}

#[test]
fn optimize_monte_carlo_with_uncertainty_level() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--monte-carlo",
            "30",
            "--uncertainty",
            "high",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("MONTE CARLO ANALYSIS"));
}

#[test]
fn optimize_monte_carlo_json_output() {
    let output = tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--monte-carlo",
            "20",
            "--output",
            "json",
        ])
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["monte_carlo"].is_object());
    assert!(json["monte_carlo"]["success_probability"].is_number());
    assert!(json["monte_carlo"]["delta_v"]["mean"].is_number());
}

#[test]
fn optimize_monte_carlo_shows_confidence_intervals() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--monte-carlo",
            "100",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Confidence Intervals"))
        .stdout(predicate::str::contains("5th %ile"))
        .stdout(predicate::str::contains("95th %ile"));
}

#[test]
fn optimize_uncertainty_none() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--monte-carlo",
            "10",
            "--uncertainty",
            "none",
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("100.0%")); // 100% success with no uncertainty
}

// ============================================================================
// Diagram output
// ============================================================================

#[test]
fn optimize_diagram_shows_ascii_rocket() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--diagram",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Payload"))
        .stdout(predicate::str::contains("S1"))
        .stdout(predicate::str::contains("S2"))
        .stdout(predicate::str::contains("/\\")) // Nose cone
        .stdout(predicate::str::contains("\\/")); // Nozzles
}

// ============================================================================
// Atmospheric losses output
// ============================================================================

#[test]
fn optimize_show_losses() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--show-losses",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ESTIMATED LOSSES"))
        .stdout(predicate::str::contains("Gravity losses"))
        .stdout(predicate::str::contains("Drag losses"))
        .stdout(predicate::str::contains("Steering losses"))
        .stdout(predicate::str::contains("LEO orbital v"));
}

// ============================================================================
// Custom engines
// ============================================================================

#[test]
fn optimize_custom_engine() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--custom-engine",
            "TestEngine:2500:360:1800:loxch4",
            "--engine",
            "TestEngine",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("TestEngine"))
        .stdout(predicate::str::contains("LOX/CH4"));
}

#[test]
fn optimize_custom_engine_invalid_format() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--custom-engine",
            "BadFormat",
            "--engine",
            "BadFormat",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid custom engine format"));
}

#[test]
fn optimize_custom_engine_invalid_propellant() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--custom-engine",
            "TestEngine:2500:360:1800:invalid",
            "--engine",
            "TestEngine",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown propellant"));
}

#[test]
fn optimize_mix_custom_and_database_engines() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--custom-engine",
            "CustomBooster:3000:320:2000:loxrp1",
            "--engine",
            "CustomBooster,raptor-2",
            "--optimizer",
            "brute-force",
            "--quiet",
        ])
        .assert()
        .success();
}

// ============================================================================
// Shell completions
// ============================================================================

#[test]
fn completions_bash() {
    tsi()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_tsi()"))
        .stdout(predicate::str::contains("COMPREPLY"));
}

#[test]
fn completions_zsh() {
    tsi()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef tsi"));
}

#[test]
fn completions_fish() {
    tsi()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c tsi"));
}

#[test]
fn completions_man_page() {
    tsi()
        .args(["completions", "--man"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".TH tsi"))
        .stdout(predicate::str::contains("SYNOPSIS"));
}

// ============================================================================
// v0.7: stage counts, pinning, gravity, TWR reporting, input validation
// ============================================================================

/// Run `tsi optimize` with JSON output and parse the result.
fn optimize_json(args: &[&str]) -> serde_json::Value {
    let output = tsi()
        .arg("optimize")
        .args(args)
        .args(["--output", "json", "--quiet"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "tsi failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("invalid JSON output")
}

fn stage_engines(json: &serde_json::Value) -> Vec<String> {
    json["stages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["engine"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn max_stages_is_a_maximum_not_an_exact_count() {
    // 3 km/s is well within one stage's reach; a second stage is dead weight.
    let json = optimize_json(&[
        "--payload",
        "1000",
        "--target-dv",
        "3000",
        "--engine",
        "raptor-2",
        "--max-stages",
        "3",
    ]);
    assert_eq!(json["stages"].as_array().unwrap().len(), 1);
}

#[test]
fn stages_flag_fixes_the_count() {
    let json = optimize_json(&[
        "--payload",
        "1000",
        "--target-dv",
        "3000",
        "--engine",
        "raptor-2",
        "--stages",
        "2",
    ]);
    assert_eq!(json["stages"].as_array().unwrap().len(), 2);
}

#[test]
fn stage1_engine_is_pinned() {
    // v0.6 added the pinned engine to the pool and then ignored it.
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "raptor-2",
        "--stage1-engine",
        "merlin-1d",
    ]);
    assert_eq!(stage_engines(&json), ["Merlin-1D", "Raptor-2"]);
}

#[test]
fn stage2_engine_is_pinned() {
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "merlin-1d",
        "--stage2-engine",
        "raptor-2",
    ]);
    assert_eq!(stage_engines(&json), ["Merlin-1D", "Raptor-2"]);
}

#[test]
fn pinned_engines_work_with_brute_force() {
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "raptor-2",
        "--stage1-engine",
        "merlin-1d",
        "--optimizer",
        "brute-force",
    ]);
    assert_eq!(stage_engines(&json)[0], "Merlin-1D");
}

#[test]
fn gravity_reaches_the_optimizer() {
    // On Mars a Raptor lifts about 2.6× as much, so the booster needs fewer
    // engines, and with no air to fly through the rocket is lighter.
    let args = [
        "--payload",
        "50000",
        "--target-dv",
        "4500",
        "--engine",
        "raptor-2",
    ];
    let earth = optimize_json(&args);
    let mut mars_args = args.to_vec();
    mars_args.extend(["--gravity", "mars"]);
    let mars = optimize_json(&mars_args);

    let engines = |j: &serde_json::Value| j["stages"][0]["engine_count"].as_u64().unwrap();
    assert!(engines(&mars) < engines(&earth));
    assert!(mars["total_mass_kg"].as_f64() < earth["total_mass_kg"].as_f64());
    assert_eq!(mars["booster_isp_model"], "vacuum");
    assert_eq!(earth["booster_isp_model"], "ascent-averaged");
}

#[test]
fn sea_level_flag_is_accepted_but_deprecated() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--sea-level",
            "--quiet",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("deprecated"));
}

#[test]
fn json_distinguishes_liftoff_and_ignition_twr() {
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "raptor-2",
    ]);
    let s1 = &json["stages"][0];
    let s2 = &json["stages"][1];
    let liftoff = s1["twr_liftoff"].as_f64().unwrap();
    let ignition = s1["twr_ignition"].as_f64().unwrap();
    // Liftoff uses sea-level thrust, so it is lower than the vacuum figure.
    assert!(
        liftoff >= 1.2 && liftoff < ignition,
        "{liftoff} vs {ignition}"
    );
    assert!(s2["twr_liftoff"].is_null());
    assert!(s2["twr_ignition"].as_f64().unwrap() >= 0.5);
    assert!(s1.get("twr").is_none());
}

#[test]
fn pretty_output_labels_twr() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("at liftoff"))
        .stdout(predicate::str::contains("at ignition"));
}

#[test]
fn zero_margin_by_default() {
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "raptor-2",
    ]);
    assert!(json["margin_mps"].as_f64().unwrap().abs() < 0.01);
}

#[test]
fn margin_flag_accepts_percent() {
    for margin in ["2", "2%"] {
        let json = optimize_json(&[
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--margin",
            margin,
        ]);
        let m = json["margin_mps"].as_f64().unwrap();
        assert!((m - 188.0).abs() < 0.01, "--margin {margin}: {m}");
        assert_eq!(json["design_margin_percent"].as_f64(), Some(2.0));
    }
}

#[test]
fn max_engines_flag_allows_super_heavy() {
    let base = [
        "--payload",
        "100000",
        "--target-dv",
        "9400",
        "--engine",
        "raptor-2",
    ];
    tsi()
        .arg("optimize")
        .args(base)
        .arg("--quiet")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--max-engines"));
    let mut args = base.to_vec();
    args.extend(["--max-engines", "40"]);
    let json = optimize_json(&args);
    assert!(json["stages"][0]["engine_count"].as_u64().unwrap() > 9);
}

#[test]
fn nan_and_infinite_inputs_are_rejected_by_name() {
    for (flag, value) in [
        ("--payload", "NaN"),
        ("--payload", "inf"),
        ("--target-dv", "NaN"),
        ("--target-dv", "-inf"),
    ] {
        let mut args = vec![
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
        ];
        let at = args.iter().position(|a| *a == flag).unwrap();
        args[at + 1] = value;
        tsi()
            .args(&args)
            .assert()
            .failure()
            .stderr(predicate::str::contains(flag));
    }
}

#[test]
fn monte_carlo_seed_is_reproducible() {
    let run = || {
        optimize_json(&[
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--monte-carlo",
            "500",
            "--seed",
            "42",
        ])["monte_carlo"]
            .clone()
    };
    let (a, b) = (run(), run());
    assert_eq!(a["seed"], 42);
    assert_eq!(a["delta_v"], b["delta_v"]);
    assert_eq!(a["successes"], b["successes"]);
}

#[test]
fn monte_carlo_stresses_the_reported_design() {
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "raptor-2",
        "--monte-carlo",
        "500",
    ]);
    let mc = &json["monte_carlo"];
    assert_eq!(mc["design_total_mass_kg"], json["total_mass_kg"]);
    // Zero margin: builds fall short about half the time.
    let p = mc["success_probability"].as_f64().unwrap();
    assert!((0.3..0.7).contains(&p), "{p}");
}

#[test]
fn calculate_rejects_nan_and_infinity() {
    for args in [
        ["--isp", "NaN", "--mass-ratio", "3"],
        ["--isp", "inf", "--mass-ratio", "3"],
        ["--isp", "300", "--mass-ratio", "NaN"],
    ] {
        tsi()
            .arg("calculate")
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("Invalid arguments"));
    }
}

#[test]
fn custom_engine_rejects_nan() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "x",
            "--custom-engine",
            "x:NaN:350:1500:loxch4",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Thrust must be positive"));
}

// ============================================================================
// v0.7 review fixes
// ============================================================================

#[test]
fn vacuum_only_engines_get_a_sea_level_error() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "rl-10c",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("can fly from sea level"));
}

#[test]
fn vacuum_engine_pinned_to_booster_is_rejected() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--stage1-engine",
            "rl-10c",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no sea-level rating"));
}

#[test]
fn upper_stage_twr_error_names_the_right_flag() {
    for optimizer in ["analytical", "brute-force"] {
        tsi()
            .args([
                "optimize",
                "--payload",
                "5000",
                "--target-dv",
                "9400",
                "--engine",
                "raptor-2",
                "--min-upper-twr",
                "200",
                "--optimizer",
                optimizer,
                "--quiet",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("--min-upper-twr"));
    }
}

#[test]
fn engine_limit_error_names_the_booster() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "100000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--quiet",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "stage 1 needs more than 9 engines",
        ));
}

#[test]
fn monte_carlo_without_uncertainty_counts_every_build() {
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "raptor-2",
        "--monte-carlo",
        "200",
        "--uncertainty",
        "none",
    ]);
    assert_eq!(json["monte_carlo"]["total_runs"], 200);
    assert_eq!(json["monte_carlo"]["successes"], 200);
}

#[test]
fn monte_carlo_on_the_moon_uses_lunar_gravity() {
    // Liftoff TWR 1.3 at 1.62 m/s² is only 0.2 at g₀. Monte Carlo used to
    // judge builds at g₀ and call every one "too heavy to lift off".
    let json = optimize_json(&[
        "--payload",
        "300000",
        "--target-dv",
        "4000",
        "--engine",
        "merlin-1d",
        "--gravity",
        "moon",
        "--max-engines",
        "30",
        "--margin",
        "3",
        "--monte-carlo",
        "200",
        "--seed",
        "1",
    ]);
    assert_eq!(json["monte_carlo"]["failures"], 0);
    assert!(json["monte_carlo"]["success_probability"].as_f64().unwrap() > 0.9);
    assert!(json["stages"][0]["twr_liftoff"].as_f64().unwrap() >= 1.2);
}

// ============================================================================
// JSON snapshots (schema_version 1)
// ============================================================================

/// Round every number to six significant figures, zero rounding noise, and
/// drop timings and iteration counts, so that snapshots survive last-digit
/// floating-point differences between platforms and wall-clock noise.
fn stable(mut value: serde_json::Value) -> serde_json::Value {
    fn walk(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Number(n) if n.is_f64() => {
                if let Some(x) = n.as_f64() {
                    if x.abs() < 1e-6 {
                        *v = serde_json::json!(0.0);
                    } else if x.fract() != 0.0 {
                        let digits = 6 - x.abs().log10().ceil() as i32;
                        let scale = 10f64.powi(digits);
                        let rounded = (x * scale).round() / scale;
                        *v = serde_json::json!(rounded);
                    }
                }
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(walk),
            serde_json::Value::Object(map) => {
                for key in ["runtime_ms", "iterations"] {
                    if map.contains_key(key) {
                        map.insert(key.into(), serde_json::json!(format!("[{key}]")));
                    }
                }
                map.values_mut().for_each(walk);
            }
            _ => {}
        }
    }
    walk(&mut value);
    value
}

#[test]
fn snapshot_optimize_json() {
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "raptor-2",
    ]);
    assert_eq!(json["schema_version"], 1);
    insta::assert_json_snapshot!("optimize_raptor_leo", stable(json));
}

#[test]
fn snapshot_optimize_json_with_monte_carlo() {
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "merlin-1d,rl-10c",
        "--max-stages",
        "3",
        "--margin",
        "3",
        "--monte-carlo",
        "500",
        "--seed",
        "7",
    ]);
    insta::assert_json_snapshot!("optimize_mixed_engines_monte_carlo", stable(json));
}

#[test]
fn structural_ratio_accepts_per_stage_values() {
    let json = optimize_json(&[
        "--payload",
        "5000",
        "--target-dv",
        "9400",
        "--engine",
        "raptor-2",
        "--structural-ratio",
        "0.04,0.06",
    ]);
    let ratio = |i: usize| {
        let s = &json["stages"][i];
        s["structural_mass_kg"].as_f64().unwrap() / s["propellant_kg"].as_f64().unwrap()
    };
    assert!((ratio(0) - 0.04).abs() < 1e-9, "{}", ratio(0));
    assert!((ratio(1) - 0.06).abs() < 1e-9, "{}", ratio(1));
}

#[test]
fn structural_ratio_list_rejects_garbage() {
    tsi()
        .args([
            "optimize",
            "--payload",
            "5000",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
            "--structural-ratio",
            "0.04,abc",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a ratio"));
}
