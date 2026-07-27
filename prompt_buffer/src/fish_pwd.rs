// Copyright 2026 Zoey Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A port of fish's `prompt_pwd`.
//!
//! Bash and zsh expand `\w` and `%~` themselves, so for those shells we only have to emit
//! the escape. Fish performs no prompt expansion at all, so we have to produce the
//! shortened path ourselves. This reproduces what `prompt_pwd` does, so a fish prompt
//! built by megaprompt looks like every other fish prompt.

use std::borrow::Cow;
use std::env;
use std::path::Path;

/// Default for `$fish_prompt_pwd_dir_length`.
const DEFAULT_DIR_LENGTH: usize = 1;

/// Default for `$fish_prompt_pwd_full_dirs`.
const DEFAULT_FULL_DIRS: usize = 1;

/// Shortens `path` the way fish's `prompt_pwd` would.
///
/// `$fish_prompt_pwd_dir_length` and `$fish_prompt_pwd_full_dirs` are fish variables, so
/// they are only visible here if they have been exported (`set -gx`). When they aren't,
/// fish's own defaults are used.
pub(crate) fn prompt_pwd(path: &Path) -> String {
    let home = env::var("HOME").ok();

    prompt_pwd_with(
        path,
        home.as_deref(),
        env_usize("fish_prompt_pwd_dir_length", DEFAULT_DIR_LENGTH),
        env_usize("fish_prompt_pwd_full_dirs", DEFAULT_FULL_DIRS),
    )
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

/// The part of [`prompt_pwd`] that doesn't read the environment, so it can be tested.
fn prompt_pwd_with(path: &Path, home: Option<&str>, dir_length: usize, full_dirs: usize) -> String {
    // Drop control characters so that a directory name can't inject escape sequences into
    // the prompt. fish does the same, for the same reason.
    let cleaned: String = unexpand_tilde(path, home)
        .chars()
        .filter(|c| !c.is_control())
        .collect();

    if dir_length == 0 {
        return cleaned;
    }

    let components: Vec<&str> = cleaned.split('/').collect();
    let shorten_upto = components.len() - full_dirs.min(components.len());

    components
        .iter()
        .enumerate()
        .map(|(ix, component)| {
            if ix < shorten_upto {
                Cow::Owned(shorten(component, dir_length))
            } else {
                Cow::Borrowed(*component)
            }
        })
        .collect::<Vec<Cow<'_, str>>>()
        .join("/")
}

/// Replaces a leading `$HOME` with `~`, matching fish's `__fish_unexpand_tilde`.
///
/// Only replaces on a component boundary, so `/Users/zoeyX` is left alone when `$HOME` is
/// `/Users/zoey`.
fn unexpand_tilde(path: &Path, home: Option<&str>) -> String {
    let path = path.to_string_lossy();

    if let Some(home) = home.filter(|home| !home.is_empty()) {
        if path == home {
            return "~".to_owned();
        }

        if let Some(rest) = path.strip_prefix(home).filter(|rest| rest.starts_with('/')) {
            return format!("~{rest}");
        }
    }

    path.into_owned()
}

/// Shortens a single path component.
///
/// fish uses `\.?[^/]{N}`, which keeps a leading dot *in addition to* `dir_length`
/// characters -- so `.bin` becomes `.b` and `..foo` becomes `..`. The count is in
/// characters rather than bytes, because fish's PCRE2 runs in UTF mode.
fn shorten(component: &str, dir_length: usize) -> String {
    let mut chars = component.chars();
    let mut shortened = String::new();

    if component.starts_with('.') {
        shortened.push('.');
        let _ = chars.next();
    }

    shortened.extend(chars.take(dir_length));
    shortened
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/Users/zbush";

    fn pwd(path: &str) -> String {
        prompt_pwd_with(Path::new(path), Some(HOME), 1, 1)
    }

    fn pwd_ext(path: &str, dir_length: usize, full_dirs: usize) -> String {
        prompt_pwd_with(Path::new(path), Some(HOME), dir_length, full_dirs)
    }

    /// Captured from `prompt_pwd` under fish 4.9.3.
    #[test]
    fn matches_fish_prompt_pwd() {
        assert_eq!(pwd("/Users/zbush"), "~");
        assert_eq!(pwd("/"), "/");
        assert_eq!(pwd("/usr/local/share/man"), "/u/l/s/man");
        assert_eq!(pwd("/Users/zbush/a/bb/ccc/dddd"), "~/a/b/c/dddd");
        assert_eq!(pwd("/Users/zbush/Projects/"), "~/P/");
    }

    #[test]
    fn only_unexpands_on_a_component_boundary() {
        assert_eq!(pwd("/Users/zbushX/foo"), "/U/z/foo");
    }

    #[test]
    fn keeps_a_leading_dot_in_addition_to_dir_length() {
        assert_eq!(
            pwd("/Users/zbush/.bin/megaprompt/prompt_buffer/src"),
            "~/.b/m/p/src"
        );
        assert_eq!(pwd("/Users/zbush/..foo/bar/baz"), "~/../b/baz");
    }

    #[test]
    fn leaves_the_last_component_alone() {
        assert_eq!(pwd("/Users/zbush/.ssh"), "~/.ssh");
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert_eq!(pwd("/Users/zbush/ünïcode/dir/last"), "~/ü/d/last");
    }

    #[test]
    fn strips_control_characters() {
        assert_eq!(pwd("/Users/zbush/a\x1b[31mb/last"), "~/a/last");
    }

    #[test]
    fn honors_dir_length() {
        assert_eq!(
            pwd_ext("/Users/zbush/a/bb/cccc/dddd", 0, 1),
            "~/a/bb/cccc/dddd"
        );
        assert_eq!(
            pwd_ext("/Users/zbush/aaaa/bbbb/cccc/dddd", 2, 1),
            "~/aa/bb/cc/dddd"
        );
    }

    #[test]
    fn honors_full_dirs() {
        assert_eq!(
            pwd_ext("/Users/zbush/aaaa/bbbb/cccc/dddd", 1, 0),
            "~/a/b/c/d"
        );
        assert_eq!(
            pwd_ext("/Users/zbush/aaaa/bbbb/cccc/dddd", 1, 2),
            "~/a/b/cccc/dddd"
        );
    }

    #[test]
    fn falls_back_to_the_full_path_without_home() {
        assert_eq!(
            prompt_pwd_with(Path::new("/Users/zbush/foo"), None, 1, 1),
            "/U/z/foo"
        );
    }
}
