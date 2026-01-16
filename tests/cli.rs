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
        .args([
            "optimize",
            "--target-dv",
            "9400",
            "--engine",
            "raptor-2",
        ])
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
