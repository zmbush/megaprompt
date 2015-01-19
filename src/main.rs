#![deny(
    unused_allocation,
    unused_attributes,
    unused_casts,
    unused_import_braces,
    unused_imports,
    unused_must_use,
    unused_mut,
    unused_parens,
    unused_results,
    unused_unsafe,
    unused_variables,
)]

extern crate term;
extern crate git2;
extern crate prompt_buffer;

use prompt_buffer::thread::PromptThread;
use prompt_buffer::buffer::PromptBuffer;

use std::collections::HashMap;
use std::io::{
    self,
    Acceptor,
    Command,
    File,
    fs,
    IoError,
    Listener,
    process,
    stdio,
    timer,
    IoErrorKind
};
use std::io::fs::PathExtensions;
use std::io::net::pipe::{
    UnixListener,
    UnixStream,
};
use std::os;
use std::time::Duration;

mod git;
mod due_date;

fn get_prompt() -> PromptBuffer {
    let mut buf = PromptBuffer::new();
    buf.add_plugin(Box::new(due_date::DueDatePlugin::new()));
    buf.add_plugin(Box::new(git::GitPlugin::new()));

    buf
}

fn exe_changed() -> u64 {
    match os::self_exe_name() {
        Some(exe_path) => {
            match exe_path.stat() {
                Ok(s) => s.modified,
                _ => 0u64
            }
        },
        None => 0u64
    }
}

fn current_pid(pid_path: &Path) -> Result<i32, IoError> {
    let mut file = try!(File::open(pid_path));
    let contents = try!(file.read_to_string());

    match contents.parse() {
        Some(value) => Ok(value),
        None => Err(io::standard_error(IoErrorKind::InvalidInput))
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

fn main() {
    let stdout_path = Path::new("/var/log/megaprompt/current.out");
    let stderr_path = Path::new("/var/log/megaprompt/current.err");
    let pid_path = Path::new("/var/log/megaprompt/current.pid");
    let socket_path = Path::new("/tmp/megaprompt-socket");

    let args = os::args();
    if args.len() > 1 { // Daemon process
        let _ = stdio::set_stdout(Box::new(File::create(&stdout_path)));
        let _ = stdio::set_stderr(Box::new(File::create(&stderr_path)));

        let last_modified = exe_changed();
        let mut threads: HashMap<Path, PromptThread> = HashMap::new();

        if socket_path.exists() {
            fs::unlink(&socket_path).unwrap();
        }

        let stream = match UnixListener::bind(&socket_path) {
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
    } else {
        let current_pid = current_pid(&pid_path).ok().unwrap_or(-1);

        match process::Process::kill(current_pid, 0) {
            Err(_) => {
                // We need to start up the daemon again
                let child = Command::new(args.get(0).as_slice()[0])
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

        let mut stream = match UnixStream::connect(&socket_path) {
            Err(_) => {
                println!("Can't connect");
                get_prompt().print();
                return;
            },
            Ok(stream) => stream
        };

        write!(&mut stream, "{}", os::make_absolute(&Path::new(".")).unwrap().display()).unwrap();
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
}
