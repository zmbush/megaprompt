//! Utilities and tools for drawing prompt lines

use std::fmt;
use term::color;
use shell::ShellType;

/// The possible types for prompt lines
#[derive(Clone, Copy)]
pub enum PromptLineType {
    /// Boxed => ┤text├
    Boxed,

    /// Free => │ text
    Free,
}

/// `PromptBox`
///
/// The smallest component of a prompt line
///
/// Contains a color, text, and "is bold" flag
#[derive(Clone)]
pub struct PromptBox {
    color: color::Color,
    text: String,
    is_bold: bool,
    shell: ShellType,
}

impl PromptBox {
    /// Creates a prompt box
    pub fn new(text: String, color: color::Color, is_bold: bool, shell: ShellType) -> PromptBox {
        PromptBox {
            color,
            text,
            is_bold,
            shell,
        }
    }
}

impl fmt::Display for PromptBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            if self.is_bold {
                self.shell.bcol(self.color)
            } else {
                self.shell.col(self.color)
            },
            self.text,
            self.shell.reset()
        )
    }
}

/// `PromptLine`
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
    /// Shell type to output for
    pub shell: ShellType,
}

impl PromptLine {
    fn new(shell: ShellType) -> PromptLine {
        PromptLine {
            level: 0,
            line_type: PromptLineType::Boxed,
            parts: Vec::new(),
            shell: shell,
        }
    }

    fn new_free(shell: ShellType) -> PromptLine {
        PromptLine {
            line_type: PromptLineType::Free,
            ..PromptLine::new(shell)
        }
    }
}

/// A list of `PromptLines`
pub type PromptLines = Vec<PromptLine>;

/// `PromptLineBuilder`
///
/// Used to easily construct `PromptLines`
pub struct PromptLineBuilder {
    line: PromptLine,
    shell: ShellType,
}

impl PromptLineBuilder {
    /// Creates a Boxed `PromptLineBuilder`
    pub(crate) fn new(shell: ShellType) -> PromptLineBuilder {
        PromptLineBuilder {
            line: PromptLine::new(shell),
            shell,
        }
    }

    /// Creates a Free `PromptLineBuilder`
    pub(crate) fn new_free(shell: ShellType) -> PromptLineBuilder {
        PromptLineBuilder {
            line: PromptLine::new_free(shell),
            shell,
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

    fn add_block<T: fmt::Display>(mut self, text: T, color: u16, bold: bool) -> PromptLineBuilder {
        self.line
            .parts
            .push(PromptBox::new(format!("{}", text), color, bold, self.shell));

        self
    }

    /// Adds a block with a given text (uses the default color of color::MAGENTA)
    pub fn block<T: fmt::Display>(self, s: T) -> PromptLineBuilder {
        self.add_block(s, color::MAGENTA, false)
    }

    /// Adds a block with a given text and color
    pub fn colored_block<T: fmt::Display>(self, s: T, c: u16) -> PromptLineBuilder {
        self.add_block(s, c, false)
    }

    /// Adds an emboldened block with a given text and color
    pub fn bold_colored_block<T: fmt::Display>(self, s: T, c: u16) -> PromptLineBuilder {
        self.add_block(s, c, true)
    }

    /// Returns the built PromptLine
    pub fn build(self) -> PromptLine {
        self.line
    }
}
