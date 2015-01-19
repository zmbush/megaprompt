//! Used as utility classes for megaprompt
//!
//! Allows easily constructing a command prompt
#![deny(unused_must_use, unused_imports)]
#![deny(unused_parens, unused_variables, unused_mut)]
#![deny(missing_docs)]
extern crate term;

pub mod buffer;
pub mod thread;
pub mod line;
pub mod escape;
