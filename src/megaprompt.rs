extern crate term;
extern crate git2;
extern crate libc;

use prompt_buffer::PromptBuffer;

use std::io::{
    Acceptor,
    Command,
    File,
    fs,
    Listener,
    process,
    stdio,
    timer
};
use std::time::Duration;
use std::io::fs::PathExtensions;
use std::io::net::pipe::{
    UnixStream,
    UnixListener,
};
use std::os;

mod prompt_buffer;
mod git;
mod due_date;

fn get_prompt() -> PromptBuffer<'static> {
    let mut buf = PromptBuffer::new();
    buf.add_plugin(box due_date::DueDatePlugin::new());
    buf.add_plugin(box git::GitPlugin::new());

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

fn main() {
    let stdout_path = Path::new("/var/log/megaprompt/current.out");
    let stderr_path = Path::new("/var/log/megaprompt/current.err");
    let pid_path = Path::new("/var/log/megaprompt/current.pid");
    let socket_path = Path::new("/tmp/megaprompt-socket");

    let args = os::args();
    if args.len() > 1 { // Daemon process
        stdio::set_stdout(box File::create(&stdout_path));
        stdio::set_stderr(box File::create(&stderr_path));

        let last_modified = exe_changed();

        let mut p = get_prompt();

        if socket_path.exists() {
            fs::unlink(&socket_path).unwrap();
        }

        let stream = match UnixListener::bind(&socket_path) {
            Err(_) => panic!("Failed to bind to socket"),
            Ok(stream) => stream
        };

        for mut connection in stream.listen().incoming() {
            let c = &mut connection;

            let output = c.read_to_string().unwrap();
            p.set_path(Path::new(output));

            write!(c, "{}", p.to_string()).unwrap();

            if last_modified != exe_changed() {
                write!(c, "♻  ").unwrap();
                return;
            }
        }
    } else {
        let current_pid = match File::open(&pid_path) {
            Ok(mut file) => match file.read_to_string() {
                Ok(line) => match line.parse() {
                    Some(value) => value,
                    _ => -1
                },
                _ => -1
            },
            _ => -1
        };

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
        println!("{}", stream.read_to_string().unwrap());
    }
}
