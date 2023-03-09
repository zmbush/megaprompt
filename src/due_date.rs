// Copyright 2017 Zachary Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate time;

use prompt_buffer::{PluginSpeed, PromptBufferPlugin, PromptLines, ShellType};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use term::color;

#[derive(Default)]
pub struct DueDatePlugin;

impl DueDatePlugin {
    pub fn new() -> DueDatePlugin {
        DueDatePlugin::default()
    }
}

struct PathTraversal {
    path: PathBuf,
}

impl PathTraversal {
    fn new(p: &Path) -> PathTraversal {
        let mut pat: PathBuf = p.into();
        pat.push("dummy");
        PathTraversal { path: pat }
    }
}

impl Iterator for PathTraversal {
    type Item = PathBuf;

    fn next(&mut self) -> Option<PathBuf> {
        if !self.path.pop() {
            return None;
        };
        Some(self.path.clone())
    }
}

struct TimePeriod {
    singular: String,
    plural: String,
}

trait ToTimePeriod {
    fn as_period(&self) -> TimePeriod;
}

impl ToTimePeriod for str {
    fn as_period(&self) -> TimePeriod {
        let mut p = self.to_owned();
        p.push('s');

        (self, p.as_ref()).as_period()
    }
}

impl<'s> ToTimePeriod for (&'s str, &'s str) {
    fn as_period(&self) -> TimePeriod {
        let (s, p) = *self;
        TimePeriod {
            singular: s.to_owned(),
            plural: p.to_owned(),
        }
    }
}

impl PromptBufferPlugin for DueDatePlugin {
    fn run(&mut self, _: PluginSpeed, shell: ShellType, path: &Path, lines: &mut PromptLines) {
        for mut path in PathTraversal::new(path) {
            path.push(".due");

            if path.is_file() {
                let mut reader = BufReader::new(File::open(&path).expect("Unable to open file"));

                let mut line = |s: &str| {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(_) => line,
                        Err(_) => s.to_owned(),
                    }
                };

                let due_format = time::macros::format_description!(
                    "[weekday repr:short] [month repr:short] [day] [hour]:[minute]:[second] [year repr:full]"
                );
                if let Ok(due_date) = time::Time::parse(line("").trim(), due_format) {
                    // let due = due_date.to_timespec();
                    let now = time::OffsetDateTime::now_utc();
                    // let now = time::Instant::now();
                    // let now = time::now().to_timespec();

                    let s = due_date - now.time();
                    let (seconds, past_due) = if s.is_negative() {
                        (-s, true)
                    } else {
                        (s, false)
                    };
                    let mut seconds: f32 = seconds.as_seconds_f32();

                    let ups: [f32; 9] =
                        [10.0, 10.0, 10.0, 365.0 / 30.0, 30.0, 24.0, 60.0, 60.0, 1.0];
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

                    let times = (0..ups.len()).map(|i| ups[i..].iter().fold(1.0, |a, &b| a * b));

                    let accuracy = 2u8;
                    let mut count = 0u8;
                    let mut due_phrase = String::new();

                    for (amount, name) in times.zip(time_periods.iter()) {
                        if seconds > amount {
                            count += 1;
                            let rem = seconds % amount;
                            let amt = seconds / amount - (rem / amount);
                            seconds = rem;
                            let name = if amt > 1.0 {
                                &name.plural
                            } else {
                                &name.singular
                            };
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

                    due_phrase = format!(
                        "{}{} {}: {}{}{}",
                        shell.col(color::MAGENTA),
                        title.trim(),
                        temporal.trim(),
                        shell.col(color),
                        due_phrase.trim(),
                        postfix
                    );

                    lines.push(shell.new_line().block(due_phrase).build());
                }
            }
        }
    }
}
