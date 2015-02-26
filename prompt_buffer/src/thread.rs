//! Used to allow a thread per path. This way the cached value can be
//! different based on which path it is running from. For paths with
//! slow prompt.to_string outputs, this is particularily useful.
//!
//! Thred will run for 10 minutes after the last request, to avoid
//! leaking too many threads.
use std::old_io::{timer, Timer};
use std::time::Duration;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use std::path::PathBuf;

use buffer::{PromptBuffer, PluginSpeed};

/// Stores information about prompt threads
pub struct PromptThread {
    send: Sender<()>,
    recv: Receiver<String>,
    death: Receiver<()>,
    path: PathBuf,
    cached: String,
    alive: bool,
}

impl PromptThread {
    /// Creates a new prompt thread for a given path
    pub fn new(path: PathBuf, make_prompt: &Fn() -> PromptBuffer) -> PromptThread {
        let (tx_notify, rx_notify) = mpsc::channel();
        let (tx_prompt, rx_prompt) = mpsc::channel();
        let (tx_death, rx_death) = mpsc::channel();

        let p = path.clone();
        let mut prompt = make_prompt();
        let cached = prompt.to_string_ext(PluginSpeed::Fast);
        thread::Builder::new().name(format!("{}", path.display())).spawn(move || {
            prompt.set_path(p);
            let mut timer = Timer::new().unwrap();

            loop {
                let timeout = timer.oneshot(Duration::minutes(10));

                select! {
                    _ = rx_notify.recv() => {
                        tx_prompt.send(prompt.to_string()).unwrap();
                    },
                    _ = timeout.recv() => {
                        // Assume someone is listening for my death
                        // Otherwise it doesn't matter
                        tx_death.send(()).unwrap();
                        break;
                    }
                }

                // Drain notify channel
                while rx_notify.try_recv().is_ok() {}
            }
        }).unwrap();

        PromptThread {
            send: tx_notify,
            recv: rx_prompt,
            death: rx_death,
            path: path,
            cached: cached,
            alive: true,
        }
    }

    /// Checks whether a prompt thread has announced it's death.
    pub fn is_alive(&mut self) -> bool {
        if self.death.try_recv().is_ok() {
            self.alive = false;
        }

        self.alive
    }

    fn revive(&mut self, make_prompt: &Fn() -> PromptBuffer) {
        *self = PromptThread::new(self.path.clone(), make_prompt)
    }

    /// Gets a result out of the prompt thread, or return a cached result
    /// if the response takes more than 100 milliseconds
    pub fn get(&mut self, make_prompt: &Fn() -> PromptBuffer) -> String {
        if !self.is_alive() {
            self.revive(make_prompt);
        }

        self.send.send(()).unwrap();

        let mut timer = Timer::new().unwrap();
        let timeout = timer.oneshot(Duration::milliseconds(100));

        loop {
            let resp = self.recv.try_recv();

            if resp.is_ok() {
                // We got a good response
                let mut text = resp.unwrap();

                loop {
                    match self.recv.try_recv() {
                        Ok(t) => text = t,
                        Err(_) => break
                    }
                }

                self.cached = text;
                return self.cached.clone();
            }

            // We ran out of time!
            if timeout.try_recv().is_ok() {
                return self.cached.clone();
            }

            timer::sleep(Duration::milliseconds(10));
        }
    }
}
