extern crate git2;
extern crate term;

use prompt_buffer::escape;
use prompt_buffer::buffer::{PromptBufferPlugin, PluginSpeed};
use prompt_buffer::line::{PromptLines, PromptLineBuilder};
use git2::{Repository, Error, StatusOptions};
use std::{fmt, env};
use term::color;
use std::ops::Deref;
use std::path::{Path, PathBuf};

trait RelativePath: Sized {
    fn make_relative(self, base: &Path) -> Option<Self>;
}

impl RelativePath for PathBuf {
    fn make_relative(self, base: &Path) -> Option<PathBuf> {
        if self.starts_with(base) {
            Some(self.strip_prefix(base).expect("starts_with is a liar").to_path_buf())
        } else {
            let mut b = base.to_path_buf();
            if b.pop() {
                match self.make_relative(&b) {
                    Some(s) => Some(Path::new("..").join(s)),
                    None => None,
                }
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
        write!(f,
               "{}",
               match *self {
                   StatusTypes::Clean => " ",
                   StatusTypes::Deleted => "D",
                   StatusTypes::Modified => "M",
                   StatusTypes::New => "A",
                   StatusTypes::Renamed => "R",
                   StatusTypes::TypeChange => "T",
                   StatusTypes::Untracked => "?",
               })
    }
}

struct GitStatus {
    index: StatusTypes,
    workdir: StatusTypes,
}

impl GitStatus {
    fn new(f: git2::Status) -> GitStatus {
        GitStatus {
            index: if f.contains(git2::STATUS_INDEX_NEW) {
                StatusTypes::New
            } else if f.contains(git2::STATUS_INDEX_MODIFIED) {
                StatusTypes::Modified
            } else if f.contains(git2::STATUS_INDEX_DELETED) {
                StatusTypes::Deleted
            } else if f.contains(git2::STATUS_INDEX_RENAMED) {
                StatusTypes::Renamed
            } else if f.contains(git2::STATUS_INDEX_TYPECHANGE) {
                StatusTypes::TypeChange
            } else if f.contains(git2::STATUS_WT_NEW) {
                StatusTypes::Untracked
            } else {
                StatusTypes::Clean
            },
            workdir: if f.contains(git2::STATUS_WT_NEW) {
                StatusTypes::Untracked
            } else if f.contains(git2::STATUS_WT_MODIFIED) {
                StatusTypes::Modified
            } else if f.contains(git2::STATUS_WT_DELETED) {
                StatusTypes::Deleted
            } else if f.contains(git2::STATUS_WT_RENAMED) {
                StatusTypes::Renamed
            } else if f.contains(git2::STATUS_WT_TYPECHANGE) {
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
                Ok(n) => {
                    match n {
                        Some(value) => Some(value.to_owned()),
                        _ => None,
                    }
                }
                _ => None,
            },
            upstream: match branch.upstream() {
                Ok(upstream) => {
                    match upstream.name() {
                        Ok(n) => {
                            match n {
                                Some(value) => Some(value.to_owned()),
                                _ => None,
                            }
                        }
                        _ => None,
                    }
                }
                Err(_) => None,
            },
        });
    }

    match repo.head() {
        Ok(r) => {
            match repo.find_object(r.target().expect("Unable to find target"), None) {
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
            }
        }
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

    #[allow(explicit_iter_loop)]
    fn status(&self, buffer: &mut PromptLines, path: &Path) -> Result<bool, Error> {
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

        let repo = try!(self.get_repo());

        let st = repo.statuses(Some(StatusOptions::new()
                                        .include_untracked(true)
                                        .renames_head_to_index(true)));

        let make_path_relative = |current: &Path| {
            let mut fullpath = repo.workdir().expect("Repo has no working dir").to_path_buf();
            fullpath.push(current);
            fullpath.make_relative(path).unwrap_or_else(|| PathBuf::from("/"))
        };

        if let Ok(statuses) = st {
            if statuses.len() == 0 {
                return Ok(false);
            }

            buffer.push(PromptLineBuilder::new()
                            .colored_block("Git Status", color::CYAN)
                            .build());

            for stat in statuses.iter() {
                let mut line = PromptLineBuilder::new_free();

                let status = GitStatus::new(stat.status());

                let diff = match stat.head_to_index() {
                    Some(delta) => Some(delta),
                    None => {
                        match stat.index_to_workdir() {
                            Some(delta) => Some(delta),
                            None => None,
                        }
                    }
                };

                let val = format!("{} {}",
                                  status,
                                  match diff {
                                      Some(delta) => {
                                          let old = make_path_relative(delta.old_file()
                                                                            .path()
                                                                            .expect("no old file"));
                                          let new = make_path_relative(delta.new_file()
                                                                            .path()
                                                                            .expect("no new file"));

                                          if old == new {
                                              format!("{}", old.display())
                                          } else {
                                              format!("{} -> {}", old.display(), new.display())
                                          }
                                      }
                                      None => {
                                          format!("{}",
                                                  Path::new(stat.path().expect("No status path"))
                                                      .display())
                                      }
                                  });

                line = match status.index {
                    StatusTypes::Clean => line.colored_block(val, file_state_color(status.workdir)),
                    _ => {
                        match status.workdir {
                            StatusTypes::Clean | StatusTypes::Untracked => {
                                line.bold_colored_block(val, file_state_color(status.index))
                            }
                            _ => line.bold_colored_block(val, color::RED),
                        }
                    }
                };

                buffer.push(line.indent().build());
            }

            return Ok(true);
        } else {
            return Ok(false);
        }

    }

    fn outgoing(&self, buffer: &mut PromptLines, has_status: bool) -> Result<bool, Error> {
        let repo = try!(self.get_repo());

        let branches = try!(git_branch(repo));

        let mut revwalk = try!(repo.revwalk());

        let from = try!(repo.revparse_single(branches.upstream
                                                     .unwrap_or("HEAD".to_owned())
                                                     .as_ref()))
                       .id();
        let to = try!(repo.revparse_single(branches.name.unwrap_or("HEAD".to_owned()).as_ref()))
                     .id();

        try!(revwalk.push(to));
        try!(revwalk.hide(from));

        let mut log_shown = false;

        for possible_id in revwalk {
            let id = match possible_id {
                Ok(id) => id,
                Err(_) => continue,
            };

            let mut commit = try!(repo.find_commit(id));

            if !log_shown {
                buffer.push(PromptLineBuilder::new()
                                .colored_block("Git Outgoing", color::CYAN)
                                .indent_by(if has_status {
                                    1
                                } else {
                                    0
                                })
                                .build());
                log_shown = true;
            }

            buffer.push(PromptLineBuilder::new_free()
                            .indent()
                            .block(format!("{}{} {}",
                    escape::reset(),
                    String::from_utf8_lossy(
                        try!(
                            try!(
                                repo.find_object(commit.id(), None)
                            ).short_id()
                        ).deref()
                    ),
                    String::from_utf8_lossy(match commit.summary_bytes() {
                        Some(b) => b,
                        None => continue
                    })))
                            .build());
        }

        Ok(log_shown)
    }

    fn end(&self, buffer: &mut PromptLines, indented: bool) -> Result<bool, Error> {
        let repo = try!(self.get_repo());

        let branches = try!(git_branch(repo));

        buffer.push(PromptLineBuilder::new()
                        .colored_block(&match (branches.name, branches.upstream) {
                                           (None, None) => "New Repository".to_owned(),
                                           (Some(name), None) => name,
                                           (Some(name), Some(remote)) => {
                                               format!("{}{} -> {}{}",
                                                       name,
                                                       escape::reset(),
                                                       escape::col(color::MAGENTA),
                                                       remote)
                                           }
                                           _ => "Unknown branch state".to_owned(),
                                       },
                                       color::CYAN)
                        .indent_by(if indented {
                            1
                        } else {
                            0
                        })
                        .build());

        Ok(true)
    }
}

impl PromptBufferPlugin for GitPlugin {
    fn run(&mut self, speed: &PluginSpeed, path: &PathBuf, lines: &mut PromptLines) {
        if self.path != *path || self.repo.is_none() {
            self.path = path.clone();
            self.repo = get_git(&self.path);
        }

        let st = match *speed {
            PluginSpeed::Slow => {
                trace!("Finding git status");
                self.status(lines, path).ok().unwrap_or(false)
            }
            _ => false,
        };
        trace!("Finding outgoing commits");
        let out = self.outgoing(lines, st).ok().unwrap_or(false);
        let _ = self.end(lines, st || out).ok();
    }
}
