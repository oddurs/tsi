//! Render a [`Doc`] as styled terminal text.
//!
//! The renderer writes ANSI styles unconditionally; printing through
//! [`anstream`] strips them when stdout isn't a terminal, when `NO_COLOR` is
//! set, or when the user asks for `--color never`. Layout is plain-text
//! arithmetic on span widths, so it is the same with or without colour.

use anstyle::{AnsiColor, Style};

use super::doc::{Align, Block, Chart, Doc, Field, Segment, Span, Table, Text, Tone};

/// Spaces before every block except the title.
const INDENT: &str = "  ";

/// Columns between field parts, and between a chart's label and bar.
const GAP: usize = 3;

/// Columns between table columns.
const TABLE_GAP: usize = 2;

/// Widest a chart bar gets.
const MAX_BAR: usize = 48;

/// Characters the renderer draws with.
#[derive(Debug, Clone, Copy)]
pub struct Glyphs {
    /// Fills for stacked-bar segments, in order
    pub fills: [char; 4],
    /// Full and partial blocks for horizontal bars, lightest first
    pub partial: &'static [char],
    /// Column heights for histograms, lowest first
    pub levels: &'static [char],
    pub rule: char,
    pub marker: char,
    pub warn: char,
    pub bad: char,
    pub good: char,
    pub bullet: char,
    /// Also spell out symbols in text (×, Δ, ±) in ASCII
    pub ascii: bool,
}

impl Glyphs {
    /// Box-drawing and block characters.
    pub const UNICODE: Glyphs = Glyphs {
        fills: ['█', '▓', '▒', '░'],
        partial: &['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'],
        levels: &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'],
        rule: '─',
        marker: '▲',
        warn: '▲',
        bad: '✖',
        good: '✔',
        bullet: '·',
        ascii: false,
    };

    /// Plain ASCII, for terminals and fonts without the above.
    pub const ASCII: Glyphs = Glyphs {
        fills: ['#', '=', '+', '.'],
        partial: &['#'],
        levels: &[' ', '.', ':', '-', '=', '+', '*', '#', '#'],
        rule: '-',
        marker: '^',
        warn: '!',
        bad: 'x',
        good: '+',
        bullet: '-',
        ascii: true,
    };
}

/// Lays out documents.
#[derive(Debug, Clone, Copy)]
pub struct Renderer {
    /// Total width available, in columns
    pub width: usize,
    pub glyphs: Glyphs,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            width: 80,
            glyphs: Glyphs::UNICODE,
        }
    }
}

/// The style for a tone.
fn style(tone: Tone) -> Style {
    match tone {
        Tone::Plain => Style::new(),
        Tone::Muted => Style::new().dimmed(),
        Tone::Strong => Style::new().bold(),
        Tone::Accent => Style::new().fg_color(Some(AnsiColor::Cyan.into())),
        Tone::Good => Style::new().fg_color(Some(AnsiColor::Green.into())),
        Tone::Warn => Style::new().fg_color(Some(AnsiColor::Yellow.into())),
        Tone::Bad => Style::new().bold().fg_color(Some(AnsiColor::Red.into())),
    }
}

fn paint(text: &str, tone: Tone) -> String {
    if text.is_empty() || tone == Tone::Plain {
        return text.to_string();
    }
    let s = style(tone);
    format!("{}{text}{}", s.render(), s.render_reset())
}

fn paint_text(text: &Text) -> String {
    text.0
        .iter()
        .map(|s: &Span| paint(&s.text, s.tone))
        .collect()
}

/// Text with trailing spaces removed, so a line never ends in whitespace
/// hidden inside a styled span.
fn trim_end(text: &Text) -> Text {
    let mut spans = text.0.clone();
    while let Some(last) = spans.last_mut() {
        let trimmed = last.text.trim_end().to_string();
        if trimmed.is_empty() {
            spans.pop();
        } else {
            last.text = trimmed;
            break;
        }
    }
    Text(spans)
}

