use std::fmt;

fn col_cmd(c: &fmt::String) -> String{
    format!("\\[{}[{}\\]", '\x1B', c)
}

pub fn col(c: u16) -> String {
    col_cmd(&format!("{}m", c + 30))
}

pub fn bcol(c: u16) -> String {
    col_cmd(&format!("1;{}m", c + 30))
}

pub fn reset() -> String {
    col_cmd(&"0m")
}
