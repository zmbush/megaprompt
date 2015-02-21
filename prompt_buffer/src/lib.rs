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
)]

#![feature(
    env,
    old_io,
    old_path,
    std_misc,
)]

extern crate term;

pub mod buffer;
pub mod thread;
pub mod line;
pub mod escape;
