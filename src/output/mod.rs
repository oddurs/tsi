//! Output for the `tsi` command line.
//!
//! Three layers, so that every command looks like part of one tool:
//!
//! - [`data`]: what each command computes, as serializable values. `--output
//!   json` prints these, in a common envelope.
//! - [`views`]: each value as a [`doc::Doc`], a list of titles, fields,
//!   tables, charts and diagrams, in semantic tones rather than colours.
//! - [`render`]: lays a document out as terminal text. Colour goes through
//!   `anstream`, which drops it for pipes, `NO_COLOR` and `--color never`.

pub mod data;
pub mod diagram;
pub mod doc;
pub mod render;
pub mod views;

use std::io::Write;

use anstream::ColorChoice;

use doc::Doc;
use render::{Glyphs, Renderer};

/// How output should look, from the global flags.
#[derive(Debug, Clone, Copy)]
pub struct Look {
    pub color: ColorChoice,
    pub ascii: bool,
}

impl Look {
    /// The renderer for this look.
    pub fn renderer(&self) -> Renderer {
        Renderer {
            glyphs: if self.ascii {
                Glyphs::ASCII
            } else {
                Glyphs::UNICODE
            },
            ..Renderer::default()
        }
    }

    /// Render and print a document to stdout.
    pub fn print(&self, doc: &Doc) {
        let text = self.renderer().render(doc);
        let mut out = anstream::AutoStream::new(std::io::stdout(), self.color);
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    }
}
