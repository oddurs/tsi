//! Views: each command's data, as a [`Doc`].
//!
//! Views decide what matters and how it reads; they never pad, colour or
//! draw lines themselves. Numbers the reader came for are [`Tone::Strong`],
//! names are [`Tone::Accent`], explanations are [`Tone::Muted`], and
//! verdicts are good, warn or bad.

use tsiolkovsky::engine::Engine;
use tsiolkovsky::optimizer::{MonteCarloResults, Solution, Uncertainty};
use tsiolkovsky::physics::losses::{self, LossEstimate};
use tsiolkovsky::physics::{IspModel, G0};
use tsiolkovsky::stage::Rocket;
use tsiolkovsky::units::{format_thousands_f64 as thousands, Time};

use super::data::CalculateReport;
use super::diagram;
use super::doc::{column, field, Align, Block, Chart, Doc, Row, Segment, Table, Text, Tone};

/// Success probability below which a design is flagged.
const CONFIDENT: f64 = 0.95;

fn kg(x: f64) -> String {
    format!("{} kg", thousands(x))
}

fn mps(x: f64) -> String {
    format!("{} m/s", thousands(x))
}

/// A signed delta-v that never prints as "-0".
fn signed_mps(x: f64) -> String {
    if x.abs() < 0.5 {
        "0 m/s".to_string()
    } else if x > 0.0 {
        format!("+{}", mps(x))
    } else {
        format!("−{}", mps(-x))
    }
}

fn title(command: &str, what: impl Into<String>) -> Block {
    Block::Title {
        text: Text::new()
            .strong(format!("tsi {command}"))
            .muted("  ·  ")
            .plain(what.into()),
    }
}

/// `tsi optimize`: the rocket, where its delta-v and mass go, and how it was found.
pub fn optimize(solution: &Solution, design_margin: f64) -> Doc {
    let rocket = solution.rocket();
    let mut doc = Doc::new();
    doc.push(title(
        "optimize",
        format!(
            "{} to {}",
            kg(rocket.payload().as_kg()),
            mps(solution.target_delta_v().as_mps())
        ),
    ));

    let stages = rocket.stage_count();
    let burn = Time::seconds(
        rocket
            .stages()
            .iter()
            .map(|s| s.burn_time().as_seconds())
            .sum(),
    );
    doc.push(Block::Fields(vec![
        field(
            "Liftoff",
            Text::new().strong(kg(rocket.total_mass().as_kg())),
        )
        .note(Text::new().muted(format!(
            "{stages} stage{}, {burn} of burning",
            if stages == 1 { "" } else { "s" }
        ))),
        field("Payload", kg(rocket.payload().as_kg())).note(
            Text::new()
                .strong(format!("{:.2}%", solution.payload_fraction_percent()))
                .muted(" of the liftoff mass"),
        ),
    ]));

    doc.push(Block::Table(stage_table(rocket)));
    if stages > 1 {
        doc.push(Block::Chart(delta_v_budget(rocket)));
    }
    doc.push(Block::Chart(mass_budget(rocket)));

    let margin = solution.margin().as_mps();
    let margin_note = if design_margin > 0.0 {
        Text::new().muted(format!("designed for +{:.1}%", design_margin * 100.0))
    } else {
        Text::new().muted("hits the target exactly; --margin adds headroom")
    };
    let booster = &rocket.stages()[0];
    let isp_note = match rocket.booster_isp() {
        IspModel::AscentAveraged => format!(
            "stage 1, averaged over the climb ({:.0} s in vacuum)",
            booster.engine().isp_vac().as_seconds()
        ),
        _ => "stage 1, in vacuum: no atmosphere to climb through".to_string(),
    };
    let mut fields = vec![
        field("Margin", Text::new().plain(signed_mps(margin))).note(margin_note),
        field(
            "Booster Isp",
            format!(
                "{:.0} s",
                booster.engine().isp_for(rocket.booster_isp()).as_seconds()
            ),
        )
        .note(Text::new().muted(isp_note)),
    ];
    if (rocket.surface_gravity() - G0).abs() > 1e-6 {
        fields.push(
            field("Gravity", format!("{:.2} m/s²", rocket.surface_gravity()))
                .note(Text::new().muted("TWR is quoted against this")),
        );
    }
    fields.push(
        field("Optimizer", solution.optimizer().to_string()).note(Text::new().muted(format!(
            "{} configurations evaluated",
            thousands(solution.iterations() as f64)
        ))),
    );
    doc.push(Block::Fields(fields));
    doc
}

