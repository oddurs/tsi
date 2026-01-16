//! ASCII rocket diagram generation.
//!
//! Generates visual representations of rocket configurations using
//! ASCII art. Stage heights are scaled proportionally to propellant mass.
//!
//! # Example Output
//!
//! ```text
//!        /\
//!       /  \
//!      /    \       ← Payload (5,000 kg)
//!     /______\
//!     |      |
//!     | S2   |      ← Stage 2: Raptor-2 ×1
//!     |      |         100,000 kg propellant
//!     |______|
//!     |      |
//!     |      |
//!     | S1   |      ← Stage 1: Raptor-2 ×3
//!     |      |         400,000 kg propellant
//!     |      |
//!     |______|
//!      \    /
//!       \  /
//!        \/
//! ```

use crate::stage::Rocket;

/// Width of the rocket body in characters (interior).
const ROCKET_WIDTH: usize = 12;

/// Minimum height for any stage (in lines).
const MIN_STAGE_HEIGHT: usize = 3;

/// Maximum height for the largest stage (in lines).
const MAX_STAGE_HEIGHT: usize = 10;

/// Generate an ASCII diagram of the rocket.
///
/// The diagram shows:
/// - Payload as a nose cone at the top
/// - Each stage as a box, height proportional to propellant mass
/// - Stage numbers and engine names as labels
/// - A nozzle/fins section at the bottom
///
/// # Arguments
///
/// * `rocket` - The rocket configuration to visualize
/// * `payload_kg` - Payload mass in kg (for label)
///
/// # Returns
///
/// A vector of strings, each representing one line of the diagram.
pub fn generate_rocket_diagram(rocket: &Rocket, payload_kg: f64) -> Vec<String> {
    let mut lines = Vec::new();
    let stages = rocket.stages();

    if stages.is_empty() {
        return vec!["(empty rocket)".to_string()];
    }

    // Calculate stage heights based on propellant mass
    let max_propellant = stages
        .iter()
        .map(|s| s.propellant_mass().as_kg())
        .fold(0.0_f64, f64::max);

    let stage_heights: Vec<usize> = stages
        .iter()
        .map(|s| {
            let ratio = s.propellant_mass().as_kg() / max_propellant;
            let height = (ratio * MAX_STAGE_HEIGHT as f64).round() as usize;
            height.max(MIN_STAGE_HEIGHT)
        })
        .collect();

    // Draw nose cone (payload)
    lines.extend(draw_nose_cone(payload_kg));

    // Draw stages from top to bottom (reverse order - upper stages first)
    for (i, stage) in stages.iter().enumerate().rev() {
        let stage_num = i + 1;
        let height = stage_heights[i];
        let engine_name = &stage.engine().name;
        let engine_count = stage.engine_count();
        let propellant_kg = stage.propellant_mass().as_kg();

        lines.extend(draw_stage(
            stage_num,
            height,
            engine_name,
            engine_count,
            propellant_kg,
        ));
    }

    // Draw nozzles/fins at bottom
    lines.extend(draw_nozzles());

    lines
}

/// Draw the nose cone section representing the payload.
fn draw_nose_cone(payload_kg: f64) -> Vec<String> {
    let half_width = ROCKET_WIDTH / 2;
    let payload_label = format!("Payload ({} kg)", format_mass(payload_kg));

    vec![
        format!("{:>width$}", "/\\", width = half_width + 2),
        format!("{:>width$}", "/  \\", width = half_width + 3),
        format!(
            "{:>width$}   <- {}",
            "/    \\",
            payload_label,
            width = half_width + 4
        ),
        format!("{:>width$}", "/______\\", width = half_width + 5),
    ]
}

/// Draw a single stage as a box with label.
fn draw_stage(
    stage_num: usize,
    height: usize,
    engine_name: &str,
    engine_count: u32,
    propellant_kg: f64,
) -> Vec<String> {
    let mut lines = Vec::new();
    let half_width = ROCKET_WIDTH / 2;

    // Build labels for this stage
    let stage_label = format!("S{}", stage_num);
    let engine_label = format!("{} x{}", engine_name, engine_count);
    let prop_label = format!("{} kg", format_mass(propellant_kg));

    // Top border (only if first stage drawn, otherwise stages connect)
    // We don't draw top border - previous section provides it

    // Stage body
    let middle_line = height / 2;
    for line_idx in 0..height {
        let left_pad = half_width - ROCKET_WIDTH / 2 + 1;
        let body = format!(
            "{:pad$}|{:^width$}|",
            "",
            if line_idx == middle_line {
                &stage_label
            } else {
                ""
            },
            pad = left_pad,
            width = ROCKET_WIDTH
        );

        // Add annotation on specific lines
        let annotation = if line_idx == 0 {
            format!("  <- Stage {}: {}", stage_num, engine_label)
        } else if line_idx == 1 {
            format!("     {}", prop_label)
        } else {
            String::new()
        };

        lines.push(format!("{}{}", body, annotation));
    }

    // Bottom border
    let left_pad = half_width - ROCKET_WIDTH / 2 + 1;
    lines.push(format!(
        "{:pad$}|{:_^width$}|",
        "",
        "",
        pad = left_pad,
        width = ROCKET_WIDTH
    ));

    lines
}

