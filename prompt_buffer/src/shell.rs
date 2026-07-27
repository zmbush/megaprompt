// Copyright 2017 Zoey Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code to handle outputting strungs to the shell.

use crate::fish_pwd;
use crate::line::PromptLineBuilder;
use std::borrow::Cow;
use std::fmt;
use std::path::Path;

/// Defines the shell type to output for
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ShellType {
    /// Bourne again shell
    Bash,

    /// Z Shell
    Zsh,

    /// The Friendly Interactive Shell
    Fish,
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

    /// Returns the working directory
    ///
    /// Bash and zsh expand their own escapes, so `path` is only consulted for fish. It has
    /// to be passed in rather than read from the environment: under the daemon the prompt
    /// is rendered for a *requested* directory, not the daemon's own working directory.
    pub fn dir(&self, path: &Path) -> Cow<'static, str> {
        match *self {
            ShellType::Bash => Cow::Borrowed(r#"\w"#),
            ShellType::Zsh => Cow::Borrowed("%~"),
            ShellType::Fish => Cow::Owned(fish_pwd::prompt_pwd(path)),
        }
    }

    /// Returns the current hostname
    pub fn hostname(&self) -> Cow<'static, str> {
        match *self {
            ShellType::Bash => Cow::Borrowed(r#"\H"#),
            ShellType::Zsh => Cow::Borrowed("%m"),
            ShellType::Fish => Cow::Owned(short_hostname()),
        }
    }

    /// Returns the escape for showing the current root/not root state of shell
    pub fn dollar(&self) -> &'static str {
        match *self {
            ShellType::Bash => r#"\$"#,
            ShellType::Zsh => "%#",
            ShellType::Fish => ">",
        }
    }

    fn col_cmd<T: fmt::Display>(&self, c: &T) -> String {
        match *self {
            ShellType::Bash => format!(r#"\[{}[{}\]"#, '\x1B', c),
            ShellType::Zsh => format!(r#"%{{{}[{}%}}"#, '\x1B', c),
            // Fish has no prompt expansion -- whatever `fish_prompt` writes goes to the
            // terminal verbatim -- so emit a real ESC byte rather than a `\e` escape. No
            // `\[..\]`/`%{..%}` bracketing is needed either: fish parses ANSI sequences
            // itself when measuring the prompt's width.
            ShellType::Fish => format!("{}[{}", '\x1B', c),
        }
    }

    /// Returns a foreground color escape sequence
    pub fn col(&self, c: u32) -> String {
        self.col_cmd(&format!("{}m", c + 30))
    }

    /// Returns a bold foreground color escape sequence
    pub fn bcol(&self, c: u32) -> String {
        self.col_cmd(&format!("1;{}m", c + 30))
    }

    /// Returns a reset sequence
    pub fn reset(&self) -> String {
        self.col_cmd(&"0m".to_owned())
    }
}

/// The hostname up to the first `.`, matching zsh's `%m` and fish's `prompt_hostname`.
///
/// TODO: swap for `std::net::hostname` once it stabilizes.
/// <https://github.com/rust-lang/rust/issues/135142>
fn short_hostname() -> String {
    let hostname = gethostname::gethostname();

    hostname
        .to_string_lossy()
        .split('.')
        .next()
        .unwrap_or_default()
        .to_owned()
}
