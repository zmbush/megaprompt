//! Used to generate color escape sequences for bash

use std::fmt;

fn col_cmd<T: fmt::Display>(c: &T) -> String {
    format!("\\[{}[{}\\]", '\x1B', c)
}

/// Returns a foreground color escape sequence
pub fn col(c: u16) -> String {
    col_cmd(&format!("{}m", c + 30))
}

/// Returns a bold foreground color escape sequence
pub fn bcol(c: u16) -> String {
    col_cmd(&format!("1;{}m", c + 30))
}

/// Resets any color sequence
pub fn reset() -> String {
    col_cmd(&"0m".to_owned())
}
