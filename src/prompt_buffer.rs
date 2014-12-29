use std::fmt;
use std::cmp;
use std::clone;
use term::color;

fn col_cmd(c: &fmt::Show) -> String{
    format!("\\[{}[{}\\]", '\x1B', c)
}

pub fn col(c: u16) -> String {
    col_cmd(&format!("{}m", c + 30))
}

fn bcol(c: u16) -> String{
    col_cmd(&format!("1;{}m", c + 30))
}

pub fn reset() -> String{
    col_cmd(&"0m")
}

pub struct PromptBuffer {
    lines: Vec<PromptLine>,
    current_line: PromptLine
}

enum PromptLineType {
    Boxed,
    Free
}

impl Copy for PromptLineType {}

struct PromptBox {
    color: u16,
    text: String,
    is_bold: bool
}

impl clone::Clone for PromptBox {
    fn clone(&self) -> PromptBox {
        PromptBox {
            color: self.color,
            text: self.text.clone(),
            is_bold: self.is_bold
        }
    }
}

impl fmt::Show for PromptBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", if self.is_bold { bcol(self.color) } else { col(self.color) }, self.text, reset())
    }
}

struct PromptLine {
    level: int,
    line_type: PromptLineType,
    parts: Vec<PromptBox>,
}

impl PromptLine {
    fn new() -> PromptLine {
        PromptLine {
            level: 0,
            line_type: PromptLineType::Boxed,
            parts: Vec::new(),
        }
    }
}

impl clone::Clone for PromptLine {
    fn clone(&self) -> PromptLine {
        PromptLine {
            level: self.level,
            line_type: self.line_type,
            parts: self.parts.clone(),
        }
    }
}

const TOP       : int = 8;
const BOTTOM    : int = 4;
const LEFT      : int = 2;
const RIGHT     : int = 1;

fn get_line(flags: int) -> char {
    return match flags {
        0b1111 => '┼',
        0b1110 => '┤',
        0b1101 => '├',
        0b1100 => '│',
        0b1011 => '┴',
        0b1010 => '┘',
        0b1001 => '└',
        0b0110 => '┐',
        0b0101 => '┌',
        0b0111 => '┬',
        0b0011 => '─',
        _      => ' '
    }
}

fn trail_off() -> String {
    let mut retval = String::new();
    for _ in range(0i,10i) {
        retval = format!("{}{}", retval, get_line(LEFT|RIGHT));
    }
    retval
}

impl PromptBuffer {
    pub fn new() -> PromptBuffer {
        PromptBuffer {
            lines: Vec::new(),
            current_line: PromptLine::new()
        }
    }

    pub fn start(&mut self) {
        self.block(&"\\w");
        self.block(&"\\H");
        self.finish_line();
    }

    pub fn add_plugin(&mut self, plugin: |&mut PromptBuffer|) {
        plugin(self);
    }

    fn add_block(&mut self, s: &fmt::Show, c: u16, bold: bool) {
        self.current_line.parts.push(
            PromptBox {
                color: c,
                text: format!("{}", s),
                is_bold: bold
            }
        );
    }

    pub fn block(&mut self, s: &fmt::Show) {
        self.add_block(s, color::MAGENTA, false);
    }

    pub fn colored_block(&mut self, s: &fmt::Show, c: u16) {
        self.add_block(s, c, false);
    }

    pub fn bold_colored_block(&mut self, s: &fmt::Show, c: u16) {
        self.add_block(s, c, true);
    }

    pub fn make_free(&mut self) {
        self.current_line.line_type = PromptLineType::Free;
    }

    pub fn indent_by(&mut self, amt: int) {
        self.current_line.level += amt;
    }

    pub fn indent(&mut self) {
        self.indent_by(1);
    }

    pub fn finish_line(&mut self) {
        self.lines.push(self.current_line.clone());
        self.current_line = PromptLine::new();
    }

    pub fn print(&self) {
        for ix in range(0, self.lines.len()) {
            let ref line = self.lines[ix];
            let current = line.level;
            let (after, start, end) = if ix + 1 <self.lines.len() {
                let a = self.lines[ix + 1].level;
                (a, cmp::min(current, a), cmp::max(current, a))
            } else {
                (0, 0, current)
            };

            let mut line_text = String::new();

            for _ in range(0, start) {
                line_text = format!(" {}", line_text);
            }

            for i in range(start, end + 1) {
                line_text = format!("{}{}", line_text,
                    get_line(
                        if i == current && ix > 0 { TOP } else { 0 } |
                        if i == after { BOTTOM } else { 0 } |
                        if i > start { LEFT } else { 0 } |
                        match line.line_type {
                            PromptLineType::Boxed => RIGHT,
                            PromptLineType::Free => if i == current {
                                0
                            } else {
                                RIGHT
                            }
                        }
                    )
                );
            }

            for b in line.parts.iter() {
                line_text = match line.line_type {
                    PromptLineType::Boxed => format!("{}{}{}{}{}",
                        line_text,
                        get_line(LEFT|RIGHT),
                        get_line(LEFT|TOP|BOTTOM),
                        b,
                        get_line(TOP|BOTTOM|RIGHT)),
                    PromptLineType::Free => format!("{} {}", line_text, b)
                };
            }

            match line.line_type {
                PromptLineType::Boxed => {
                    line_text = format!("{}{}", line_text, trail_off());
                },
                _ => {}
            }

            println!("{}", line_text);
        }
        println!("{}{}{} ", get_line(TOP|RIGHT), get_line(LEFT|RIGHT), PromptBox {
            text: "\\$".to_string(),
            color: color::RED,
            is_bold: false
        });
    }
}
