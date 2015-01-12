extern crate time;

use prompt_buffer;
use prompt_buffer::{PromptBufferPlugin, PromptLine, PromptLineBuilder};
use std::io::fs::PathExtensions;
use std::io::{BufferedReader, File};
use std::num::from_i64;
use term::color;

pub struct DueDatePlugin;


impl DueDatePlugin {
    pub fn new() -> DueDatePlugin {
        DueDatePlugin
    }
}

struct PathTraversal {
    path: Path
}

impl PathTraversal {
    fn new(p: &Path) -> PathTraversal {
        let mut pat = p.clone();
        pat.push("dummy");
        PathTraversal {
            path: pat,
        }
    }
}

impl Iterator for PathTraversal {
    type Item = Path;

    fn next(&mut self) -> Option<Path> {
        if !self.path.pop() { return None; };
        Some(self.path.clone())
    }
}

struct TimePeriod {
    singular: String,
    plural: String
}

impl TimePeriod {
    fn new(singular: &str) -> TimePeriod {
        let mut p = singular.to_string();
        p.push('s');

        TimePeriod::new_unique(singular, p.as_slice())
    }

    fn new_unique(singular: &str, plural: &str) -> TimePeriod {
        TimePeriod {
            singular: singular.to_string(),
            plural: plural.to_string()
        }
    }
}

impl PromptBufferPlugin for DueDatePlugin {
    fn run(&mut self, path: &Path, lines: &mut Vec<PromptLine>) {
        for mut path in PathTraversal::new(path) {
            path.push(".due");

            if path.is_file() {
                let mut reader = BufferedReader::new(File::open(&path));

                let mut line = |&mut: s: &str| { reader.read_line().unwrap_or(s.to_string()) };

                match time::strptime(line("").trim().as_slice(), "%a %b %d %H:%M:%S %Y") {
                    Ok(due_date) => {
                        let due = due_date.to_timespec();
                        let now = time::now().to_timespec();

                        let s = due.sec - now.sec;
                        let (seconds, past_due) = if s < 0 { (-s, true) } else { (s, false) };
                        let mut seconds: f32 = from_i64(seconds).unwrap_or(0.0);

                        let ups: [f32; 9] = [10.0, 10.0, 10.0, 365.0/30.0, 30.0, 24.0, 60.0, 60.0, 1.0];
                        let time_periods = [
                            TimePeriod::new_unique("millenium", "millenia"),
                            TimePeriod::new_unique("century", "centuries"),
                            TimePeriod::new("decade"),
                            TimePeriod::new("year"),
                            TimePeriod::new("month"),
                            TimePeriod::new("day"),
                            TimePeriod::new("hour"),
                            TimePeriod::new("minute"),
                            TimePeriod::new("second"),
                        ];

                        let times = range(0, ups.len()).map(|i| {
                            ups.slice_from(i).iter().fold(1.0, |a, &b| a * b)
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
                                due_phrase = format!("{}{} {} ", due_phrase, amt, name);
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
                            prompt_buffer::col(color::MAGENTA),
                            title.trim(),
                            temporal.trim(),
                            prompt_buffer::col(color),
                            due_phrase.trim(),
                            postfix
                        );

                        lines.push(PromptLineBuilder::new()
                            .block(&due_phrase)
                            .build());
                    },
                    Err(_) => {}
                }
            }
        }
    }
}
