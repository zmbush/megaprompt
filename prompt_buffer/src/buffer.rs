//! The base class
use std::os;
use std::cmp;

use term::color;

use self::lines::*;
use line::{PromptLineType, PromptLine, PromptLineBuilder, PromptBox};

/// Defines the speed at which to run the to_string method
pub enum PluginSpeed {
    /// Don't run plugin
    Ignored,

    /// Don't do anything that might be slow
    Fast,

    /// Do whatever you want (still try to be fast as possible)
    Slow
}

impl PluginSpeed {
    fn is_ignored(&self) -> bool {
        match *self {
            PluginSpeed::Ignored => true,
            _ => false
        }
    }
}

mod lines {
    pub const TOP       : i16 = 8;
    pub const BOTTOM    : i16 = 4;
    pub const LEFT      : i16 = 2;
    pub const RIGHT     : i16 = 1;
}

/// PromptBuffer
///
/// Used to contain a list of PromptLines
/// Knows how to format a serise of PromptLines in a pretty way
pub struct PromptBuffer {
    plugins: Vec<Box<PromptBufferPlugin+Send>>,
    path: Path
}

impl PromptBuffer {
    /// Creates a new PromptBuffer at the current path
    pub fn new() -> PromptBuffer {
        PromptBuffer {
            plugins: Vec::new(),
            path: os::make_absolute(&Path::new(".")).unwrap()
        }
    }

    fn get_line(flags: i16) -> char {
        return match flags {
            c if c == TOP | BOTTOM | LEFT | RIGHT => '┼',
            c if c == TOP | BOTTOM | LEFT         => '┤',
            c if c == TOP | BOTTOM        | RIGHT => '├',
            c if c == TOP | BOTTOM                => '│',
            c if c == TOP          | LEFT | RIGHT => '┴',
            c if c == TOP          | LEFT         => '┘',
            c if c == TOP                 | RIGHT => '└',
            c if c ==       BOTTOM | LEFT         => '┐',
            c if c ==       BOTTOM        | RIGHT => '┌',
            c if c ==       BOTTOM | LEFT | RIGHT => '┬',
            c if c ==                LEFT | RIGHT => '─',
            _      => panic!("Passed invalid value to get_line")
        }
    }

    fn trail_off() -> String {
        let mut retval = String::new();
        for _ in 0..10 {
            retval = format!("{}{}", retval, PromptBuffer::get_line(LEFT|RIGHT));
        }
        retval
    }

    fn start(&self, lines: &mut Vec<PromptLine>) {
        lines.push(PromptLineBuilder::new()
            .block(&"\\w")
            .block(&"\\H")
            .build());
    }

    /// Adds a plugin to the prompt buffer
    ///
    /// They will be executed in order
    pub fn add_plugin(&mut self, plugin: Box<PromptBufferPlugin+Send>) {
        self.plugins.push(plugin);
    }

    /// Store the new path for the PromptBuffer.
    ///
    /// This is sent in as context to PromptBufferPlugins
    pub fn set_path(&mut self, p: Path) {
        self.path = p;
    }

    /// Returns the result of the prompt
    ///
    /// Allows specifying wanted plugin speed
    pub fn to_string_ext(&mut self, speed: PluginSpeed) -> String {
        let mut retval = String::new();
        let mut lines = Vec::new();

        self.start(&mut lines);

        if !speed.is_ignored() {
            let mut pl = self.plugins.as_mut_slice();
            for i in 0 .. pl.len() {
                pl[i].run(&speed, &self.path, &mut lines);
            }
        }

        for ix in 0 .. lines.len() {
            let ref line = lines[ix];
            let current = line.level;
            let (after, start, end) = if ix + 1 < lines.len() {
                let a = lines[ix + 1].level;
                (a, cmp::min(current, a), cmp::max(current, a))
            } else {
                (0, 0, current)
            };

            let mut line_text = String::new();

            // FIXME: change when range syntax is fixed
            for _ in (0..start) {
                line_text = format!(" {}", line_text);
            }

            for i in start .. end + 1 {
                line_text = format!("{}{}", line_text,
                    PromptBuffer::get_line(
                        if i == current && ix > 0 { TOP } else { 0 } |
                        if i == after { BOTTOM } else { 0 } |
                        if i > start { LEFT } else { 0 } |
                        match line.line_type {
                            PromptLineType::Boxed => RIGHT,
                            PromptLineType::Free => if i == current {
                                0
                            } else {
                                RIGHT
                            }
                        }
                    )
                );
            }

            for b in line.parts.iter() {
                line_text = match line.line_type {
                    PromptLineType::Boxed => format!("{}{}{}{}{}",
                        line_text,
                        PromptBuffer::get_line(LEFT|RIGHT),
                        PromptBuffer::get_line(LEFT|TOP|BOTTOM),
                        b,
                        PromptBuffer::get_line(TOP|BOTTOM|RIGHT)),
                    PromptLineType::Free => format!("{} {}", line_text, b)
                };
            }

            match line.line_type {
                PromptLineType::Boxed => {
                    line_text = format!("{}{}", line_text, PromptBuffer::trail_off());
                },
                _ => {}
            }

            retval = format!("{}{}\n", retval, line_text);
        }

        format!("{}{}{}{} ",
            retval,
            PromptBuffer::get_line(TOP|RIGHT), PromptBuffer::get_line(LEFT|RIGHT),
            PromptBox::create("\\$".to_string(), color::RED, false))
    }

    /// Returns the prompt with plugins run
    pub fn to_string(&mut self) -> String {
        self.to_string_ext(PluginSpeed::Slow)
    }

    /// Print a result with the plugins
    pub fn print(&mut self) {
        println!("{}", self.to_string());
    }

    /// Print a result while skipping all plugins
    pub fn print_fast(&mut self) {
        println!("{}", self.to_string_ext(PluginSpeed::Fast));
    }
}

/// Implement this trait to allow extension of the PromptBuffer's result
pub trait PromptBufferPlugin {
    /// Should append as many PromptLines as it wants to the lines Vec
    ///
    /// The path can be used to provide context if necessary
    fn run(&mut self, speed: &PluginSpeed, path: &Path, lines: &mut Vec<PromptLine>);
}
