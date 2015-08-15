//! Used to allow a thread per path. This way the cached value can be
//! different based on which path it is running from. For paths with
//! slow prompt.to_string outputs, this is particularily useful.
//!
//! Thred will run for 10 minutes after the last request, to avoid
//! leaking too many threads.
use std::time::Duration;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use std::path::PathBuf;

use buffer::{PromptBuffer, PluginSpeed};
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
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        thread::sleep_ms(dur.as_secs() as u32);

        let _ = tx.send(());
    });

    rx
}

impl PromptThread {
    /// Creates a new prompt thread for a given path
    pub fn new(path: PathBuf, make_prompt: &Fn() -> PromptBuffer) -> PromptBufferResult<PromptThread> {
        let (tx_notify, rx_notify) = mpsc::channel();
        let (tx_prompt, rx_prompt) = mpsc::channel();
        let (tx_death, rx_death) = mpsc::channel();

        let p = path.clone();
        let mut prompt = make_prompt();
        let cached = prompt.to_string_ext(PluginSpeed::Fast);
        try!(thread::Builder::new().name(format!("{}", path.display())).spawn(move || {
            prompt.set_path(p);

            loop {
                let timeout = oneshot_timer(Duration::from_secs(10*60));

                select! {
                    _ = rx_notify.recv() => {
                        if let Err(_) = tx_prompt.send(prompt.to_string()) {
                            return;
                        }
                    },
                    _ = timeout.recv() => {
                        let _ = tx_death.send(());
                        break;
                    }
                }

                // Drain notify channel
                while let Ok(_) = rx_notify.try_recv() {}
            }
        }));

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
    pub fn is_alive(&mut self) -> bool {
        if self.death.try_recv().is_ok() {
            self.alive = false;
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
        if !self.is_alive() {
            try!(self.revive(make_prompt));
        }

        try!(self.send.send(()));

        let timeout = oneshot_timer(Duration::from_millis(100));

        loop {
            if let Ok(mut text) = self.recv.try_recv() {
                while let Ok(t) = self.recv.try_recv() {
                    text = t;
                }

                self.cached = text;
                return Ok(self.cached.clone());
            }

            // We ran out of time!
            if let Ok(_) = timeout.try_recv() {
                return Ok(self.cached.clone());
            }

            thread::sleep_ms(1);
        }
    }
}
