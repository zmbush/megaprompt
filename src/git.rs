extern crate git2;

use prompt_buffer;
use prompt_buffer::PromptBuffer;
use git2::{Repository, Error, StatusOptions, STATUS_WT_NEW};
use std::{os, fmt};
use term::color;

enum StatusTypes {
    New,
    Modified,
    Deleted,
    Renamed,
    TypeChange,
    Untracked,
    Clean
}

impl fmt::Show for StatusTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            StatusTypes::New => "A",
            StatusTypes::Modified => "M",
            StatusTypes::Deleted => "D",
            StatusTypes::Renamed => "R",
            StatusTypes::TypeChange => "T",
            StatusTypes::Untracked => "?",
            StatusTypes::Clean => " "
        })
    }
}

struct GitStatus {
    index: StatusTypes,
    workdir: StatusTypes
}

impl GitStatus {
    fn new(f: git2::Status) -> GitStatus {
        GitStatus {
            index:
                     if f.contains(git2::STATUS_INDEX_NEW) { StatusTypes::New }
                else if f.contains(git2::STATUS_INDEX_MODIFIED) { StatusTypes::Modified }
                else if f.contains(git2::STATUS_INDEX_DELETED) { StatusTypes::Deleted }
                else if f.contains(git2::STATUS_INDEX_RENAMED) { StatusTypes::Renamed }
                else if f.contains(git2::STATUS_INDEX_TYPECHANGE) { StatusTypes::TypeChange }
                else if f.contains(git2::STATUS_WT_NEW) { StatusTypes::Untracked }
                else { StatusTypes::Clean },
            workdir:
                     if f.contains(git2::STATUS_WT_NEW) { StatusTypes::Untracked }
                else if f.contains(git2::STATUS_WT_MODIFIED) { StatusTypes::Modified }
                else if f.contains(git2::STATUS_WT_DELETED) { StatusTypes::Deleted }
                else if f.contains(git2::STATUS_WT_RENAMED) { StatusTypes::Renamed }
                else if f.contains(git2::STATUS_WT_TYPECHANGE) { StatusTypes::TypeChange }
                else { StatusTypes::Clean },
        }
    }
}

impl fmt::Show for GitStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.index, self.workdir)
    }
}

fn get_git() -> Result<Repository, Error> {
    let path = os::make_absolute(&Path::new(".")).unwrap();
    return Repository::discover(&path);
}

fn status(buffer: &mut PromptBuffer, repo: &Repository) -> bool {
    let st = repo.statuses(Some(StatusOptions::new()
        .include_untracked(true)
        .renames_head_to_index(true)
    ));
    match st {
        Ok(statuses) => {
            if statuses.len() <= 0 { return false }

            buffer.colored_block(&"Git Status", color::CYAN);
            buffer.finish_line();
            for stat in statuses.iter() {
                buffer.make_free();
                buffer.indent();

                let status = GitStatus::new(stat.status());
                let val = format!("{} {}", status, stat.path().unwrap());

                match status.index {
                    StatusTypes::Clean => buffer.colored_block(&val, file_state_color(status.workdir)),
                    _ => match status.workdir {
                        StatusTypes::Clean | StatusTypes::Untracked =>
                            buffer.bold_colored_block(&val, file_state_color(status.index)),
                        _ => buffer.bold_colored_block(&val, color::RED)
                    }
                }

                buffer.finish_line();
            }

            return true
        },
        _ => { return false }
    }

    fn file_state_color(state: StatusTypes) -> u16 {
        match state {
            StatusTypes::Clean | StatusTypes::Untracked => color::WHITE,
            StatusTypes::Deleted => color::RED,
            StatusTypes::Modified => color::BLUE,
            StatusTypes::New => color::GREEN,
            StatusTypes::Renamed => color::CYAN,
            StatusTypes::TypeChange => color::YELLOW,
        }
    }
}

fn git_branch(repo: &Repository) -> Result<Vec<String>, String> {
    let mut branches = repo.branches(None).ok().expect("Unable to load branches");

    for (mut branch, _) in branches {
        if !branch.is_head() {
            continue;
        }

        let mut result = Vec::new();

        let name = branch.name();

        match name {
            Ok(n) => match n {
                Some(value) => {
                    result.push(value.to_string())
                },
                None => {}
            },
            Err(_) => {}
        };

        match branch.upstream() {
            Ok(upstream) => {
                match upstream.name() {
                    Ok(n) => match n {
                        Some(value) => result.push(value.to_string()),
                        None => {}
                    },
                    Err(_) => {}
                }
            }
            Err(_) => {}
        };

        return Ok(result);
    }

    match repo.head() {
        Ok(r) => match repo.find_object(r.target().unwrap(), None) {
            Ok(obj) => {
                let sid = obj.short_id().ok().unwrap();
                let s = sid.as_str();
                let short_id = s.unwrap();
                let mut retval = Vec::new();
                retval.push(format!("{}", short_id));
                retval.push("?".to_string());
                Ok(retval)
            },
            _ => Err("BOOT".to_string())
        },
        Err(_) => Err("No active branch".to_string())
    }
}

fn outgoing(buffer: &mut PromptBuffer, repo: &Repository) -> bool {
    let branches = git_branch(repo).ok().unwrap();
    if branches.len() <= 1 { return false }

    let revspec = match repo.refname_to_id("HEAD") {
        Ok(rs) => rs,
        _ => return false
    };
    return false;
}

fn end(buffer: &mut PromptBuffer, repo: &Repository, indented: bool) {
    match git_branch(repo) {
        Ok(branches) => {
            let b = branches.as_slice();

            if b.len() <= 0 {
                buffer.colored_block(&"New Repository", color::CYAN);
            } else if b.len() <= 1 {
                buffer.colored_block(&b[0], color::CYAN);
            } else if b.len() >= 2 {
                let branch = &b[0];
                let remote_branch = &b[1];
                buffer.colored_block(&format!("{}{} -> {}{}",
                    branch,
                    prompt_buffer::reset(),
                    prompt_buffer::col(color::MAGENTA),
                    remote_branch), color::CYAN);
            } else {
                buffer.colored_block(&"What???", color::RED);
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
