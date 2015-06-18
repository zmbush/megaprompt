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
    unused,

    clippy
)]

#![feature(std_misc, duration, plugin)]
#![plugin(clippy)]

extern crate term;

pub mod buffer;
pub mod thread;
pub mod line;
pub mod escape;
