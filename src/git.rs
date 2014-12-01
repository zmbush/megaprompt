extern crate git2;

use prompt_buffer;
use prompt_buffer::PromptBuffer;
use git2::{Repository, Error, FileState};
use std::os;
use term::color;

fn get_git() -> Result<Repository, Error> {
    let path = os::make_absolute(&Path::new(".")).unwrap();
    return Repository::discover(&path);
}

fn status(buffer: &mut PromptBuffer, repo: &Repository) -> bool {
    let st = repo.statuses();
    match st {
        Ok(statuses) => {
            buffer.colored_block("Git Status".to_string(), color::CYAN);
            buffer.finish_line();
            for stat in statuses.iter() {
                if !stat.is_ignored {
                    buffer.make_free();
                    buffer.indent();
                    let val = format!("{}", stat);

                    match stat.indexed_state {
                        FileState::Clean => buffer.colored_block(val, file_state_color(stat.working_state)),
                        _ => match stat.working_state {
                            FileState::Clean | FileState::Untracked => buffer.bold_colored_block(val, file_state_color(stat.indexed_state)),
                            _ => buffer.bold_colored_block(val, color::RED)
                        }
                    }

                    buffer.finish_line();
                }
            }

            return true
        },
        _ => { return false }
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

fn outgoing(buffer: &mut PromptBuffer, repo: &Repository) -> bool {
    let upstream = git_branch(repo).ok().unwrap().as_slice()[1];
    let revspec = match repo.revparse(upstream) {
        Ok(rs) => rs,
        Err(e) => {
            println!("Err-> {}", e);
            return false
        }
    };
    println!("rev -> {}", revspec.from().unwrap().short_id().ok().unwrap().as_str());
    return false;
}

fn end(buffer: &mut PromptBuffer, repo: &Repository, indented: bool) {
    match git_branch(repo) {
        Ok(branches) => {
            let b = branches.as_slice();

            if b.len() <= 0 {
                buffer.colored_block("New Repository".to_string(), color::CYAN);
            } else if b.len() <= 1 {
                buffer.colored_block(b[0].to_string(), color::CYAN);
            } else if b.len() >= 2 {
                let branch = b[0];
                let remote_branch = b[1];
                buffer.colored_block(format!("{}{} -> {}{}",
                    branch,
                    prompt_buffer::reset(),
                    prompt_buffer::col(color::MAGENTA),
                    remote_branch), color::CYAN);
            } else {
                buffer.colored_block("What???".to_string(), color::RED);
            }

            if indented {
                buffer.indent();
            }

            buffer.finish_line();
        },
        Err(_) => {}
    };
}

pub fn plugin(buffer: &mut PromptBuffer) {
    let repo = get_git();
    if repo.is_ok() {
        let r = repo.ok().unwrap();
        let st = status(buffer, &r);
        let out = outgoing(buffer, &r);
        end(buffer, &r, st || out);
    }
}