/// One row per stage, first stage first, then a totals row.
fn stage_table(rocket: &Rocket) -> Table {
    let mut rows: Vec<Row> = rocket
        .stages()
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (twr, when) = if i == 0 {
                (rocket.liftoff_twr(), "liftoff")
            } else {
                (rocket.stage_twr(i), "ignition")
            };
            let role = if i == 0 { "booster" } else { "upper" };
            Row {
                cells: vec![
                    Text::new().plain(format!("{} ", i + 1)).muted(role),
                    Text::new().accent(format!("{} × {}", s.engine_count(), s.engine().name())),
                    kg(s.propellant_mass().as_kg()).into(),
                    kg(s.dry_mass().as_kg()).into(),
                    format!(
                        "{:.0} s",
                        s.engine().isp_for(rocket.isp_model(i)).as_seconds()
                    )
                    .into(),
                    Text::new().strong(mps(rocket.stage_delta_v(i).as_mps())),
                    Text::new()
                        .plain(format!("{:.2} ", twr.as_f64()))
                        .muted(when),
                ],
                rule: false,
            }
        })
        .collect();
    let propellant: f64 = rocket
        .stages()
        .iter()
        .map(|s| s.propellant_mass().as_kg())
        .sum();
    let dry: f64 = rocket.stages().iter().map(|s| s.dry_mass().as_kg()).sum();
    if rocket.stage_count() > 1 {
        rows.push(Row {
            cells: vec![
                Text::new().muted("total"),
                Text::new(),
                kg(propellant).into(),
                kg(dry).into(),
                Text::new(),
                Text::new().strong(mps(rocket.total_delta_v().as_mps())),
                Text::new(),
            ],
            rule: true,
        });
    }
    Table {
        columns: vec![
            column("Stage", Align::Left),
            column("Engines", Align::Left),
            column("Propellant", Align::Right),
            column("Dry mass", Align::Right),
            column("Isp", Align::Right),
            column("Δv", Align::Right),
            column("TWR", Align::Left),
        ],
        rows,
    }
}

/// How the delta-v is shared between stages.
fn delta_v_budget(rocket: &Rocket) -> Chart {
    Chart::Stacked {
        label: "Δv  ".into(),
        segments: (0..rocket.stage_count())
            .map(|i| Segment {
                label: format!("stage {}", i + 1),
                value: rocket.stage_delta_v(i).as_mps(),
            })
            .collect(),
        total: mps(rocket.total_delta_v().as_mps()),
    }
}

/// Where the liftoff mass goes: each stage, then the payload.
fn mass_budget(rocket: &Rocket) -> Chart {
    let mut segments: Vec<Segment> = rocket
        .stages()
        .iter()
        .enumerate()
        .map(|(i, s)| Segment {
            label: format!("stage {}", i + 1),
            value: s.wet_mass().as_kg(),
        })
        .collect();
    segments.push(Segment {
        label: "payload".into(),
        value: rocket.payload().as_kg(),
    });
    Chart::Stacked {
        label: "Mass".into(),
        segments,
        total: kg(rocket.total_mass().as_kg()),
    }
}

/// `--diagram`: the rocket drawn to scale.
pub fn diagram(rocket: &Rocket, ascii: bool) -> Doc {
    let mut doc = Doc::new();
    doc.push(Block::Heading("The rocket".into()));
    doc.push(diagram::rocket(rocket, ascii));
    doc
}