/// Pad styled text to `width` columns.
fn pad(text: &Text, width: usize, align: Align) -> String {
    let fill = " ".repeat(width.saturating_sub(text.width()));
    match align {
        Align::Left => format!("{}{fill}", paint_text(text)),
        Align::Right => format!("{fill}{}", paint_text(text)),
    }
}

impl Renderer {
    /// Render a document.
    pub fn render(&self, doc: &Doc) -> String {
        let blocks: Vec<String> = doc.blocks.iter().map(|b| self.block(b)).collect();
        let mut out = String::from("\n");
        out.push_str(&blocks.join("\n\n"));
        out.push_str("\n\n");
        if self.glyphs.ascii {
            out = to_ascii(&out);
        }
        out
    }

    fn block(&self, block: &Block) -> String {
        match block {
            Block::Title { text } => paint_text(text),
            Block::Heading(text) => format!("{INDENT}{}", paint(text, Tone::Strong)),
            Block::Fields(fields) => self.fields(fields),
            Block::Table(table) => self.table(table),
            Block::Chart(chart) => self.chart(chart),
            Block::Art(lines) => lines
                .iter()
                .map(|l| format!("{INDENT}{}", paint_text(&trim_end(l))))
                .collect::<Vec<_>>()
                .join("\n"),
            Block::Note { tone, text } => {
                let mark = match tone {
                    Tone::Warn => self.glyphs.warn,
                    Tone::Bad => self.glyphs.bad,
                    Tone::Good => self.glyphs.good,
                    _ => self.glyphs.bullet,
                };
                format!(
                    "{INDENT}{} {}",
                    paint(&mark.to_string(), *tone),
                    paint_text(text)
                )
            }
        }
    }

