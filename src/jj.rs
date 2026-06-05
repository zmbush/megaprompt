// Copyright 2026 Zoey Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(unused)]

use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use eyre::Context as _;
use futures::StreamExt as _;
use jj_cli::{
    cli_util::CliRunner,
    config::{ConfigEnv, config_from_environment, default_config_layers},
};
use jj_lib::{
    backend::Backend,
    git_backend::GitBackend,
    local_working_copy::{LocalWorkingCopy, LocalWorkingCopyFactory, TreeState, TreeStateSettings},
    matchers::{EverythingMatcher, Matcher},
    merge::Diff,
    repo::{Repo, RepoLoader, StoreFactories},
    settings::UserSettings,
    signing::Signer,
    store::Store,
    tree,
    tree_merge::MergeOptions,
    working_copy::WorkingCopy,
    workspace::{WorkingCopyFactories, Workspace},
};
use prompt_buffer::{PromptBufferPlugin, PromptLines};
use term::color;
use tokio::process::Command;

use crate::git::RelativePath as _;

#[derive(Default)]
pub struct JujutsuPlugin {}

#[derive(clap::Subcommand)]
enum Megaprompt {}

#[async_trait::async_trait(?Send)]
impl PromptBufferPlugin for JujutsuPlugin {
    #[allow(unused)]
    async fn run(
        &mut self,
        _speed: prompt_buffer::PluginSpeed,
        shell: prompt_buffer::ShellType,
        path: &std::path::Path,
    ) -> Result<PromptLines, eyre::Report> {
        let mut lines = PromptLines::new();
        let mut relative = PathBuf::new();
        let mut current = path.to_owned();
        let (jj_path, root) = loop {
            let dir = current.join(".jj/");
            if dir.is_dir() {
                break (dir, current);
            }

            match current.parent() {
                Some(parent) => {
                    relative.push("../");
                    current = parent.to_owned();
                }
                None => return Ok(vec![]),
            }
        };

        let mut raw_config = config_from_environment(default_config_layers());
        let config_env = ConfigEnv::from_environment();
        config_env.reload_user_config(&mut raw_config).ok();
        let mut config = config_env.resolve_config(&raw_config)?;
        let settings = UserSettings::from_config(config)?;
        let mut store_factories = StoreFactories::default();
        let mut working_copy_factories = WorkingCopyFactories::new();
        working_copy_factories.insert("local".to_string(), Box::new(LocalWorkingCopyFactory {}));
        let workspace =
            Workspace::load(&settings, &root, &store_factories, &working_copy_factories)
                .context("boo")?;
        let repo = workspace.repo_loader().load_at_head().await?;
        let view = repo.view();
        let commit = view
            .get_wc_commit_id(workspace.workspace_name())
            .map(|id| repo.store().get_commit(id))
            .transpose()?
            .ok_or(eyre::eyre!("No commit for current workspace"))?;
        let parent = commit.parent_tree(&*repo).await?;
        let tree = commit.tree();

        lines.push(
            shell
                .new_line()
                .colored_block("Jujutsu", color::YELLOW)
                .build(),
        );
        if tree.tree_ids() != parent.tree_ids() {
            let mut stream = parent.diff_stream(&tree, &EverythingMatcher);
            while let Some(diff) = stream.next().await {
                lines.push(
                    shell
                        .new_free_line()
                        .colored_block(
                            format!(
                                "{}",
                                diff.path
                                    .to_fs_path_unchecked(&relative)
                                    .canonicalize()?
                                    .make_relative(path)
                                    .ok_or_else(|| eyre::eyre!("Failed to make path relative"))?
                                    .display()
                            ),
                            color::BLUE,
                        )
                        .indent()
                        .build(),
                );
            }
        }

        for (name, target) in view.local_bookmarks_for_commit(commit.id()) {
            lines.push(
                shell
                    .new_line()
                    .colored_block(name.as_str().to_string(), color::GREEN)
                    .indent()
                    .build(),
            );
        }
        let mut ancestors = commit
            .parents()
            .await?
            .iter()
            .map(|parent| (1, parent.clone()))
            .collect::<VecDeque<_>>();
        while let Some((depth, ancestor)) = ancestors.pop_front() {
            if depth > 10 {
                break;
            }
            let bookmarks = view
                .local_bookmarks_for_commit(ancestor.id())
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>();
            if bookmarks.is_empty() {
                ancestors.extend(
                    ancestor
                        .parents()
                        .await?
                        .iter()
                        .map(|parent| (depth + 1, parent.clone())),
                );
                continue;
            }
            lines.push(
                shell
                    .new_line()
                    .colored_block(
                        format!(
                            "{} (+{} commit{})",
                            bookmarks.join(", "),
                            depth,
                            if depth > 1 { "s" } else { "" }
                        ),
                        color::GREEN,
                    )
                    .indent()
                    .build(),
            )
        }

        Ok(lines)
    }
}
