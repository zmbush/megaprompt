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

#![feature(
    path_ext,
    std_misc,
    path_relative_from,
    duration,
    plugin
)]

#![plugin(clippy)]

extern crate term;
extern crate git2;
extern crate unix_socket;
extern crate prompt_buffer;

use prompt_buffer::thread::PromptThread;
use prompt_buffer::buffer::PromptBuffer;

use std::collections::HashMap;
use std::fs;
use std::io::{Write, Read};

use unix_socket::{
    UnixListener,
    UnixStream,
};
use std::env;
use std::time::Duration;
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
    run(match args.len() {
        2 => RunMode::Daemon,
        1 => RunMode::Main,
        _ => panic!("Number of arguments must be 0 or 1")
    });
}

fn do_daemon(socket_path: &Path) {
    // let stdout_path = Path::new("/var/log/megaprompt/current.out");
    // let stderr_path = Path::new("/var/log/megaprompt/current.err");

    // let _ = stdio::set_stdout(Box::new(File::create(&stdout_path)));
    // let _ = stdio::set_stderr(Box::new(File::create(&stderr_path)));

    let last_modified = exe_changed();
    let mut threads: HashMap<PathBuf, PromptThread> = HashMap::new();

    if socket_path.exists() {
        fs::remove_file(socket_path).ok().expect("Unable to remove file");
    }

    let stream = match UnixListener::bind(socket_path) {
        Err(_) => panic!("Failed to bind to socket"),
        Ok(stream) => stream
    };

    println!("BIND");

    for connection in stream.incoming() {
        let c = &mut connection.unwrap();

        let mut output = String::new();
        let _  = sock_try!(c.read_to_string(&mut output));
        let output = PathBuf::from(&output);
        println!("Preparing to respond to for {}", output.display());

        let keys: Vec<PathBuf> = threads.keys().map(|x| { x.clone() }).collect();
        for path in keys.iter() {
            if !threads.get_mut(path).unwrap().is_alive() {
                println!("- Remove thread {}", path.display());
                let _ = threads.remove(path);
            }
        }

        if !threads.contains_key(&output) {
            println!("+ Add thread {}", output.display());
            let _ = threads.insert(output.clone(), PromptThread::new(output.clone(), &get_prompt));
        }

        for path in threads.keys() {
            println!("* Active thread {}", path.display());
        }

        let mut thr = threads.get_mut(&output).unwrap();

        sock_try!(write!(c, "{}", thr.get(&get_prompt)));

        println!("");

        if last_modified != exe_changed() {
            sock_try!(write!(c, "♻  "));
            return;
        }
    }
}

fn oneshot_timer(dur: Duration) -> Receiver<()> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let time = dur.secs() * 1000 + dur.extra_nanos() as u64 / 1000;
        thread::sleep_ms(time as u32);

        tx.send(()).unwrap();
    });

    rx
}

fn read_with_timeout(mut stream: UnixStream, dur: Duration) -> Result<String,String> {
    let (tx, rx) = mpsc::channel();

    let _ = thread::spawn(move || {
        let mut ret = String::new();
        stream.read_to_string(&mut ret).unwrap();
        tx.send(ret).unwrap();
    });

    let timeout = oneshot_timer(dur);

    select! {
        resp = rx.recv() => Ok(resp.unwrap()),
        _ = timeout.recv() => Err("Timeout".to_owned())
    }
}

fn do_main(socket_path: &Path) {
    let is_running = match Command::new("megapromptd").arg("status").output() {
        Ok(output) => String::from_utf8_lossy(output.stdout.as_ref()).contains("is running"),
        Err(_) => false
    };

    if !is_running {
        let _ = Command::new("megapromptd").arg("start").output();
        thread::sleep_ms(10);
    }

    let mut stream = match UnixStream::connect(socket_path) {
        Err(_) => {
            println!("Can't connect");
            get_prompt().print();
            return;
        },
        Ok(stream) => stream
    };

    write!(&mut stream, "{}", env::current_dir().unwrap().display()).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();

    match read_with_timeout(stream, Duration::from_millis(200)) {
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
