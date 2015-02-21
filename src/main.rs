#![deny(
    unused_allocation,
    unused_attributes,
    unused_import_braces,
    unused_imports,
    unused_must_use,
    unused_mut,
    unused_parens,
    unused_results,
    unused_unsafe,
    unused_variables,

    dead_code,
    deprecated
)]

#![feature(
    core,
    env,
    io,
    old_io,
    old_path,
    std_misc,
)]

extern crate term;
extern crate git2;
extern crate prompt_buffer;

use prompt_buffer::thread::PromptThread;
use prompt_buffer::buffer::PromptBuffer;

use std::collections::HashMap;
use std::old_io::{
    Acceptor,
    Command,
    File,
    fs,
    Listener,
    process,
    stdio,
    timer,
};
use std::old_io::fs::PathExtensions;
use std::old_io::net::pipe::{
    UnixListener,
    UnixStream,
};
use std::env;
use std::time::Duration;
use std::error::Error;

mod git;
mod due_date;

fn get_prompt() -> PromptBuffer {
    let mut buf = PromptBuffer::new();
    buf.add_plugin(due_date::DueDatePlugin::new());
    buf.add_plugin(git::GitPlugin::new());

    buf
}

fn exe_changed() -> u64 {
    match env::current_exe() {
        Ok(exe_path) => {
            match exe_path.stat() {
                Ok(s) => s.modified,
                _ => 0u64
            }
        },
        Err(_) => 0u64
    }
}

fn current_pid(pid_path: &Path) -> Result<i32, Box<Error>> {
    let mut file = try!(File::open(pid_path));
    let contents = try!(file.read_to_string());
    Ok(try!(contents.parse()))
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
    let stdout_path = Path::new("/var/log/megaprompt/current.out");
    let stderr_path = Path::new("/var/log/megaprompt/current.err");

    let _ = stdio::set_stdout(Box::new(File::create(&stdout_path)));
    let _ = stdio::set_stderr(Box::new(File::create(&stderr_path)));

    let last_modified = exe_changed();
    let mut threads: HashMap<Path, PromptThread> = HashMap::new();

    if socket_path.exists() {
        fs::unlink(socket_path).unwrap();
    }

    let stream = match UnixListener::bind(socket_path) {
        Err(_) => panic!("Failed to bind to socket"),
        Ok(stream) => stream
    };

    for mut connection in stream.listen().incoming() {
        let c = &mut connection;

        let output = Path::new(sock_try!(c.read_to_string()));
        println!("Preparing to respond to for {}", output.display());

        let keys: Vec<Path> = threads.keys().map(|x| { x.clone() }).collect();
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

fn do_main(socket_path: &Path) {
    let pid_path = Path::new("/var/log/megaprompt/current.pid");

    let current_pid = current_pid(&pid_path).ok().unwrap_or(-1);

    match process::Process::kill(current_pid, 0) {
        Err(_) => {
            // We need to start up the daemon again
            let child = Command::new(env::args().next().unwrap().as_slice())
                .arg("daemon")
                .detached().spawn().unwrap();


            let mut f = match File::create(&pid_path) {
                Ok(f) => f,
                Err(_) => panic!("Unable to open pid file")
            };

            write!(&mut f, "{}", child.id()).unwrap();

            println!("Spawned child {}", child.id());

            child.forget();

            timer::sleep(Duration::milliseconds(10));
        }
        _ => {}
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
    stream.close_write().unwrap();
    stream.set_read_timeout(Some(200));
    match stream.read_to_string() {
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
