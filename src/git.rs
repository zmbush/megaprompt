// Copyright 2017 Zachary Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate git2;
// extern crate term;

use git2::{Error, Repository, StatusOptions};
use prompt_buffer::{PluginSpeed, PromptBufferPlugin, PromptLines, ShellType};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::{env, fmt};
use term::color;

trait RelativePath: Sized {
    fn make_relative(self, base: &Path) -> Option<Self>;
}

impl RelativePath for PathBuf {
    fn make_relative(self, base: &Path) -> Option<PathBuf> {
        if self.starts_with(base) {
            Some(
                self.strip_prefix(base)
                    .expect("starts_with is a liar")
                    .to_path_buf(),
            )
        } else {
            let mut b = base.to_path_buf();
            if b.pop() {
                self.make_relative(&b).map(|s| Path::new("..").join(s))
            } else {
                None
            }
        }
    }
}

enum StatusTypes {
    New,
    Modified,
    Deleted,
    Renamed,
    TypeChange,
    Untracked,
    Clean,
}

impl fmt::Display for StatusTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                StatusTypes::Clean => " ",
                StatusTypes::Deleted => "D",
                StatusTypes::Modified => "M",
                StatusTypes::New => "A",
                StatusTypes::Renamed => "R",
                StatusTypes::TypeChange => "T",
                StatusTypes::Untracked => "?",
            }
        )
    }
}

struct GitStatus {
    index: StatusTypes,
    workdir: StatusTypes,
}

impl GitStatus {
    fn new(f: git2::Status) -> GitStatus {
        GitStatus {
            index: if f.contains(git2::Status::INDEX_NEW) {
                StatusTypes::New
            } else if f.contains(git2::Status::INDEX_MODIFIED) {
                StatusTypes::Modified
            } else if f.contains(git2::Status::INDEX_DELETED) {
                StatusTypes::Deleted
            } else if f.contains(git2::Status::INDEX_RENAMED) {
                StatusTypes::Renamed
            } else if f.contains(git2::Status::INDEX_TYPECHANGE) {
                StatusTypes::TypeChange
            } else if f.contains(git2::Status::WT_NEW) {
                StatusTypes::Untracked
            } else {
                StatusTypes::Clean
            },
            workdir: if f.contains(git2::Status::WT_NEW) {
                StatusTypes::Untracked
            } else if f.contains(git2::Status::WT_MODIFIED) {
                StatusTypes::Modified
            } else if f.contains(git2::Status::WT_DELETED) {
                StatusTypes::Deleted
            } else if f.contains(git2::Status::WT_RENAMED) {
                StatusTypes::Renamed
            } else if f.contains(git2::Status::WT_TYPECHANGE) {
                StatusTypes::TypeChange
            } else {
                StatusTypes::Clean
            },
        }
    }
}

impl fmt::Display for GitStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.index, self.workdir)
    }
}

fn get_git(path: &Path) -> Option<Repository> {
    Repository::discover(path).ok()
}

struct BranchInfo {
    name: Option<String>,
    upstream: Option<String>,
}

fn git_branch(repo: &Repository) -> Result<BranchInfo, Error> {
    let branches = repo.branches(None).expect("Unable to load branches");

    for possible_branch in branches {
        let branch = match possible_branch {
            Ok((b, _)) => b,
            Err(_) => continue,
        };

        if !branch.is_head() {
            continue;
        }

        let name = branch.name();
        return Ok(BranchInfo {
            name: match name {
                Ok(n) => n.map(|value| value.to_owned()),
                _ => None,
            },
            upstream: match branch.upstream() {
                Ok(upstream) => match upstream.name() {
                    Ok(n) => n.map(|value| value.to_owned()),
                    _ => None,
                },
                Err(_) => None,
            },
        });
    }

    match repo.head() {
        Ok(r) => match repo.find_object(r.target().expect("Unable to find target"), None) {
            Ok(obj) => {
                let sid = obj.short_id().expect("Object has no short_id");
                let s = sid.as_str();
                let short_id = s.expect("Unable to convert short id to string");
                Ok(BranchInfo {
                    name: Some(short_id.to_owned()),
                    upstream: Some("?".to_owned()),
                })
            }
            Err(e) => Err(e),
        },
        Err(e) => Err(e),
    }
}

pub struct GitPlugin {
    repo: Option<Repository>,
    path: PathBuf,
}

impl Default for GitPlugin {
    fn default() -> GitPlugin {
        GitPlugin {
            repo: None,
            path: env::current_dir().expect("There is no current directory!"),
        }
    }
}

impl GitPlugin {
    pub fn new() -> GitPlugin {
        GitPlugin::default()
    }

    fn get_repo(&self) -> Result<&Repository, Error> {
        match self.repo {
            Some(ref repo) => Ok(repo),
            None => Err(Error::from_str("No repo")),
        }
    }

