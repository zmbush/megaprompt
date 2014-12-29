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

            buffer.start_boxed()
                .colored_block(&"Git Status", color::CYAN)
                .finish();

            for stat in statuses.iter() {
                let mut line = buffer.start_free();

                let status = GitStatus::new(stat.status());
                let val = format!("{} {}", status, stat.path().unwrap());

                match status.index {
                    StatusTypes::Clean => line.colored_block(&val, file_state_color(status.workdir)),
                    _ => match status.workdir {
                        StatusTypes::Clean | StatusTypes::Untracked =>
                            line.bold_colored_block(&val, file_state_color(status.index)),
                        _ => line.bold_colored_block(&val, color::RED)
                    }
                };

                line.indent().finish();
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

struct BranchInfo {
    name: Option<String>,
    upstream: Option<String>
}

fn git_branch(repo: &Repository) -> Result<BranchInfo, git2::Error> {
    let mut branches = repo.branches(None).ok().expect("Unable to load branches");

    for (mut branch, _) in branches {
        if !branch.is_head() {
            continue;
        }

        let name = branch.name();
        return Ok(BranchInfo {
            name: match name {
                Ok(n) => match n {
                    Some(value) => Some(value.to_string()),
                    _ => None
                },
                _ => None
            },
            upstream: match branch.upstream() {
                Ok(upstream) => {
                    match upstream.name() {
                        Ok(n) => match n {
                            Some(value) => Some(value.to_string()),
                            _ => None
                        },
                        _ => None
                    }
                },
                Err(_) => None
            }
        });
    }

    match repo.head() {
        Ok(r) => match repo.find_object(r.target().unwrap(), None) {
            Ok(obj) => {
                let sid = obj.short_id().ok().unwrap();
                let s = sid.as_str();
                let short_id = s.unwrap();
                Ok(BranchInfo {
                    name: Some(format!("{}", short_id)),
                    upstream: Some("?".to_string())
                })
            },
            Err(e) => Err(e)
        },
        Err(e) => Err(e)
    }
}

fn outgoing(buffer: &mut PromptBuffer, repo: &Repository, has_status: bool) -> bool {
    match do_outgoing(buffer, repo, has_status) {
        Ok(r) => r,
        Err(e) => {
            println!("Error from outgoing: {}", e);
            false
        }
    }
}

fn do_outgoing(buffer: &mut PromptBuffer, repo: &Repository, has_status: bool) -> Result<bool, git2::Error> {
    let branches = try!(git_branch(repo));

    let mut revwalk = try!(repo.revwalk());
    revwalk.set_sorting(git2::SORT_REVERSE);

    let from = try!(repo.revparse_single(branches.upstream.unwrap().as_slice())).id();
    let to = try!(repo.revparse_single(branches.name.unwrap().as_slice())).id();

    try!(revwalk.push(to));
    try!(revwalk.hide(from));

    let mut log_shown = false;

    for id in revwalk {
        let mut commit = try!(repo.find_commit(id));

        if !log_shown {
            buffer.start_boxed()
                .colored_block(&"Git Outgoing", color::CYAN)
                .indent_by(if has_status { 1 } else { 0 })
                .finish();
            log_shown = true;
        }

        buffer.start_free()
            .indent()
            .colored_block(&format!("{} {}",
                String::from_utf8_lossy(
                    try!(try!(repo.find_object(commit.id(), None)).short_id()).get()
                ),
                String::from_utf8_lossy(match commit.summary_bytes() {
                    Some(b) => b,
                    None => continue
                })), color::WHITE)
            .finish();
    }

    return Ok(log_shown);
}

fn end(buffer: &mut PromptBuffer, repo: &Repository, indented: bool) {
    match git_branch(repo) {
        Ok(branches) => {
            buffer.start_boxed()
                .colored_block(
                    &match (branches.name, branches.upstream) {
                        (None, None) => "New Repository".to_string(),
                        (Some(name), None) => name,
                        (Some(name), Some(remote)) => format!("{}{} -> {}{}",
                            name,
                            prompt_buffer::reset(),
                            prompt_buffer::col(color::MAGENTA),
                            remote),
                        _ => "Unknown branch state".to_string()
                    }, color::CYAN)
                .indent_by(if indented { 1 } else { 0 })
                .finish();
        },
        Err(_) => {}
    };
}

pub fn plugin(buffer: &mut PromptBuffer) {
    let repo = get_git();
    if repo.is_ok() {
        let r = repo.ok().unwrap();
        let st = status(buffer, &r);
        let out = outgoing(buffer, &r, st);
        end(buffer, &r, st || out);
    }
}