    fn fields(&self, fields: &[Field]) -> String {
        let label_w = fields
            .iter()
            .map(|f| f.label.chars().count())
            .max()
            .unwrap_or(0);
        let value_w = fields
            .iter()
            .filter(|f| f.note.is_some())
            .map(|f| f.value.width())
            .max()
            .unwrap_or(0);
        fields
            .iter()
            .map(|f| {
                let label = format!("{:<label_w$}", f.label);
                let mut line = format!("{INDENT}{}{}", paint(&label, Tone::Muted), " ".repeat(GAP));
                match &f.note {
                    Some(note) => {
                        line.push_str(&pad(&f.value, value_w, Align::Left));
                        line.push_str(&" ".repeat(GAP));
                        line.push_str(&paint_text(note));
                    }
                    None => line.push_str(&paint_text(&f.value)),
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn table(&self, table: &Table) -> String {
        let widths: Vec<usize> = table
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| {
                table
                    .rows
                    .iter()
                    .filter_map(|r| r.cells.get(i))
                    .map(Text::width)
                    .chain([c.title.chars().count()])
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let total: usize =
            widths.iter().sum::<usize>() + TABLE_GAP * widths.len().saturating_sub(1);
        let gap = " ".repeat(TABLE_GAP);

        let line = |cells: Vec<String>| {
            format!("{INDENT}{}", cells.join(&gap))
                .trim_end()
                .to_string()
        };
        let mut out = vec![line(
            table
                .columns
                .iter()
                .zip(&widths)
                .map(|(c, &w)| pad(&Text::new().muted(c.title.clone()), w, c.align))
                .collect(),
        )];
        for row in &table.rows {
            if row.rule {
                out.push(format!(
                    "{INDENT}{}",
                    paint(&self.glyphs.rule.to_string().repeat(total), Tone::Muted)
                ));
            }
            out.push(line(
                table
                    .columns
                    .iter()
                    .zip(&widths)
                    .enumerate()
                    .map(|(i, (c, &w))| {
                        let empty = Text::new();
                        pad(row.cells.get(i).unwrap_or(&empty), w, c.align)
                    })
                    .collect(),
            ));
        }
        out.join("\n")
    }

    fn chart(&self, chart: &Chart) -> String {
        match chart {
            Chart::Stacked {
                label,
                segments,
                total,
            } => self.stacked(label, segments, total),
            Chart::Bars { rows } => self.bars(rows),
            Chart::Histogram {
                counts,
                min,
                max,
                marker,
                unit,
            } => self.histogram(counts, *min, *max, *marker, unit),
        }
    }

    /// Bar width for a chart whose rows start with a `label_w`-wide label.
    fn bar_width(&self, label_w: usize, tail: usize) -> usize {
        self.width
            .saturating_sub(INDENT.len() + label_w + GAP + tail + GAP)
            .clamp(10, MAX_BAR)
    }

    fn stacked(&self, label: &str, segments: &[Segment], total: &str) -> String {
        let label_w = label.chars().count();
        let width = self.bar_width(label_w, total.chars().count());
        let sum: f64 = segments.iter().map(|s| s.value.max(0.0)).sum();

        // Share out the cells by largest remainder, so the bar is exactly
        // `width` wide and anything non-zero gets at least one cell.
        let exact: Vec<f64> = segments
            .iter()
            .map(|s| {
                if sum > 0.0 {
                    s.value.max(0.0) / sum * width as f64
                } else {
                    0.0
                }
            })
            .collect();
        let mut cells: Vec<usize> = exact
            .iter()
            .map(|&x| {
                if x > 0.0 {
                    (x.floor() as usize).max(1)
                } else {
                    0
                }
            })
            .collect();
        while cells.iter().sum::<usize>() > width {
            if let Some(i) = (0..cells.len())
                .filter(|&i| cells[i] > 1)
                .max_by_key(|&i| cells[i])
            {
                cells[i] -= 1;
            } else {
                break;
            }
        }
        let mut order: Vec<usize> = (0..cells.len()).collect();
        order.sort_by(|&a, &b| {
            (exact[b] - exact[b].floor()).total_cmp(&(exact[a] - exact[a].floor()))
        });
        let mut k = 0;
        while cells.iter().sum::<usize>() < width && !order.is_empty() {
            let i = order[k % order.len()];
            if exact[i] > 0.0 {
                cells[i] += 1;
            }
            k += 1;
            if k > 4 * order.len() {
                break;
            }
        }

        let fill = |i: usize| self.glyphs.fills[i % self.glyphs.fills.len()];
        let tones = [Tone::Accent, Tone::Plain, Tone::Muted, Tone::Accent];
        let bar: String = cells
            .iter()
            .enumerate()
            .map(|(i, &n)| paint(&fill(i).to_string().repeat(n), tones[i % tones.len()]))
            .collect();
        let legend = segments
            .iter()
            .enumerate()
            .map(|(i, s)| {
                format!(
                    "{} {} {}",
                    paint(&fill(i).to_string(), tones[i % tones.len()]),
                    s.label,
                    paint(&format!("{:.0}%", s.value / sum * 100.0), Tone::Muted)
                )
            })
            .collect::<Vec<_>>()
            .join("   ");
        format!(
            "{INDENT}{}{}{bar}{}{}\n{INDENT}{}{legend}",
            paint(label, Tone::Muted),
            " ".repeat(GAP),
            " ".repeat(GAP),
            total,
            " ".repeat(label_w + GAP)
        )
    }

    fn bars(&self, rows: &[(String, f64, String)]) -> String {
        let label_w = rows.iter().map(|r| r.0.chars().count()).max().unwrap_or(0);
        let value_w = rows.iter().map(|r| r.2.chars().count()).max().unwrap_or(0);
        let width = self.bar_width(label_w, value_w);
        let max = rows.iter().map(|r| r.1).fold(0.0_f64, f64::max);
        rows.iter()
            .map(|(label, value, shown)| {
                let bar = self.bar(if max > 0.0 { value / max } else { 0.0 }, width);
                format!(
                    "{INDENT}{}{}{:>value_w$}{}{}",
                    paint(&format!("{label:<label_w$}"), Tone::Muted),
                    " ".repeat(GAP),
                    shown,
                    " ".repeat(GAP),
                    paint(&bar, Tone::Accent)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A horizontal bar `fraction` of `width`, with a partial last cell.
    fn bar(&self, fraction: f64, width: usize) -> String {
        let eighths = (fraction.clamp(0.0, 1.0) * width as f64 * 8.0).round() as usize;
        let parts = self.glyphs.partial;
        if parts.len() == 1 {
            return parts[0].to_string().repeat(eighths.div_ceil(8));
        }
        let mut s = parts[7].to_string().repeat(eighths / 8);
        if !eighths.is_multiple_of(8) {
            s.push(parts[eighths % 8 - 1]);
        }
        s
    }

    fn histogram(
        &self,
        counts: &[u64],
        min: f64,
        max: f64,
        marker: Option<f64>,
        unit: &str,
    ) -> String {
        let levels = self.glyphs.levels;
        let top = counts.iter().copied().max().unwrap_or(0).max(1);
        let n = counts.len();
        // Two rows of eighths give 16 levels of height.
        let height = |c: u64| (c as f64 / top as f64 * 16.0).round() as usize;
        let row = |upper: bool| -> String {
            counts
                .iter()
                .map(|&c| {
                    let h = height(c);
                    let h = if upper { h.saturating_sub(8) } else { h.min(8) };
                    levels[h.min(levels.len() - 1)]
                })
                .collect()
        };
        let pos = |v: f64| -> usize {
            if max > min {
                (((v - min) / (max - min)) * n as f64)
                    .floor()
                    .clamp(0.0, (n - 1) as f64) as usize
            } else {
                0
            }
        };

        let mut lines = vec![
            format!("{INDENT}{}", paint(row(true).trim_end(), Tone::Accent)),
            format!("{INDENT}{}", paint(row(false).trim_end(), Tone::Accent)),
        ];
        let mut axis: Vec<char> = self.glyphs.rule.to_string().repeat(n).chars().collect();
        // Labels: minimum at the left, maximum and unit at the right, and the
        // marked value under its tick when there's room between them.
        let low = fmt_axis(min);
        let high = format!("{} {unit}", fmt_axis(max));
        let span = n.max(low.chars().count() + high.chars().count() + 2);
        let mut labels: Vec<char> = " ".repeat(span).chars().collect();
        let put = |labels: &mut Vec<char>, at: usize, text: &str| {
            for (k, c) in text.chars().enumerate() {
                labels[at + k] = c;
            }
        };
        put(&mut labels, 0, &low);
        let high_at = span - high.chars().count();
        put(&mut labels, high_at, &high);
        if let Some(m) = marker {
            let p = pos(m);
            axis[p] = self.glyphs.marker;
            let text = fmt_axis(m);
            let w = text.chars().count();
            let earliest = low.chars().count() + 2;
            let at = p.saturating_sub(w / 2).max(earliest);
            // A marker at either end is already labelled by the range
            let at_edge = p == 0 || p + 1 == n;
            if !at_edge && at + w + 2 <= high_at {
                put(&mut labels, at, &text);
            }
        }
        let axis: String = axis.into_iter().collect();
        let labels: String = labels.into_iter().collect();
        lines.push(format!(
            "{INDENT}{}",
            axis.chars()
                .map(|c| if c == self.glyphs.marker {
                    paint(&c.to_string(), Tone::Warn)
                } else {
                    paint(&c.to_string(), Tone::Muted)
                })
                .collect::<String>()
        ));
        lines.push(format!("{INDENT}{}", paint(labels.trim_end(), Tone::Muted)));
        lines.join("\n")
    }
}

/// Spell out the non-ASCII symbols views use in text.
fn to_ascii(s: &str) -> String {
    s.replace('×', "x")
        .replace('Δ', "d")
        .replace('±', "+/-")
        .replace('σ', "sd")
        .replace(['—', '–', '−', '·'], "-")
        .replace('²', "^2")
}

/// A number for an axis label: thousands separators, no decimals.
fn fmt_axis(v: f64) -> String {
    tsiolkovsky::units::format_thousands_f64(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::doc::{column, field, Row};

    fn plain(doc: &Doc) -> String {
        let styled = Renderer::default().render(doc);
        strip(&styled)
    }

    /// Remove ANSI escapes.
    fn strip(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn fields_align_values_and_notes() {
        let mut doc = Doc::new();
        doc.push(Block::Fields(vec![
            field("Δv", "7,771 m/s").note("in vacuum"),
            field("Burn time", "2m 20s").note("full thrust"),
        ]));
        let text = plain(&doc);
        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        // Columns, not bytes: "Δ" is two bytes wide in UTF-8.
        let col = |line: &str, needle: &str| line.find(needle).map(|b| line[..b].chars().count());
        assert_eq!(col(lines[0], "7,771"), col(lines[1], "2m 20s"));
        assert_eq!(col(lines[0], "in vacuum"), col(lines[1], "full thrust"));
    }

    #[test]
    fn table_columns_align() {
        let mut doc = Doc::new();
        doc.push(Block::Table(Table {
            columns: vec![column("Name", Align::Left), column("Mass", Align::Right)],
            rows: vec![
                Row {
                    cells: vec!["Merlin-1D".into(), "470 kg".into()],
                    rule: false,
                },
                Row {
                    cells: vec!["F-1".into(), "8,400 kg".into()],
                    rule: false,
                },
            ],
        }));
        let text = plain(&doc);
        let ends: Vec<usize> = text
            .lines()
            .filter(|l| l.contains("kg") || l.contains("Mass"))
            .map(|l| l.chars().count())
            .collect();
        assert!(ends.windows(2).all(|w| w[0] == w[1]), "{text}");
    }

    #[test]
    fn stacked_bar_fills_its_width_and_shows_small_parts() {
        let r = Renderer::default();
        let out = strip(&r.stacked(
            "Mass",
            &[
                Segment {
                    label: "big".into(),
                    value: 97.0,
                },
                Segment {
                    label: "tiny".into(),
                    value: 0.1,
                },
            ],
            "100 t",
        ));
        let bar_line = out.lines().next().unwrap();
        assert!(
            bar_line.contains('▓'),
            "a tiny segment still gets a cell: {bar_line}"
        );
    }

    #[test]
    fn histogram_marks_the_target() {
        let r = Renderer::default();
        let counts: Vec<u64> = (0..48).map(|i: u64| 24 - i.abs_diff(24)).collect();
        let out = strip(&r.histogram(&counts, 9_000.0, 10_000.0, Some(9_400.0), "m/s"));
        assert!(out.contains('▲'));
        assert!(out.contains("9,400"));
        assert!(out.contains("10,000 m/s"));
    }

    #[test]
    fn narrow_histogram_labels_never_collide() {
        let r = Renderer::default();
        let out = strip(&r.histogram(&[1, 4, 9, 4, 1], 9_000.0, 10_000.0, Some(9_400.0), "m/s"));
        let labels = out.lines().last().unwrap();
        assert!(labels.starts_with("  9,000"), "{labels}");
        assert!(labels.ends_with("10,000 m/s"), "{labels}");
    }

    #[test]
    fn ascii_glyphs_are_ascii() {
        let r = Renderer {
            glyphs: Glyphs::ASCII,
            ..Renderer::default()
        };
        let mut doc = Doc::new();
        doc.push(Block::Chart(Chart::Bars {
            rows: vec![
                ("a".into(), 1.0, "1".into()),
                ("b".into(), 0.5, "0.5".into()),
            ],
        }));
        doc.push(Block::Note {
            tone: Tone::Warn,
            text: "careful".into(),
        });
        doc.push(Block::Fields(vec![field("Δv", "1 × Raptor ±1% σ")]));
        let out = strip(&r.render(&doc));
        assert!(out.is_ascii(), "{out}");
    }
}
