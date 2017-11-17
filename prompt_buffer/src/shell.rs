// Copyright 2017 Zachary Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code to handle outputting strungs to the shell.

use std::fmt;
use line::PromptLineBuilder;

/// Defines the shell type to output for
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ShellType {
    /// Bourne again shell
    Bash,

    /// Z Shell
    Zsh,
}

impl ShellType {
    /// Creates a Boxed `PromptLineBuilder`
    pub fn new_line(&self) -> PromptLineBuilder {
        PromptLineBuilder::new(*self)
    }

    /// Creates a Free `PromptLineBuilder`
    pub fn new_free_line(&self) -> PromptLineBuilder {
        PromptLineBuilder::new_free(*self)
    }

    /// Returns the escape for showing the working directory
    pub fn dir(&self) -> &'static str {
        match *self {
            ShellType::Bash => r#"\w"#,
            ShellType::Zsh => "%~",
        }
    }

    /// Returns the escape for showing the current hostname
    pub fn hostname(&self) -> &'static str {
        match *self {
            ShellType::Bash => r#"\H"#,
            ShellType::Zsh => "%m",
        }
    }

    /// Returns the escape for showing the current root/not root state of shell
    pub fn dollar(&self) -> &'static str {
        match *self {
            ShellType::Bash => r#"\$"#,
            ShellType::Zsh => "%#",
        }
    }

    fn col_cmd<T: fmt::Display>(&self, c: &T) -> String {
        match *self {
            ShellType::Bash => format!(r#"\[{}[{}\]"#, '\x1B', c),
            ShellType::Zsh => format!(r#"%{{{}[{}%}}"#, '\x1B', c),
        }
    }

    /// Returns a foreground color escape sequence
    pub fn col(&self, c: u16) -> String {
        self.col_cmd(&format!("{}m", c + 30))
    }

    /// Returns a bold foreground color escape sequence
    pub fn bcol(&self, c: u16) -> String {
        self.col_cmd(&format!("1;{}m", c + 30))
    }

    /// Returns a reset sequence
    pub fn reset(&self) -> String {
        self.col_cmd(&"0m".to_owned())
    }
}