    fn status(
        &self,
        shell: ShellType,
        buffer: &mut PromptLines,
        path: &Path,
    ) -> Result<bool, Error> {
        fn file_state_color(state: &StatusTypes) -> u32 {
            match *state {
                StatusTypes::Clean | StatusTypes::Untracked => color::WHITE,
                StatusTypes::Deleted => color::RED,
                StatusTypes::Modified => color::BLUE,
                StatusTypes::New => color::GREEN,
                StatusTypes::Renamed => color::CYAN,
                StatusTypes::TypeChange => color::YELLOW,
            }
        }

        let repo = self.get_repo()?;

        let st = repo.statuses(Some(
            StatusOptions::new()
                .include_untracked(true)
                .renames_head_to_index(true),
        ));

        let make_path_relative = |current: &Path| {
            let mut fullpath = repo
                .workdir()
                .expect("Repo has no working dir")
                .to_path_buf();
            fullpath.push(current);
            fullpath
                .make_relative(path)
                .unwrap_or_else(|| PathBuf::from("/"))
        };

        if let Ok(statuses) = st {
            if statuses.is_empty() {
                return Ok(false);
            }

            buffer.push(
                shell
                    .new_line()
                    .colored_block("Git Status", color::CYAN)
                    .build(),
            );

            for stat in statuses.iter() {
                let mut line = shell.new_free_line();

                let status = GitStatus::new(stat.status());

                let diff = match stat.head_to_index() {
                    Some(delta) => Some(delta),
                    None => stat.index_to_workdir(),
                };

                let val = format!(
                    "{} {}",
                    status,
                    match diff {
                        Some(delta) => {
                            let old =
                                make_path_relative(delta.old_file().path().expect("no old file"));
                            let new =
                                make_path_relative(delta.new_file().path().expect("no new file"));

                            if old == new {
                                format!("{}", old.display())
                            } else {
                                format!("{} -> {}", old.display(), new.display())
                            }
                        }
                        None => format!(
                            "{}",
                            Path::new(stat.path().expect("No status path")).display()
                        ),
                    }
                );

                line = match status.index {
                    StatusTypes::Clean => {
                        line.colored_block(val, file_state_color(&status.workdir))
                    }
                    _ => match status.workdir {
                        StatusTypes::Clean | StatusTypes::Untracked => {
                            line.bold_colored_block(val, file_state_color(&status.index))
                        }
                        _ => line.bold_colored_block(val, color::RED),
                    },
                };

                buffer.push(line.indent().build());
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn outgoing(
        &self,
        shell: ShellType,
        buffer: &mut PromptLines,
        has_status: bool,
    ) -> Result<bool, Error> {
        let repo = self.get_repo()?;

        let branches = git_branch(repo)?;

        let mut revwalk = repo.revwalk()?;

        let from = repo
            .revparse_single(
                branches
                    .upstream
                    .unwrap_or_else(|| "HEAD".to_owned())
                    .as_ref(),
            )?
            .id();
        let to = repo
            .revparse_single(branches.name.unwrap_or_else(|| "HEAD".to_owned()).as_ref())?
            .id();

        revwalk.push(to)?;
        revwalk.hide(from)?;

        let mut log_shown = false;

        for possible_id in revwalk {
            let id = match possible_id {
                Ok(id) => id,
                Err(_) => continue,
            };

            let commit = repo.find_commit(id)?;

            if !log_shown {
                buffer.push(
                    shell
                        .new_line()
                        .colored_block("Git Outgoing", color::CYAN)
                        .indent_by(if has_status { 1 } else { 0 })
                        .build(),
                );
                log_shown = true;
            }

            buffer.push(
                shell
                    .new_free_line()
                    .indent()
                    .block(format!(
                        "{}{} {}",
                        shell.reset(),
                        String::from_utf8_lossy(
                            repo.find_object(commit.id(), None)?.short_id()?.deref()
                        ),
                        String::from_utf8_lossy(match commit.summary_bytes() {
                            Some(b) => b,
                            None => continue,
                        })
                    ))
                    .build(),
            );
        }

        Ok(log_shown)
    }

    fn end(
        &self,
        shell: ShellType,
        buffer: &mut PromptLines,
        indented: bool,
    ) -> Result<bool, Error> {
        let repo = self.get_repo()?;

        let branches = git_branch(repo)?;

        buffer.push(
            shell
                .new_line()
                .colored_block(
                    match (branches.name, branches.upstream) {
                        (None, None) => "New Repository".to_owned(),
                        (Some(name), None) => name,
                        (Some(name), Some(remote)) => format!(
                            "{}{} -> {}{}",
                            name,
                            shell.reset(),
                            shell.col(color::MAGENTA),
                            remote
                        ),
                        _ => "Unknown branch state".to_owned(),
                    },
                    color::CYAN,
                )
                .indent_by(if indented { 1 } else { 0 })
                .build(),
        );

        Ok(true)
    }
}

impl PromptBufferPlugin for GitPlugin {
    fn run(&mut self, speed: PluginSpeed, shell: ShellType, path: &Path, lines: &mut PromptLines) {
        if self.path != *path || self.repo.is_none() {
            self.path = path.into();
            self.repo = get_git(&self.path);
        }

        let st = match speed {
            PluginSpeed::Slow => {
                trace!("Finding git status");
                self.status(shell, lines, path).ok().unwrap_or(false)
            }
            _ => false,
        };
        trace!("Finding outgoing commits");
        let out = self.outgoing(shell, lines, st).ok().unwrap_or(false);
        let _ = self.end(shell, lines, st || out).ok();
    }
}
