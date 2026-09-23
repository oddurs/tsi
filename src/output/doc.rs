//! A small document model for terminal output.
//!
//! Views describe *what* to show (a title, some fields, a table, a chart)
//! as a [`Doc`]; the [renderer](super::render) decides *how*: widths,
//! glyphs, colour. Nothing in a view knows about ANSI codes or padding, so
//! every command looks like part of the same tool, and changing the look
//! means changing one place.
//!
//! Text is made of [`Span`]s, each with a semantic [`Tone`] ("good",
//! "warn", "muted") rather than a colour.

/// The meaning of a piece of text, which the renderer maps to a style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    /// Ordinary text
    #[default]
    Plain,
    /// Secondary detail: units, hints, labels
    Muted,
    /// The number the reader came for
    Strong,
    /// Names of things: engines, stages, commands
    Accent,
    /// Something is fine
    Good,
    /// Something deserves a second look
    Warn,
    /// Something is wrong
    Bad,
}

/// A run of text in one tone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub tone: Tone,
}

/// A line of spans.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Text(pub Vec<Span>);

impl Text {
    /// Empty text.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a span.
    pub fn push(mut self, text: impl Into<String>, tone: Tone) -> Self {
        self.0.push(Span {
            text: text.into(),
            tone,
        });
        self
    }

    /// Append plain text.
    pub fn plain(self, text: impl Into<String>) -> Self {
        self.push(text, Tone::Plain)
    }

    /// Append muted text.
    pub fn muted(self, text: impl Into<String>) -> Self {
        self.push(text, Tone::Muted)
    }

    /// Append strong text.
    pub fn strong(self, text: impl Into<String>) -> Self {
        self.push(text, Tone::Strong)
    }

    /// Append accented text.
    pub fn accent(self, text: impl Into<String>) -> Self {
        self.push(text, Tone::Accent)
    }

    /// Width in terminal columns.
    pub fn width(&self) -> usize {
        self.0.iter().map(|s| s.text.chars().count()).sum()
    }

    /// The text without tones.
    #[cfg(test)]
    pub fn plain_text(&self) -> String {
        self.0.iter().map(|s| s.text.as_str()).collect()
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text::new().plain(s)
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Text::new().plain(s)
    }
}

/// One aligned `label  value  note` row.
#[derive(Debug, Clone)]
pub struct Field {
    pub label: String,
    pub value: Text,
    pub note: Option<Text>,
}

/// Column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

/// A table column.
#[derive(Debug, Clone)]
pub struct Column {
    pub title: String,
    pub align: Align,
}

/// A table row. `rule` draws a line above it (for totals).
#[derive(Debug, Clone)]
pub struct Row {
    pub cells: Vec<Text>,
    pub rule: bool,
}

/// A table: columns sized to their widest cell.
#[derive(Debug, Clone)]
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

/// One part of a stacked bar.
#[derive(Debug, Clone)]
pub struct Segment {
    pub label: String,
    pub value: f64,
}

/// Charts, drawn to the renderer's width.
#[derive(Debug, Clone)]
pub enum Chart {
    /// One bar split into parts, with a legend: a budget.
    Stacked {
        label: String,
        segments: Vec<Segment>,
        /// Legend text for the whole bar, e.g. "9,400 m/s"
        total: String,
    },
    /// One bar per row, scaled to the largest value.
    Bars { rows: Vec<(String, f64, String)> },
    /// A distribution: counts in equal-width bins over `[min, max]`, with an
    /// optional marked value (the target).
    Histogram {
        counts: Vec<u64>,
        min: f64,
        max: f64,
        marker: Option<f64>,
        unit: String,
    },
}

/// A block of output.
#[derive(Debug, Clone)]
pub enum Block {
    /// The first line: what this is.
    Title { text: Text },
    /// A section heading.
    Heading(String),
    /// Aligned label/value rows.
    Fields(Vec<Field>),
    /// A table.
    Table(Table),
    /// A chart.
    Chart(Chart),
    /// Pre-drawn lines (diagrams).
    Art(Vec<Text>),
    /// A remark, with a tone that says how much it matters.
    Note { tone: Tone, text: Text },
}

/// A document: blocks separated by blank lines.
#[derive(Debug, Clone, Default)]
pub struct Doc {
    pub blocks: Vec<Block>,
}

impl Doc {
    /// An empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a block.
    pub fn push(&mut self, block: Block) -> &mut Self {
        self.blocks.push(block);
        self
    }

    /// Add every block of another document.
    pub fn extend(&mut self, other: Doc) -> &mut Self {
        self.blocks.extend(other.blocks);
        self
    }
}

/// Build a field.
pub fn field(label: impl Into<String>, value: impl Into<Text>) -> Field {
    Field {
        label: label.into(),
        value: value.into(),
        note: None,
    }
}

impl Field {
    /// Add a note after the value.
    pub fn note(mut self, note: impl Into<Text>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Build a column.
pub fn column(title: impl Into<String>, align: Align) -> Column {
    Column {
        title: title.into(),
        align,
    }
}
