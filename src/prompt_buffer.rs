use std::fmt;
use std::cmp;
use std::clone;
use std::os;
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

pub struct PromptBuffer<'a> {
    plugins: Vec<Box<PromptBufferPlugin+'a>>,
    path: Path
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

pub struct PromptLine {
    level: u8,
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

    fn new_free() -> PromptLine {
        let mut r = PromptLine::new();
        r.line_type = PromptLineType::Free;
        r
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

pub struct PromptLineBuilder {
    line: PromptLine
}

impl PromptLineBuilder {
    pub fn new() -> PromptLineBuilder {
        PromptLineBuilder {
            line: PromptLine::new()
        }
    }

    pub fn new_free() -> PromptLineBuilder {
        PromptLineBuilder {
            line: PromptLine::new_free()
        }
    }

    pub fn indent_by(mut self, amt: u8) -> PromptLineBuilder {
        self.line.level += amt;

        self
    }

    pub fn indent(self) -> PromptLineBuilder {
        self.indent_by(1)
    }

    fn add_block(mut self, s: &fmt::Show, c: u16, bold: bool) -> PromptLineBuilder {
        self.line.parts.push(
            PromptBox {
                color: c,
                text: format!("{}", s),
                is_bold: bold
            }
        );

        self
    }

    pub fn block(self, s: &fmt::Show) -> PromptLineBuilder {
        self.add_block(s, color::MAGENTA, false)
    }

    pub fn colored_block(self, s: &fmt::Show, c: u16) -> PromptLineBuilder {
        self.add_block(s, c, false)
    }

    pub fn bold_colored_block(self, s: &fmt::Show, c: u16) -> PromptLineBuilder {
        self.add_block(s, c, true)
    }

    pub fn build(self) -> PromptLine {
        self.line
    }
}

impl<'a> PromptBuffer<'a> {
    pub fn new() -> PromptBuffer<'a> {
        PromptBuffer {
            plugins: Vec::new(),
            path: os::make_absolute(&Path::new(".")).unwrap()
        }
    }

    pub fn start(&self, lines: &mut Vec<PromptLine>) {
        lines.push(PromptLineBuilder::new()
            .block(&"\\w")
            .block(&"\\H")
            .build());
    }

    pub fn add_plugin(&mut self, plugin: Box<PromptBufferPlugin+'a>) {
        self.plugins.push(plugin);
    }

    pub fn set_path(&mut self, p: Path) {
        self.path = p;
    }

    pub fn to_string(&mut self) -> String {
        let mut retval = String::new();
        let mut lines = Vec::new();

        self.start(&mut lines);

        let mut pl = self.plugins.as_mut_slice();
        for i in range(0, pl.len()) {
            pl[i].run(&self.path, &mut lines);
        }

        for ix in range(0, lines.len()) {
            let ref line = lines[ix];
            let current = line.level;
            let (after, start, end) = if ix + 1 < lines.len() {
                let a = lines[ix + 1].level;
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

            retval = format!("{}{}\n", retval, line_text);
        }

        format!("{}{}{}{} ", retval, get_line(TOP|RIGHT), get_line(LEFT|RIGHT), PromptBox {
            text: "\\$".to_string(),
            color: color::RED,
            is_bold: false
        })
    }

    pub fn print(&mut self) {
        println!("{}", self.to_string());
    }
}

pub trait PromptBufferPlugin {
    fn run(&mut self, path: &Path, lines: &mut Vec<PromptLine>);
}
