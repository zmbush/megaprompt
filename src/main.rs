// Copyright 2017 Zachary Bush.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![deny(unused_allocation, unused_attributes, unused_features, unused_import_braces,
        unused_parens, unused_must_use, stable_features, bad_style, unused)]


#[macro_use]
extern crate chan;
extern crate clap;
extern crate git2;
extern crate log4rs;
#[macro_use]
extern crate log;
extern crate num;
extern crate prompt_buffer;
extern crate term;
extern crate time;
extern crate unix_socket;

use prompt_buffer::{PromptBuffer, PromptThread, ShellType};

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};

use time::Duration;
use log4rs::config;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;

use unix_socket::{UnixListener, UnixStream};
use std::env;
use std::path::{Path, PathBuf};
use std::net::Shutdown;
use std::thread;
use chan::Receiver;
use std::os::unix::fs::MetadataExt;
use std::process::Command;
use clap::{App, Arg, ArgGroup};

mod git;
mod due_date;

fn get_prompt(shell: ShellType) -> PromptBuffer {
    let mut buf = PromptBuffer::new(shell);
    buf.add_plugin(due_date::DueDatePlugin::new());
    buf.add_plugin(git::GitPlugin::new());

    buf
}

fn exe_changed() -> i64 {
    match env::current_exe() {
        Ok(exe_path) => match fs::metadata(exe_path) {
            Ok(m) => m.mtime(),
            Err(_) => 0i64,
        },
        Err(_) => 0i64,
    }
}

macro_rules! sock_try {
    ($x:expr) => {
        match $x {
            Ok(v) => v,
            Err(_) => continue
        }
    };
}

#[allow(dead_code)]
enum RunMode {
    Daemon,
    Main,
    Test,
}

#[allow(dead_code)]
fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .after_help(env!("CARGO_PKG_HOMEPAGE"))
        .arg(
            Arg::with_name("daemon")
                .short("d")
                .long("daemon")
                .help("Run the daemon"),
        )
        .arg(
            Arg::with_name("bash")
                .short("b")
                .long("bash")
                .help("Get output for bash"),
        )
        .arg(
            Arg::with_name("zsh")
                .short("z")
                .long("zsh")
                .help("Get output for zsh"),
        )
        .group(
            ArgGroup::with_name("mode")
                .args(&["daemon", "bash", "zsh"])
                .required(true),
        )
        .get_matches();
    let daemon = matches.is_present("daemon");
    let shell = if matches.is_present("bash") {
        ShellType::Bash
    } else {
        ShellType::Zsh
    };
    run(
        if daemon {
            RunMode::Daemon
        } else {
            RunMode::Main
        },
        shell,
    )
}

fn do_daemon(socket_path: &Path) {
    let main_log = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({f:>30.30}: {m}{n})}")))
        .build("/var/log/megaprompt/current.out")
        .expect("Unable to create file appender");

    let config = config::Config::builder()
        .appender(config::Appender::builder().build("main", Box::new(main_log)))
        .build(
            config::Root::builder()
                .appender("main")
                .build(log::LogLevelFilter::Trace),
        )
        .expect("Unable to create logger config");

    log4rs::init_config(config).expect("Unable to init logger");

    let last_modified = exe_changed();
    let mut threads: HashMap<(PathBuf, ShellType), PromptThread> = HashMap::new();

    if socket_path.exists() {
        fs::remove_file(socket_path).expect("Unable to remove socket file");
    }

    let stream = match UnixListener::bind(socket_path) {
        Err(_) => unreachable!("unable to bind to socket"),
        Ok(stream) => stream,
    };

    info!("BIND");

    for connection in stream.incoming() {
        let mut c = match connection {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut output = String::new();
        let _ = sock_try!(c.read_to_string(&mut output));
        let (output, shell) = if output.starts_with("!2 ") {
            let parts = output.split(" ").collect::<Vec<_>>();
            let output = PathBuf::from(&parts[1]);
            let shell = match parts[2] {
                "Bash" => ShellType::Bash,
                "Zsh" => ShellType::Zsh,
                _ => ShellType::Bash,
            };
            (output, shell)
        } else {
            (PathBuf::from(&output), ShellType::Bash)
        };
        info!(
            "Preparing to respond to for {} [{:?}]",
            output.display(),
            shell
        );

        let keys: Vec<(PathBuf, ShellType)> = threads.keys().cloned().collect();
        for entry in &keys {
            if !threads
                .get_mut(entry)
                .expect("thread not there!")
                .check_is_alive()
            {
                info!("- Remove thread {}", entry.0.display());
                let _ = threads.remove(entry);
            }
        }

        if !threads.contains_key(&(output.clone(), shell)) {
            info!("+ Add thread {}", output.display());
            let t = sock_try!(PromptThread::new(output.clone(), &|| get_prompt(shell)));
            let _ = threads.insert((output.clone(), shell), t);
        }

        for &(ref path, ref shell) in threads.keys() {
            info!("* Active thread {} [{:?}]", path.display(), shell);
        }

        let thr = threads
            .get_mut(&(output, shell))
            .expect("Thread not present");

        info!("Getting response from thread");
        sock_try!(write!(c, "{}", sock_try!(thr.get(&|| get_prompt(shell)))));

        info!("");

        if last_modified != exe_changed() {
            warn!("Found newer version of myself. Quitting.");
            sock_try!(write!(c, "♻  "));
            return;
        }
    }
}

fn oneshot_timer(dur: Duration) -> Receiver<()> {
    let (tx, rx) = chan::async();

    thread::spawn(move || {
        thread::sleep(::std::time::Duration::from_millis(
            dur.num_milliseconds() as u64,
        ));
        let _ = tx.send(());
    });

    rx
}

fn read_with_timeout(mut stream: UnixStream, dur: Duration) -> Result<String, String> {
    let (tx, rx) = chan::sync(0);

    let _ = thread::spawn(move || {
        let mut ret = String::new();
        stream
            .read_to_string(&mut ret)
            .expect("Unable to read from string");
        let _ = tx.send(ret);
    });

    let timeout = oneshot_timer(dur);

    #[allow(unused_mut)]
    {
        chan_select! {
            rx.recv() ->resp => return Ok(resp.expect("There is no response!")),
            timeout.recv() => return Err("Timeout".to_owned())
        }
    }
}

fn do_main(socket_path: &Path, shell: ShellType) {
    let _ = Command::new("megapromptd").arg("start").output();

    let mut stream = match UnixStream::connect(socket_path) {
        Err(_) => {
            println!("Can't connect");
            get_prompt(shell).print();
            return;
        }
        Ok(stream) => stream,
    };

    write!(
        &mut stream,
        "!2 {} {:?}",
        env::current_dir()
            .expect("There is no current dir")
            .display(),
        shell
    ).expect("Unable to print current directory");
    stream
        .shutdown(Shutdown::Write)
        .expect("Cannot shutdown stream");

    match read_with_timeout(stream, Duration::milliseconds(100)) {
        Ok(s) => println!("{}", s),
        Err(_) => {
            println!("Response too slow");
            get_prompt(shell).print_fast();
            return;
        }
    }
}

fn run(mode: RunMode, shell: ShellType) {
    let socket_path = Path::new("/tmp/megaprompt-socket");

    match mode {
        RunMode::Daemon => do_daemon(socket_path),
        RunMode::Main => do_main(socket_path, shell),
        RunMode::Test => {}
    }
}

#[test]
fn test_main_does_not_error() {
    run(RunMode::Test);
}
