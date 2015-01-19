use std::fmt;
use term::color;

use escape::*;

#[derive(Clone)]
pub enum PromptLineType {
    Boxed,
    Free
}

/// PromptBox
///
/// The smallest component of a prompt line
///
/// Contains a color, text, and "is bold" flag
#[derive(Clone)]
pub struct PromptBox {
    pub color: color::Color,
    pub text: String,
    pub is_bold: bool
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
    pub level: u8,
    pub line_type: PromptLineType,
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
    pub fn new() -> PromptLineBuilder {
        PromptLineBuilder {
            line: PromptLine::new()
        }
    }

    pub fn new_free() -> PromptLineBuilder {
        PromptLineBuilder {
            line: PromptLine::new_free()
        }
    }

    pub fn indent_by(mut self, amt: u8) -> PromptLineBuilder {
        self.line.level += amt;

        self
    }

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

    pub fn block(self, s: &fmt::String) -> PromptLineBuilder {
        self.add_block(s, color::MAGENTA, false)
    }

    pub fn colored_block(self, s: &fmt::String, c: u16) -> PromptLineBuilder {
        self.add_block(s, c, false)
    }

    pub fn bold_colored_block(self, s: &fmt::String, c: u16) -> PromptLineBuilder {
        self.add_block(s, c, true)
    }

    pub fn build(self) -> PromptLine {
        self.line
    }
}
