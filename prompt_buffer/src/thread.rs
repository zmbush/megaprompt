// Copyright 2017 Zachary Bush.
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
use std::time::Duration;
use std::thread;
use chan::{self, Receiver, Sender};
use std::path::PathBuf;

use buffer::{PluginSpeed, PromptBuffer};
use error::PromptBufferResult;

/// Stores information about prompt threads
pub struct PromptThread {
    send: Sender<()>,
    recv: Receiver<String>,
    death: Receiver<()>,
    path: PathBuf,
    cached: String,
    alive: bool,
}

fn oneshot_timer(dur: Duration) -> Receiver<()> {
    let (tx, rx) = chan::async();

    thread::spawn(move || {
        thread::sleep(dur);

        let _ = tx.send(());
    });

    rx
}

impl PromptThread {
    /// Creates a new prompt thread for a given path
    pub fn new(
        path: PathBuf,
        make_prompt: &Fn() -> PromptBuffer,
    ) -> PromptBufferResult<PromptThread> {
        let (tx_notify, rx_notify) = chan::async();
        let (tx_prompt, rx_prompt) = chan::async();
        let (tx_death, rx_death) = chan::async();

        let p = path.clone();
        let mut prompt = make_prompt();
        let cached = prompt.convert_to_string_ext(PluginSpeed::Fast);
        let name = format!("{}", path.display());
        try!(thread::Builder::new().name(name.to_owned()).spawn(
            move || {
                prompt.set_path(p);

                loop {
                    let timeout = oneshot_timer(Duration::from_secs(10 * 60));

                    // Weird issue with stuff... Not sure yet...
                    #[allow(unused_mut)]
                    {
                        chan_select! {
                            rx_notify.recv() => {
                                tx_prompt.send(prompt.convert_to_string())
                            },
                            timeout.recv() => {
                                info!("Thread {} timed out", name);
                                let _ = tx_death.send(());
                                break;
                            }
                        }
                    }
                }
            }
        ));

        Ok(PromptThread {
            send: tx_notify,
            recv: rx_prompt,
            death: rx_death,
            path: path,
            cached: cached,
            alive: true,
        })
    }

    /// Checks whether a prompt thread has announced it's death.
    pub fn check_is_alive(&mut self) -> bool {
        let ref death = self.death;
        #[allow(unused_mut)]
        {
            chan_select! {
                default => {},
                death.recv() =>{
                    self.alive = false;
                },
            }
        }

        self.alive
    }

    fn revive(&mut self, make_prompt: &Fn() -> PromptBuffer) -> PromptBufferResult<()> {
        *self = try!(PromptThread::new(self.path.clone(), make_prompt));
        Ok(())
    }

    /// Gets a result out of the prompt thread, or return a cached result
    /// if the response takes more than 100 milliseconds
    pub fn get(&mut self, make_prompt: &Fn() -> PromptBuffer) -> PromptBufferResult<String> {
        info!("Checking lifesigns");
        if !self.check_is_alive() {
            info!("Thread is not alive. Reviving it");
            self.revive(make_prompt)?;
        }

        info!("Asking for a new prompt");
        self.send.send(());

        info!("Creating timeout");
        let timeout = oneshot_timer(Duration::from_millis(50));

        loop {
            let ref recv = self.recv;
            #[allow(unused_mut)]
            {
                chan_select! {
                    default =>{},
                    recv.recv() -> text => {
                        info!("Got text");
                        if let Some(t) = text {
                            self.cached = t;
                            return Ok(self.cached.clone());
                        }
                    },
                    timeout.recv() => {
                        info!("Got timeout");
                        return Ok(self.cached.clone());
                    }
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}
