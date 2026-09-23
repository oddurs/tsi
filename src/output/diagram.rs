//! A side view of a rocket.
//!
//! ```text
//!      ╱╲      payload    5,000 kg
//!     ╱  ╲
//!    ┌────┐
//!    │ S2 │    stage 2    1 × Raptor-2     28.8 t propellant   4,955 m/s
//!    ├────┤
//!    │    │
//!    │ S1 │    stage 1    1 × Raptor-2    136.4 t propellant   4,445 m/s
//!    │    │
//!    └┬──┬┘
//! ```
//!
//! Each stage's height follows the square root of its fully loaded mass, so
//! a small upper stage is still visible above a large booster, and larger
//! stages look larger without the drawing running off the screen.

use tsiolkovsky::stage::Rocket;
use tsiolkovsky::units::format_thousands_f64;

use super::doc::{Block, Text, Tone};

/// Tallest a stage is drawn, in rows.
const MAX_ROWS: f64 = 5.0;

/// Width of the drawing column, before the labels.
const ART_WIDTH: usize = 10;

struct Pen {
    nose: [&'static str; 2],
    top: &'static str,
    wall: (char, char),
    joint: &'static str,
    base: &'static str,
}

const UNICODE: Pen = Pen {
    nose: ["  ╱╲", " ╱  ╲"],
    top: "┌────┐",
    wall: ('│', '│'),
    joint: "├────┤",
    base: "└┬──┬┘",
};

const ASCII: Pen = Pen {
    nose: ["  /\\", " /  \\"],
    top: "+----+",
    wall: ('|', '|'),
    joint: "+----+",
    base: "+-++-+",
};

/// Draw `rocket` as art lines with aligned labels.
pub fn rocket(rocket: &Rocket, ascii: bool) -> Block {
    let pen = if ascii { &ASCII } else { &UNICODE };
    let stages = rocket.stages();
    let heaviest = stages
        .iter()
        .map(|s| s.wet_mass().as_kg())
        .fold(0.0_f64, f64::max);

    // Label columns: engines, propellant, delta-v, each padded to line up.
    let labels: Vec<[String; 3]> = stages
        .iter()
        .enumerate()
        .map(|(i, s)| {
            [
                format!("{} × {}", s.engine_count(), s.engine().name()),
                format!("{} t propellant", tonnes(s.propellant_mass().as_kg())),
                format!(
                    "{} m/s",
                    format_thousands_f64(rocket.stage_delta_v(i).as_mps())
                ),
            ]
        })
        .collect();
    let widths: Vec<usize> = (0..3)
        .map(|c| {
            labels
                .iter()
                .map(|l| l[c].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();

    let art = |s: &str| format!("{s:<ART_WIDTH$}");
    let mut lines = vec![
        Text::new()
            .muted(art(pen.nose[0]))
            .muted("payload    ")
            .strong(format!(
                "{} kg",
                format_thousands_f64(rocket.payload().as_kg())
            )),
        Text::new().muted(pen.nose[1]),
        Text::new().muted(art(pen.top)),
    ];

    // Top stage first, as it stands on the pad.
    for (i, stage) in stages.iter().enumerate().rev() {
        let rows = ((stage.wet_mass().as_kg() / heaviest).sqrt() * MAX_ROWS)
            .round()
            .max(1.0) as usize;
        let middle = rows / 2;
        for row in 0..rows {
            let inside = if row == middle {
                format!("{:^4}", format!("S{}", i + 1))
            } else {
                "    ".to_string()
            };
            let wall = Text::new()
                .muted(pen.wall.0.to_string())
                .accent(inside)
                .muted(pen.wall.1.to_string());
            let mut line = Text(wall.0);
            if row == middle {
                let l = &labels[i];
                line = line
                    .plain(" ".repeat(ART_WIDTH - 6))
                    .muted(format!("stage {:<5}", i + 1))
                    .accent(format!("{:<w$}   ", l[0], w = widths[0]))
                    .plain(format!("{:>w$}   ", l[1], w = widths[1]))
                    .strong(format!("{:>w$}", l[2], w = widths[2]));
            }
            lines.push(line);
        }
        lines.push(Text::new().muted(if i == 0 { pen.base } else { pen.joint }));
    }
    // The body lines start one column in from the nose's left edge
    let lines = lines
        .into_iter()
        .map(|l| {
            let mut spans = vec![super::doc::Span {
                text: " ".into(),
                tone: Tone::Plain,
            }];
            spans.extend(l.0);
            Text(spans)
        })
        .collect();
    Block::Art(lines)
}

/// Tonnes with one decimal below 1,000 t, none above.
fn tonnes(kg: f64) -> String {
    let t = kg / 1000.0;
    if t < 1_000.0 {
        format!("{t:.1}")
    } else {
        format_thousands_f64(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsiolkovsky::engine::EngineDatabase;
    use tsiolkovsky::stage::Stage;
    use tsiolkovsky::units::Mass;

    fn two_stage() -> Rocket {
        let raptor = EngineDatabase::builtin().get("raptor-2").unwrap().clone();
        Rocket::new(
            vec![
                Stage::with_structural_ratio(raptor.clone(), 7, Mass::kg(1_000_000.0), 0.05)
                    .unwrap(),
                Stage::with_structural_ratio(raptor, 1, Mass::kg(30_000.0), 0.08).unwrap(),
            ],
            Mass::kg(5_000.0),
        )
        .unwrap()
    }

    fn text(block: &Block) -> Vec<String> {
        match block {
            Block::Art(lines) => lines.iter().map(Text::plain_text).collect(),
            _ => panic!("not art"),
        }
    }

    #[test]
    fn stages_are_labelled_top_down() {
        let lines = text(&rocket(&two_stage(), false));
        let s2 = lines.iter().position(|l| l.contains("stage 2")).unwrap();
        let s1 = lines.iter().position(|l| l.contains("stage 1")).unwrap();
        assert!(s2 < s1);
        assert!(lines[s1].contains("7 × Raptor-2"));
        assert!(lines[0].contains("5,000 kg"));
    }

    #[test]
    fn bigger_stages_are_taller_but_small_ones_still_show() {
        let lines = text(&rocket(&two_stage(), false));
        let body = |label: &str| {
            let at = lines.iter().position(|l| l.contains(label)).unwrap();
            let up = lines[..at]
                .iter()
                .rev()
                .take_while(|l| l.contains('│'))
                .count();
            let down = lines[at + 1..]
                .iter()
                .take_while(|l| l.contains('│'))
                .count();
            up + down + 1
        };
        assert!(body("stage 1") > body("stage 2"));
        assert!(body("stage 2") >= 1);
    }

    #[test]
    fn labels_line_up() {
        let lines = text(&rocket(&two_stage(), false));
        let col = |label: &str| {
            let line = lines.iter().find(|l| l.contains(label)).unwrap();
            line.find(" m/s").unwrap()
        };
        assert_eq!(col("stage 1"), col("stage 2"));
    }

    #[test]
    fn ascii_pen_draws_in_ascii() {
        // Labels may hold symbols like ×; the renderer spells those out.
        for line in text(&rocket(&two_stage(), true)) {
            let drawing: String = line.chars().take(ART_WIDTH).collect();
            assert!(drawing.is_ascii(), "{line}");
        }
    }
}
