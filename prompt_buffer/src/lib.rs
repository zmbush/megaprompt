extern crate term;

use std::fmt;
use std::cmp;
use std::os;
use term::color;

pub mod buffer;
pub mod thread;
pub mod line;
pub mod escape;


#[test]
fn test_prompt_lines() {
    assert_eq!(PromptBuffer::get_line(TOP | BOTTOM | LEFT | RIGHT), '┼');
    assert_eq!(PromptBuffer::get_line(TOP | BOTTOM | LEFT        ), '┤');
    assert_eq!(PromptBuffer::get_line(TOP | BOTTOM        | RIGHT), '├');
    assert_eq!(PromptBuffer::get_line(TOP | BOTTOM               ), '│');
    assert_eq!(PromptBuffer::get_line(TOP          | LEFT | RIGHT), '┴');
    assert_eq!(PromptBuffer::get_line(TOP          | LEFT        ), '┘');
    assert_eq!(PromptBuffer::get_line(TOP                 | RIGHT), '└');
    assert_eq!(PromptBuffer::get_line(      BOTTOM | LEFT        ), '┐');
    assert_eq!(PromptBuffer::get_line(      BOTTOM        | RIGHT), '┌');
    assert_eq!(PromptBuffer::get_line(      BOTTOM | LEFT | RIGHT), '┬');
    assert_eq!(PromptBuffer::get_line(               LEFT | RIGHT), '─');
}
