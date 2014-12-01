extern crate term;
extern crate git2;

use prompt_buffer::PromptBuffer;

mod prompt_buffer;
mod git;

fn main() {
    let mut buf = PromptBuffer::new();
    buf.start();
    buf.add_plugin(git::plugin);
    buf.print();
}
