// Copyright 2017 Zachary Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The base class
use std::cmp;
use std::env;
use std::path::PathBuf;

use term::color;
use self::lines::*;
use line::{PromptBox, PromptLineBuilder, PromptLineType, PromptLines};
use shell::ShellType;

/// Defines the speed at which to run the `to_string` method
#[derive(Clone, Copy)]
pub enum PluginSpeed {
    /// Don't run plugin
    Ignored,

    /// Don't do anything that might be slow
    Fast,

    /// Do whatever you want (still try to be fast as possible)
    Slow,
}

impl PluginSpeed {
    fn is_ignored(&self) -> bool {
        match *self {
            PluginSpeed::Ignored => true,
            _ => false,
        }
    }
}

mod lines {
    pub const TOP: i16 = 8;
    pub const BOTTOM: i16 = 4;
    pub const LEFT: i16 = 2;
    pub const RIGHT: i16 = 1;
}

/// `PromptBuffer`
///
/// Used to contain a list of `PromptLines`
/// Knows how to format a serise of `PromptLines` in a pretty way
pub struct PromptBuffer {
    plugins: Vec<Box<PromptBufferPlugin>>,
    path: PathBuf,
    shell: ShellType,
}

impl PromptBuffer {
    /// Construct a new default `PromptBuffer`
    pub fn new(shell: ShellType) -> PromptBuffer {
        PromptBuffer {
            shell,
            plugins: Vec::new(),
            path: env::current_dir().unwrap_or_else(|_| PathBuf::new()),
        }
    }

    fn get_line(flags: i16) -> char {
        match flags {
            c if c == TOP | BOTTOM | LEFT | RIGHT => '┼',
            c if c == TOP | BOTTOM | LEFT => '┤',
            c if c == TOP | BOTTOM | RIGHT => '├',
            c if c == TOP | BOTTOM => '│',
            c if c == TOP | LEFT | RIGHT => '┴',
            c if c == TOP | LEFT => '┘',
            c if c == TOP | RIGHT => '└',
            c if c == BOTTOM | LEFT => '┐',
            c if c == BOTTOM | RIGHT => '┌',
            c if c == BOTTOM | LEFT | RIGHT => '┬',
            c if c == LEFT | RIGHT => '─',
            _ => panic!("Passed invalid value to get_line"),
        }
    }

    fn trail_off() -> String {
        let mut retval = String::new();
        for _ in 0..10 {
            retval = format!("{}{}", retval, PromptBuffer::get_line(LEFT | RIGHT));
        }
        retval
    }

    fn start(&self, lines: &mut PromptLines) {
        lines.push(
            PromptLineBuilder::new(self.shell)
                .block(self.shell.dir())
                .block(self.shell.hostname())
                .build(),
        );
    }

    /// Adds a plugin to the prompt buffer
    ///
    /// They will be executed in order
    pub fn add_plugin<T: PromptBufferPlugin + Send + 'static>(&mut self, plugin: T) {
        self.plugins.push(Box::new(plugin));
    }

    /// Store the new path for the PromptBuffer.
    ///
    /// This is sent in as context to PromptBufferPlugins
    pub fn set_path(&mut self, p: PathBuf) {
        self.path = p;
    }

    /// Returns the result of the prompt
    ///
    /// Allows specifying wanted plugin speed
    pub fn convert_to_string_ext(&mut self, speed: PluginSpeed) -> String {
        let mut retval = String::new();
        let mut lines = Vec::new();

        self.start(&mut lines);

        if !speed.is_ignored() {
            for p in &mut self.plugins {
                p.run(speed, self.shell, &self.path, &mut lines);
            }
        }

        for (ix, line) in lines.iter().enumerate() {
            let current = line.level;
            let (after, start, end) = if ix + 1 < lines.len() {
                let a = lines[ix + 1].level;
                (a, cmp::min(current, a), cmp::max(current, a))
            } else {
                (0, 0, current)
            };

            let mut line_text = String::new();

            for _ in 0..start {
                line_text = format!(" {}", line_text);
            }

            for i in start..end + 1 {
                line_text = format!(
                    "{}{}",
                    line_text,
                    PromptBuffer::get_line(
                        if i == current && ix > 0 { TOP } else { 0 } | if i == after {
                            BOTTOM
                        } else {
                            0
                        } | if i > start { LEFT } else { 0 }
                            | match line.line_type {
                                PromptLineType::Boxed => RIGHT,
                                PromptLineType::Free => if i == current {
                                    0
                                } else {
                                    RIGHT
                                },
                            }
                    )
                );
            }

            for b in &line.parts {
                line_text = match line.line_type {
                    PromptLineType::Boxed => format!(
                        "{}{}{}{}{}",
                        line_text,
                        PromptBuffer::get_line(LEFT | RIGHT),
                        PromptBuffer::get_line(LEFT | TOP | BOTTOM),
                        b,
                        PromptBuffer::get_line(TOP | BOTTOM | RIGHT)
                    ),
                    PromptLineType::Free => format!("{} {}", line_text, b),
                };
            }

            if let PromptLineType::Boxed = line.line_type {
                line_text = format!("{}{}", line_text, PromptBuffer::trail_off());
            }

            retval = format!("{}{}\n", retval, line_text);
        }

        format!(
            "{}{}{}{} ",
            retval,
            PromptBuffer::get_line(TOP | RIGHT),
            PromptBuffer::get_line(LEFT | RIGHT),
            PromptBox::new(
                self.shell.dollar().to_owned(),
                color::RED,
                false,
                self.shell
            )
        )
    }

    /// Returns the prompt with plugins run
    pub fn convert_to_string(&mut self) -> String {
        self.convert_to_string_ext(PluginSpeed::Slow)
    }

    /// Print a result with the plugins
    pub fn print(&mut self) {
        println!("{}", self.convert_to_string());
    }

    /// Print a result while skipping all plugins
    pub fn print_fast(&mut self) {
        println!("{}", self.convert_to_string_ext(PluginSpeed::Fast));
    }
}

/// Implement this trait to allow extension of the `PromptBuffer`'s result
pub trait PromptBufferPlugin: Send {
    /// Should append as many PromptLines as it wants to the lines Vec
    ///
    /// The path can be used to provide context if necessary
    fn run(
        &mut self,
        speed: PluginSpeed,
        shell: ShellType,
        path: &PathBuf,
        lines: &mut PromptLines,
    );
}
