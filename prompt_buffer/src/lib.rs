// Copyright 2017 Zachary Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

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
