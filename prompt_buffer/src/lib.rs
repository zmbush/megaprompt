//! Used as utility classes for megaprompt
//!
//! Allows easily constructing a command prompt

#![deny(deprecated, missing_docs, unused_imports, unused_must_use, unused_mut, unused_parens,
        unused_variables, unused_features, bad_style, unused)]

#[macro_use]
extern crate chan;
#[macro_use]
extern crate log;
extern crate num;
extern crate term;

mod buffer;
mod thread;
mod line;
mod error;
mod shell;

pub use buffer::{PluginSpeed, PromptBuffer, PromptBufferPlugin};
pub use shell::ShellType;
pub use thread::PromptThread;
pub use line::PromptLines;
