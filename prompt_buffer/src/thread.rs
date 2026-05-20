// Copyright 2017 Zoey Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Used to allow a thread per path. This way the cached value can be
//! different based on which path it is running from. For paths with
//! slow `prompt.to_string` outputs, this is particularily useful.
//!
//! Thred will run for 10 minutes after the last request, to avoid
//! leaking too many threads.
use log::info;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::buffer::{PluginSpeed, PromptBuffer};
use crate::error::PromptBufferResult;

/// Stores information about prompt threads
pub struct PromptTask {
    send: Sender<()>,
    recv: Receiver<String>,
    death: Receiver<()>,
    path: PathBuf,
    cached: String,
    alive: bool,
}

impl PromptTask {
    /// Creates a new prompt thread for a given path
    pub async fn new(
        path: PathBuf,
        make_prompt: &dyn Fn() -> PromptBuffer,
    ) -> PromptBufferResult<PromptTask> {
        let (tx_notify, mut rx_notify) = tokio::sync::mpsc::channel(1);
        let (tx_prompt, rx_prompt) = tokio::sync::mpsc::channel(1);
        let (tx_death, rx_death) = tokio::sync::mpsc::channel(1);

        let p = path.clone();
        let mut prompt = make_prompt();
        let cached = prompt.convert_to_string_ext(PluginSpeed::Fast).await;
        tokio::task::spawn_local(async move {
            prompt.set_path(p);
            loop {
                match tokio::time::timeout(Duration::from_mins(10), rx_notify.recv()).await {
                    Ok(_) => tx_prompt
                        .send(prompt.convert_to_string().await)
                        .await
                        .unwrap(),
                    Err(e) => {
                        info!("Task timed out: {e}");
                        tx_death.send(()).await.unwrap();
                        break;
                    }
                }
            }
        });

        Ok(PromptTask {
            send: tx_notify,
            recv: rx_prompt,
            death: rx_death,
            path,
            cached,
            alive: true,
        })
    }

    /// Checks whether a prompt thread has announced it's death.
    pub fn check_is_alive(&mut self) -> bool {
        if let Ok(_) = self.death.try_recv() {
            self.alive = false;
        }
        self.alive
    }

    async fn revive(&mut self, make_prompt: &dyn Fn() -> PromptBuffer) -> PromptBufferResult<()> {
        *self = PromptTask::new(self.path.clone(), make_prompt).await?;
        Ok(())
    }

    /// Gets a result out of the prompt thread, or return a cached result
    /// if the response takes more than 100 milliseconds
    pub async fn get(
        &mut self,
        make_prompt: &dyn Fn() -> PromptBuffer,
    ) -> PromptBufferResult<String> {
        info!("Checking lifesigns");
        if !self.check_is_alive() {
            info!("Thread is not alive. Reviving it");
            self.revive(make_prompt).await?;
        }

        info!("Asking for a new prompt");
        self.send.send(()).await.unwrap();

        match tokio::time::timeout(Duration::from_millis(50), self.recv.recv()).await {
            Ok(text) => {
                if let Some(t) = text {
                    self.cached = t;
                }
            }
            Err(_) => {
                info!("Got timeout");
            }
        }

        Ok(self.cached.clone())
    }
}