/// Draw the nozzle section at the bottom of the rocket.
fn draw_nozzles() -> Vec<String> {
    let half_width = ROCKET_WIDTH / 2;
    vec![
        format!("{:>width$}", "\\    /", width = half_width + 4),
        format!("{:>width$}", "\\  /", width = half_width + 3),
        format!("{:>width$}", "\\/", width = half_width + 2),
    ]
}

/// Format mass with thousands separators for display.
fn format_mass(kg: f64) -> String {
    if kg >= 1_000_000.0 {
        format!("{:.1}M", kg / 1_000_000.0)
    } else if kg >= 1_000.0 {
        format!("{:.0}k", kg / 1_000.0)
    } else {
        format!("{:.0}", kg)
    }
}

/// Print the rocket diagram to stdout.
pub fn print_rocket_diagram(rocket: &Rocket, payload_kg: f64) {
    println!();
    for line in generate_rocket_diagram(rocket, payload_kg) {
        println!("{}", line);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Engine, EngineDatabase, Propellant};
    use crate::stage::{Rocket, Stage};
    use crate::units::{Force, Isp, Mass};

    fn make_test_rocket() -> Rocket {
        let engine = Engine::new(
            "TestEngine",
            Force::newtons(1_000_000.0),
            Force::newtons(1_100_000.0),
            Isp::seconds(300.0),
            Isp::seconds(350.0),
            Mass::kg(1000.0),
            Propellant::LoxCh4,
        );

        let stage1 = Stage::new(engine.clone(), 3, Mass::kg(400_000.0), Mass::kg(20_000.0));
        let stage2 = Stage::new(engine, 1, Mass::kg(100_000.0), Mass::kg(5_000.0));
        let payload = Mass::kg(5_000.0);

        Rocket::new(vec![stage1, stage2], payload)
    }

    #[test]
    fn diagram_generates_lines() {
        let rocket = make_test_rocket();
        let lines = generate_rocket_diagram(&rocket, 5000.0);

        assert!(!lines.is_empty());
        // Should have nose cone + stages + nozzles
        assert!(lines.len() >= 10);
    }

    #[test]
    fn diagram_contains_stage_labels() {
        let rocket = make_test_rocket();
        let diagram = generate_rocket_diagram(&rocket, 5000.0).join("\n");

        assert!(diagram.contains("S1"));
        assert!(diagram.contains("S2"));
    }

    #[test]
    fn diagram_contains_payload_label() {
        let rocket = make_test_rocket();
        let diagram = generate_rocket_diagram(&rocket, 5000.0).join("\n");

        assert!(diagram.contains("Payload"));
    }

    #[test]
    fn diagram_with_real_engine() {
        let db = EngineDatabase::load_embedded().expect("load database");
        let raptor = db.get("raptor-2").expect("get raptor");

        let stage1 = Stage::new(raptor.clone(), 3, Mass::kg(400_000.0), Mass::kg(20_000.0));
        let stage2 = Stage::new(raptor.clone(), 1, Mass::kg(100_000.0), Mass::kg(5_000.0));
        let rocket = Rocket::new(vec![stage1, stage2], Mass::kg(5_000.0));

        let diagram = generate_rocket_diagram(&rocket, 5000.0).join("\n");

        assert!(diagram.contains("Raptor-2"));
    }

    #[test]
    fn format_mass_thousands() {
        assert_eq!(format_mass(500.0), "500");
        assert_eq!(format_mass(5_000.0), "5k");
        assert_eq!(format_mass(100_000.0), "100k");
        assert_eq!(format_mass(1_500_000.0), "1.5M");
    }

    #[test]
    fn single_stage_rocket() {
        let engine = Engine::new(
            "SingleEngine",
            Force::newtons(1_000_000.0),
            Force::newtons(1_100_000.0),
            Isp::seconds(300.0),
            Isp::seconds(350.0),
            Mass::kg(1000.0),
            Propellant::LoxCh4,
        );

        let stage = Stage::new(engine, 1, Mass::kg(50_000.0), Mass::kg(3_000.0));
        let rocket = Rocket::new(vec![stage], Mass::kg(1_000.0));

        let lines = generate_rocket_diagram(&rocket, 1000.0);
        assert!(!lines.is_empty());

        let diagram = lines.join("\n");
        assert!(diagram.contains("S1"));
    }
}
