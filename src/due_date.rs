extern crate time;

use prompt_buffer::escape;
use prompt_buffer::buffer::{PromptBufferPlugin, PluginSpeed};
use prompt_buffer::line::{PromptLine, PromptLineBuilder};
use std::old_io::fs::PathExtensions;
use std::num::Float;
use term::color;
use std::path::PathBuf;
use std::fs::{
    PathExt,
    File
};
use std::io::{
    BufReader,
    BufRead
};

pub struct DueDatePlugin;

impl DueDatePlugin {
    pub fn new() -> DueDatePlugin {
        DueDatePlugin
    }
}

struct PathTraversal {
    path: PathBuf
}

impl PathTraversal {
    fn new(p: &PathBuf) -> PathTraversal {
        let mut pat = p.clone();
        pat.push("dummy");
        PathTraversal {
            path: pat,
        }
    }
}

impl Iterator for PathTraversal {
    type Item = PathBuf;

    fn next(&mut self) -> Option<PathBuf> {
        if !self.path.pop() { return None; };
        Some(self.path.clone())
    }
}

struct TimePeriod {
    singular: String,
    plural: String
}

trait ToTimePeriod {
    fn as_period(&self) -> TimePeriod;
}

impl ToTimePeriod for str {
    fn as_period(&self) -> TimePeriod {
        let mut p = self.to_string();
        p.push('s');

        (self, p.as_slice()).as_period()
    }
}

impl<'s> ToTimePeriod for (&'s str, &'s str) {
    fn as_period(&self) -> TimePeriod {
        let (s, p) = *self;
        TimePeriod {
            singular: s.to_string(),
            plural: p.to_string()
        }
    }
}

impl PromptBufferPlugin for DueDatePlugin {
    fn run(&mut self, _: &PluginSpeed, path: &PathBuf, lines: &mut Vec<PromptLine>) {
        for mut path in PathTraversal::new(path) {
            path.push(".due");

            if path.is_file() {
                let mut reader = BufReader::new(File::open(&path).unwrap());

                let mut line = |s: &str| {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(_) => line,
                        Err(_) => s.to_string()
                    }
                };

                match time::strptime(line("").trim().as_slice(), "%a %b %d %H:%M:%S %Y") {
                    Ok(due_date) => {
                        let due = due_date.to_timespec();
                        let now = time::now().to_timespec();

                        let s = due.sec - now.sec;
                        let (seconds, past_due) = if s < 0 { (-s, true) } else { (s, false) };
                        let mut seconds: f32 = seconds as f32;

                        let ups: [f32; 9] = [10.0, 10.0, 10.0, 365.0/30.0, 30.0, 24.0, 60.0, 60.0, 1.0];
                        let time_periods = [
                            ("millenium", "millenia").as_period(),
                            ("century", "centuries").as_period(),
                            "decade".as_period(),
                            "year".as_period(),
                            "month".as_period(),
                            "day".as_period(),
                            "hour".as_period(),
                            "minute".as_period(),
                            "second".as_period(),
                        ];

                        let times = range(0, ups.len()).map(|i| {
                            ups[i..].iter().fold(1.0, |a, &b| a * b)
                        });

                        let accuracy = 2u8;
                        let mut count = 0u8;
                        let mut due_phrase = String::new();

                        for (amount, name) in times.zip(time_periods.iter()) {
                            if seconds > amount {
                                count += 1;
                                let rem = seconds % amount;
                                let amt = seconds / amount - (rem / amount);
                                seconds = rem;
                                let name = if amt > 1.0 { &name.plural } else { &name.singular };
                                due_phrase = format!("{}{} {} ", due_phrase, amt.round() as i32, name);
                            }

                            if count >= accuracy {
                                break;
                            }
                        }

                        let title = line("Project");
                        let future = line("is due in");
                        let past = line("was due");
                        let (color, temporal, postfix) = if past_due {
                            (color::RED, past, " ago")
                        } else {
                            (color::CYAN, future, "")
                        };

                        due_phrase = format!("{}{} {}: {}{}{}",
                            escape::col(color::MAGENTA),
                            title.trim(),
                            temporal.trim(),
                            escape::col(color),
                            due_phrase.trim(),
                            postfix
                        );

                        lines.push(PromptLineBuilder::new()
                            .block(due_phrase)
                            .build());
                    },
                    Err(_) => {}
                }
            }
        }
    }
}
