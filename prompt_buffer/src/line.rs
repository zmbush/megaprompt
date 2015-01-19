//! Utilities and tools for drawing prompt lines
use std::fmt;
use term::color;

use escape::*;

/// The possible types for prompt lines
#[derive(Clone,Copy)]
pub enum PromptLineType {
    /// Boxed => ┤text├
    Boxed,

    /// Free => │ text
    Free
}

/// PromptBox
///
/// The smallest component of a prompt line
///
/// Contains a color, text, and "is bold" flag
#[derive(Clone)]
pub struct PromptBox {
    color: color::Color,
    text: String,
    is_bold: bool
}

impl PromptBox {
    /// Creates a prompt box
    pub fn create(t: String, color: color::Color, is_bold: bool) -> PromptBox {
        PromptBox {
            color: color,
            text: t,
            is_bold: is_bold
        }
    }
}

impl fmt::String for PromptBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", if self.is_bold { bcol(self.color) } else { col(self.color) }, self.text, reset())
    }
}

/// PromptLine
///
/// The small pieces used to display prompt lines
#[derive(Clone)]
pub struct PromptLine {
    /// The indent level of the line
    pub level: u8,
    /// Boxed or Free
    pub line_type: PromptLineType,
    /// The liste of boxes to use to construct the line
    pub parts: Vec<PromptBox>,
}

impl PromptLine {
    fn new() -> PromptLine {
        PromptLine {
            level: 0,
            line_type: PromptLineType::Boxed,
            parts: Vec::new(),
        }
    }

    fn new_free() -> PromptLine {
        let mut r = PromptLine::new();
        r.line_type = PromptLineType::Free;
        r
    }
}

/// PromptLineBuilder
///
/// Used to easily construct PromptLines
pub struct PromptLineBuilder {
    line: PromptLine
}

impl PromptLineBuilder {
    /// Creates a Boxed PromptLineBuilder
    pub fn new() -> PromptLineBuilder {
        PromptLineBuilder {
            line: PromptLine::new()
        }
    }

    /// Creates a Free PromptLineBuilder
    pub fn new_free() -> PromptLineBuilder {
        PromptLineBuilder {
            line: PromptLine::new_free()
        }
    }

    /// Increases indent by amt
    pub fn indent_by(mut self, amt: u8) -> PromptLineBuilder {
        self.line.level += amt;

        self
    }

    /// Increases indent by 1
    pub fn indent(self) -> PromptLineBuilder {
        self.indent_by(1)
    }

    fn add_block(mut self, s: &fmt::String, c: u16, bold: bool) -> PromptLineBuilder {
        self.line.parts.push(
            PromptBox {
                color: c,
                text: format!("{}", s),
                is_bold: bold
            }
        );

        self
    }

    /// Adds a block with a given text (uses the default color of color::MAGENTA)
    pub fn block(self, s: &fmt::String) -> PromptLineBuilder {
        self.add_block(s, color::MAGENTA, false)
    }

    /// Adds a block with a given text and color
    pub fn colored_block(self, s: &fmt::String, c: u16) -> PromptLineBuilder {
        self.add_block(s, c, false)
    }

    /// Adds an emboldened block with a given text and color
    pub fn bold_colored_block(self, s: &fmt::String, c: u16) -> PromptLineBuilder {
        self.add_block(s, c, true)
    }

    /// Returns the built PromptLine
    pub fn build(self) -> PromptLine {
        self.line
    }
}