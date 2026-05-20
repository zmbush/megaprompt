// Copyright 2017 Zoey Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Used as utility classes for megaprompt
//!
//! Allows easily constructing a command prompt

#![deny(
    deprecated,
    missing_docs,
    unused_imports,
    unused_must_use,
    unused_mut,
    unused_parens,
    unused_variables,
    unused_features,
    bad_style,
    unused
)]

mod buffer;
mod error;
mod line;
mod shell;
mod thread;

pub use buffer::{PluginSpeed, PromptBuffer, PromptBufferPlugin};
pub use line::PromptLines;
pub use shell::ShellType;
pub use thread::PromptThread;
