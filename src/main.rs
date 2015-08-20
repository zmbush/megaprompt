#![deny(
    unused_allocation,
    unused_attributes,
    unused_features,
    unused_import_braces,
    unused_parens,
    unused_must_use,
    stable_features,

    bad_style,
    unused,

    clippy
)]

// Some buggy clippy lints
#![allow(
    non_ascii_literal
)]

#![feature(
    mpsc_select,
    path_ext,
    path_relative_from,
    plugin,
    result_expect
)]

#![plugin(clippy)]

extern crate term;
extern crate git2;
extern crate unix_socket;
extern crate prompt_buffer;
extern crate time;
#[macro_use] extern crate log;
extern crate log4rs;

use prompt_buffer::thread::PromptThread;
use prompt_buffer::buffer::PromptBuffer;

use std::collections::HashMap;
use std::fs;
use std::io::{Write, Read};

use time::Duration;
use log4rs::{config, appender};

use unix_socket::{
    UnixListener,
    UnixStream,
};
use std::env;
use std::fs::PathExt;
use std::path::{Path, PathBuf};
use std::net::Shutdown;
use std::thread;
use std::sync::mpsc::{self, Receiver};
use std::os::unix::fs::MetadataExt;
use std::process::Command;

mod git;
mod due_date;

fn get_prompt() -> PromptBuffer {
    let mut buf = PromptBuffer::new();
    buf.add_plugin(due_date::DueDatePlugin::new());
    buf.add_plugin(git::GitPlugin::new());

    buf
}

fn exe_changed() -> i64 {
    match env::current_exe() {
        Ok(exe_path) => {
            match fs::metadata(exe_path) {
                Ok(m) => m.mtime(),
                Err(_) => 0i64
            }
        },
        Err(_) => 0i64
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
    Test
}

#[allow(dead_code)]
fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{}ms $ ", Duration::span(|| {
        run(match args.len() {
            2 => RunMode::Daemon,
            1 => RunMode::Main,
            _ => panic!("Number of arguments must be 0 or 1")
        });
    }).num_milliseconds());
}

fn do_daemon(socket_path: &Path) {
    // let stdout_path = Path::new("/var/log/megaprompt/current.out");
    // let stderr_path = Path::new("/var/log/megaprompt/current.err");

    // let _ = stdio::set_stdout(Box::new(File::create(&stdout_path)));
    // let _ = stdio::set_stderr(Box::new(File::create(&stderr_path)));
    log4rs::init_config(
        config::Config::builder(
            config::Root::builder(log::LogLevelFilter::Trace)
            .appender("main".to_owned())
            .build())
        .appender(
            config::Appender::builder(
                "main".to_owned(), Box::new(appender::FileAppender::builder(
                    "/var/log/megaprompt/current.out")
                .pattern(log4rs::pattern::PatternLayout::new("%l\t%t\t- %m").expect("Bad format"))
                .build().expect("Unable to create file appender")))
            .build())
        .build().expect("Unable to create config")
    ).expect("Unable to init logger");
    // log4rs::init_file("~/.megaprompt.toml", Default::default()).expect("Couldn't start logger");

    let last_modified = exe_changed();
    let mut threads: HashMap<PathBuf, PromptThread> = HashMap::new();

    if socket_path.exists() {
        fs::remove_file(socket_path).expect("Unable to remove socket file");
    }

    let stream = match UnixListener::bind(socket_path) {
        Err(_) => panic!("Failed to bind to socket"),
        Ok(stream) => stream
    };

    info!("BIND");

    for connection in stream.incoming() {
        let mut c = match connection {
            Ok(c) => c,
            Err(_) => continue
        };

        let mut output = String::new();
        let _  = sock_try!(c.read_to_string(&mut output));
        let output = PathBuf::from(&output);
        info!("Preparing to respond to for {}", output.display());

        let keys: Vec<PathBuf> = threads.keys().map(|x| { x.clone() }).collect();
        for path in &keys {
            if !threads.get_mut(path).expect("thread not there!").is_alive() {
                info!("- Remove thread {}", path.display());
                let _ = threads.remove(path);
            }
        }

        if !threads.contains_key(&output) {
            info!("+ Add thread {}", output.display());
            let t = sock_try!(PromptThread::new(output.clone(), &get_prompt));
            let _ = threads.insert(output.clone(), t);
        }

        for path in threads.keys() {
            info!("* Active thread {}", path.display());
        }

        let mut thr = threads.get_mut(&output).expect("Thread not present");

        sock_try!(write!(c, "{}", sock_try!(thr.get(&get_prompt))));

        info!("");

        if last_modified != exe_changed() {
            warn!("Found newer version of myself. Quitting.");
            sock_try!(write!(c, "♻  "));
            return;
        }
    }
}

fn oneshot_timer(dur: Duration) -> Receiver<()> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        thread::sleep_ms(dur.num_milliseconds() as u32);
        let _ = tx.send(());
    });

    rx
}

fn read_with_timeout(mut stream: UnixStream, dur: Duration) -> Result<String,String> {
    let (tx, rx) = mpsc::channel();

    let _ = thread::spawn(move || {
        let mut ret = String::new();
        stream.read_to_string(&mut ret).expect("Unable to read from string");
        let _ = tx.send(ret);
    });

    let timeout = oneshot_timer(dur);

    select! {
        resp = rx.recv() => Ok(resp.expect("There is no response!")),
        _ = timeout.recv() => Err("Timeout".to_owned())
    }
}

fn do_main(socket_path: &Path) {
    let _ = Command::new("megapromptd").arg("start").output();

    let mut stream = match UnixStream::connect(socket_path) {
        Err(_) => {
            println!("Can't connect");
            get_prompt().print();
            return;
        },
        Ok(stream) => stream
    };

    write!(&mut stream, "{}",
           env::current_dir().expect("There is no current dir").display())
        .expect("Unable to print current directory");
    stream.shutdown(Shutdown::Write).expect("Cannot shutdown stream");

    match read_with_timeout(stream, Duration::milliseconds(100)) {
        Ok(s) => println!("{}", s),
        Err(_) => {
            println!("Response too slow");
            get_prompt().print_fast();
            return;
        }
    }
}

fn run(mode: RunMode) {
    let socket_path = Path::new("/tmp/megaprompt-socket");

    match mode {
        RunMode::Daemon => do_daemon(&socket_path),
        RunMode::Main => do_main(&socket_path),
        RunMode::Test => {}
    }
}

#[test]
fn test_main_does_not_error() {
    run(RunMode::Test);
}