/// `--show-losses`: what ideal delta-v leaves out.
pub fn losses(estimate: &LossEstimate, rocket: &Rocket) -> Doc {
    let mut doc = Doc::new();
    doc.push(Block::Heading(
        "Losses on the way to low Earth orbit".into(),
    ));
    let rows = [
        ("Gravity", estimate.gravity),
        ("Drag", estimate.drag),
        ("Steering", estimate.steering),
    ]
    .into_iter()
    .map(|(label, v)| (label.to_string(), v.as_mps(), mps(v.as_mps())))
    .collect();
    doc.push(Block::Chart(Chart::Bars { rows }));

    let ideal = rocket.total_delta_v().as_mps();
    let lost = estimate.total().as_mps();
    let left = ideal - lost;
    let orbit = losses::leo_orbital_velocity().as_mps();
    let (verdict, tone) = if left >= orbit {
        (
            format!("{} to spare over orbital velocity", mps(left - orbit)),
            Tone::Good,
        )
    } else {
        (
            format!("{} short of orbital velocity", mps(orbit - left)),
            Tone::Warn,
        )
    };
    doc.push(Block::Fields(vec![
        field("Ideal Δv", mps(ideal)),
        field("Losses", signed_mps(-lost))
            .note(Text::new().muted("first-order estimates; see docs/physics.md")),
        field("Left", Text::new().strong(mps(left))).note(Text::new().push(verdict, tone)),
        field("Orbit", mps(orbit)).note(Text::new().muted("circular, 200 km")),
    ]));
    doc
}

/// `--monte-carlo`: how often the design, built for real, still works.
pub fn monte_carlo(results: &MonteCarloResults, uncertainty: &Uncertainty) -> Doc {
    let mut doc = Doc::new();
    doc.push(Block::Heading(format!(
        "Monte Carlo: this design built {} times",
        thousands(results.total_runs() as f64)
    )));

    let p = results.success_probability();
    let (verdict, tone) = if p >= CONFIDENT {
        ("confident", Tone::Good)
    } else if p >= 0.8 {
        ("marginal", Tone::Warn)
    } else {
        ("low confidence", Tone::Bad)
    };
    let target = results.target_delta_v().as_mps();
    let mut fields = vec![
        field(
            "Success",
            Text::new().push(format!("{:.1}%", p * 100.0), tone),
        )
        .note(
            Text::new()
                .muted(format!("reach {}, ", mps(target)))
                .push(verdict, tone),
        ),
        field(
            "Delta-v",
            format!(
                "{} – {}",
                thousands(results.delta_v_percentile(5.0)),
                mps(results.delta_v_percentile(95.0))
            ),
        )
        .note(Text::new().muted(format!(
            "5th to 95th percentile, median {}",
            thousands(results.delta_v_percentile(50.0))
        ))),
        field(
            "Uncertainty",
            Text::new()
                .plain(format!(
                    "Isp ±{}%, thrust ±{}%, structure ±{}%",
                    uncertainty.isp_percent(),
                    uncertainty.thrust_percent(),
                    uncertainty.structural_percent()
                ))
                .muted("  (1σ, each stage)"),
        ),
        field("Seed", results.seed().to_string())
            .note(Text::new().muted(format!("repeat with --seed {}", results.seed()))),
    ];
    if results.failures() > 0 {
        fields.push(
            field(
                "Stuck",
                Text::new().push(thousands(results.failures() as f64), Tone::Bad),
            )
            .note(Text::new().muted("builds too heavy to leave the pad")),
        );
    }
    doc.push(Block::Fields(fields));

    let samples = results.delta_v_samples();
    if samples.len() > 1 {
        // The range always includes the target, so the marker is never
        // pinned to an edge it doesn't belong at.
        let (min, max) = samples
            .iter()
            .fold((target, target), |(lo, hi), &x| (lo.min(x), hi.max(x)));
        let bins = 48;
        let mut counts = vec![0u64; bins];
        for &x in samples {
            let i = if max > min {
                ((x - min) / (max - min) * bins as f64) as usize
            } else {
                0
            };
            counts[i.min(bins - 1)] += 1;
        }
        doc.push(Block::Chart(Chart::Histogram {
            counts,
            min,
            max,
            marker: Some(target),
            unit: "m/s".into(),
        }));
    }

    if p < CONFIDENT {
        let needed = results.required_margin(CONFIDENT);
        doc.push(Block::Note {
            tone: Tone::Warn,
            text: Text::new()
                .plain(format!(
                    "For 95% confidence, design in {} ",
                    signed_mps(needed)
                ))
                .muted(format!(
                    "(--margin {:.1})",
                    // round up to the next tenth of a percent
                    (needed / target * 1000.0).ceil() / 10.0
                )),
        });
    }
    doc
}

