extern crate term;
extern crate git2;

use term::color;
use git2::{Repository, Error, FileState, BranchType, Branch};
use std::os;

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

fn col_cmd(c: String) -> String{
    format!("\\[{}[{}\\]", '\x1B', c)
}

fn col(c: u16) -> String {
    col_cmd(format!("{}m", c + 30))
}

fn bcol(c: u16) -> String{
    col_cmd(format!("1;{}m", c + 30))
}

fn reset() -> String{
    col_cmd("0m".to_string())
}

fn surround(t: String, color: u16) -> String{
    format!("{}{}{}{}{}",
        get_line(TOP|BOTTOM|LEFT),
        col(color),
        t,
        reset(),
        get_line(TOP|BOTTOM|RIGHT))
}

fn trail_off() -> String {
    let mut retval = String::new();
    for _ in range(0i,10i) {
        retval = retval + format!("{}", get_line(LEFT|RIGHT));
    }
    retval
}

fn get_git() -> Result<Repository, Error> {
    let path = os::make_absolute(&Path::new(".")).unwrap();
    return Repository::discover(&path);
}

fn print_main() {
    println!("{}{}{}{}{}{}{}",
        reset(),
        get_line(BOTTOM|RIGHT),
        get_line(LEFT|RIGHT),
        surround("\\w".to_string(), color::MAGENTA),
        get_line(LEFT|RIGHT),
        surround("\\H".to_string(), color::MAGENTA),
        trail_off());
}

fn print_git_status(repo: &Repository) -> bool {
    let st = repo.statuses();
    match st {
        Ok(statuses) => {
            println!("{}{}{}{}{}",
                get_line(TOP|RIGHT), get_line(LEFT|RIGHT|BOTTOM), get_line(LEFT|RIGHT),
                surround("Git Status".to_string(), color::CYAN),
                trail_off());
            for stat in statuses.iter() {
                if !stat.is_ignored {
                    println!(" {} {}{}{}",
                        get_line(TOP|BOTTOM),
                        match stat.indexed_state {
                            FileState::Clean => col(file_state_color(stat.working_state)),
                            _ => match stat.working_state {
                                FileState::Clean | FileState::Untracked => bcol(file_state_color(stat.indexed_state)),
                                _ => bcol(color::RED)
                            }
                        },
                        stat,
                        reset());
                }
            }
            return true
        },
        Err(_) => return false
    }

    fn file_state_color(state: FileState) -> u16 {
        match state {
            FileState::Clean | FileState::Untracked => color::WHITE,
            FileState::Deleted => color::RED,
            FileState::Modified => color::BLUE,
            FileState::New => color::GREEN,
            FileState::Renamed => color::CYAN,
            FileState::TypeChange => color::YELLOW,
        }
    }
}

fn git_branch(repo: &Repository) -> Result<Vec<&str>, &str> {
    let mut branches = repo.branches(None).ok().expect("Unable to load branches");

    for (mut branch, _) in branches {
        if !branch.is_head() {
            continue;
        }

        let mut result = Vec::new();

        let name = branch.name();

        match name {
            Ok(n) => match n {
                Some(value) => result.push(value.clone()),
                None => {}
            },
            Err(_) => {}
        };

        match branch.upstream() {
            Ok(upstream) => {
                match upstream.name() {
                    Ok(n) => match n {
                        Some(value) => result.push(value.clone()),
                        None => {}
                    },
                    Err(_) => {}
                }
            }
            Err(_) => {}
        };

        return Ok(result);
    }

    return Err("No active branch");
}

fn print_git_outgoing(repo: &Repository) -> bool {
    return false;
}

fn print_final_git(repo: &Repository, indented: bool) {
    if indented {
        print!("{}{}{}", get_line(BOTTOM|RIGHT), get_line(TOP|LEFT|RIGHT), get_line(LEFT|RIGHT));
    } else {
        print!("{}{}", get_line(TOP|BOTTOM|RIGHT), get_line(LEFT|RIGHT));
    }

    match git_branch(repo) {
        Ok(mut branches) => {
            let b = branches.as_slice();

            println!("{}{}", if b.len() <= 0 {
                surround("New Repository".to_string(), color::CYAN)
            } else if b.len() <= 1 {
                surround(b[0].to_string(), color::CYAN)
            } else if b.len() >= 2 {
                let branch = b[0];
                let remote_branch = b[1];
                surround(format!("{}{} -> {}{}",
                    branch,
                    reset(),
                    col(color::MAGENTA),
                    remote_branch), color::CYAN)
            } else {
                surround("What???".to_string(), color::RED)
            }, trail_off());
        },
        Err(_) => {}
    };
}

fn print_git() {
    let repo = get_git();
    if repo.is_ok() {
        let r = repo.ok().expect("Lies and slander");
        let printed_status = print_git_status(&r) || print_git_outgoing(&r);
        print_final_git(&r, printed_status);
    }
}

fn main() {
    print_main();
    print_git();

    print!("{}{}", get_line(TOP|RIGHT), get_line(LEFT|RIGHT));
    col(color::RED);
    print!("\\$ ");
    reset();
}