/// `tsi calculate`: one stage's performance.
pub fn calculate(report: &CalculateReport) -> Doc {
    let mut doc = Doc::new();
    let what = match (&report.engine, report.propellant_kg) {
        (Some(engine), Some(p)) => format!(
            "{}{}, {} propellant",
            engine,
            report
                .engine_count
                .filter(|&n| n > 1)
                .map_or(String::new(), |n| format!(" × {n}")),
            kg(p)
        ),
        _ => format!(
            "Isp {:.0} s, mass ratio {:.2}",
            report.isp_s, report.mass_ratio
        ),
    };
    doc.push(title("calculate", what));

    let mut fields = vec![field("Δv", Text::new().strong(mps(report.delta_v_mps)))
        .note(Text::new().muted("in vacuum, carrying nothing"))];
    let masses = match (report.wet_mass_kg, report.dry_mass_kg) {
        (Some(w), Some(d)) => format!("{} wet, {} dry", kg(w), kg(d)),
        _ => "wet mass over dry mass".into(),
    };
    fields.push(
        field("Mass ratio", format!("{:.2}", report.mass_ratio)).note(Text::new().muted(masses)),
    );
    let isp_note = match &report.propellant {
        Some(p) => format!("vacuum, {p}"),
        None => "vacuum".into(),
    };
    fields.push(field("Isp", format!("{:.0} s", report.isp_s)).note(Text::new().muted(isp_note)));
    if let (Some(t), Some(thrust)) = (report.burn_time_s, report.thrust_n) {
        fields.push(
            field("Burn time", Time::seconds(t).to_string()).note(Text::new().muted(format!(
                "at full vacuum thrust, {} kN",
                thousands(thrust / 1000.0)
            ))),
        );
    }
    if let Some(twr) = report.twr_vacuum {
        let tone = if twr >= 1.0 { Tone::Plain } else { Tone::Warn };
        fields.push(
            field("TWR", Text::new().push(format!("{twr:.2}"), tone))
                .note(Text::new().muted("vacuum thrust over fully loaded weight")),
        );
    }
    doc.push(Block::Fields(fields));
    doc
}

/// `tsi engines`: the database as a table.
pub fn engines(engines: &[&Engine], verbose: bool, filtered: bool) -> Doc {
    let mut doc = Doc::new();
    doc.push(title(
        "engines",
        format!(
            "{} engine{}{}",
            engines.len(),
            if engines.len() == 1 { "" } else { "s" },
            if filtered { " matching" } else { "" }
        ),
    ));
    let dash = || Text::new().muted("—");
    let rows = engines
        .iter()
        .map(|e| {
            let mut cells = vec![
                Text::new().accent(e.name()),
                Text::new().muted(e.propellant().name()),
                format!("{} kN", thousands(e.thrust_vac().as_kilonewtons())).into(),
                format!("{:.0} s", e.isp_vac().as_seconds()).into(),
            ];
            if verbose {
                if e.is_upper_stage_only() {
                    cells.push(dash());
                    cells.push(dash());
                } else {
                    cells.push(format!("{} kN", thousands(e.thrust_sl().as_kilonewtons())).into());
                    cells.push(format!("{:.0} s", e.isp_sl().as_seconds()).into());
                }
            }
            cells.push(kg(e.dry_mass().as_kg()).into());
            if verbose {
                // Sea-level thrust per kilogram of engine: what lifts rockets
                let tw = e.thrust_vac().as_newtons() / (e.dry_mass().as_kg() * G0);
                cells.push(format!("{tw:.0}").into());
            }
            Row { cells, rule: false }
        })
        .collect();
    let mut columns = vec![
        column("Engine", Align::Left),
        column("Propellant", Align::Left),
        column("Thrust vac", Align::Right),
        column("Isp vac", Align::Right),
    ];
    if verbose {
        columns.push(column("Thrust SL", Align::Right));
        columns.push(column("Isp SL", Align::Right));
    }
    columns.push(column("Mass", Align::Right));
    if verbose {
        columns.push(column("T/W", Align::Right));
    }
    doc.push(Block::Table(Table { columns, rows }));
    if verbose {
        doc.push(Block::Note {
            tone: Tone::Muted,
            text: Text::new()
                .muted("— : no sea-level rating; a vacuum engine, for upper stages only"),
        });
        doc.push(Block::Note {
            tone: Tone::Muted,
            text: Text::new().muted("T/W: vacuum thrust over the engine's own weight"),
        });
    }
    doc
}
