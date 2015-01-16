#![deny(unused_must_use)]
extern crate term;
extern crate git2;
#[allow(unstable)] extern crate libc;

use prompt_buffer::PromptBuffer;

use std::io::{
    Acceptor,
    Command,
    File,
    fs,
    Listener,
    process,
    stdio,
    timer,
    IoError,
    Timer
};
use std::time::Duration;
use std::io::fs::PathExtensions;
use std::io::net::pipe::{
    UnixStream,
    UnixListener,
};
use std::os;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::Thread;

mod prompt_buffer;
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
        None => Err(IoError::from_errno(libc::consts::os::posix88::ENOMSG as usize, true))
    }
}

fn main() {
    let stdout_path = Path::new("/var/log/megaprompt/current.out");
    let stderr_path = Path::new("/var/log/megaprompt/current.err");
    let pid_path = Path::new("/var/log/megaprompt/current.pid");
    let socket_path = Path::new("/tmp/megaprompt-socket");

    let args = os::args();
    if args.len() > 1 { // Daemon process
        stdio::set_stdout(Box::new(File::create(&stdout_path)));
        stdio::set_stderr(Box::new(File::create(&stderr_path)));

        let last_modified = exe_changed();
        let mut cached_response = get_prompt().to_string_ext(true);
        let mut index = 0i32;

        if socket_path.exists() {
            fs::unlink(&socket_path).unwrap();
        }

        let stream = match UnixListener::bind(&socket_path) {
            Err(_) => panic!("Failed to bind to socket"),
            Ok(stream) => stream
        };

        let (snd_path, recv_path) = mpsc::channel();
        let (snd_prompt, recv_prompt) = mpsc::channel();

        Thread::spawn(move || {
            let mut prompt = get_prompt();

            for (ix, path) in recv_path.iter() {
                prompt.set_path(path);

                snd_prompt.send((ix, prompt.to_string())).unwrap();
            }
        });

        for mut connection in stream.listen().incoming() {
            macro_rules! sock_try {
                ($x:expr) => {
                    match $x {
                        Ok(v) => v,
                        Err(_) => continue
                    }
                };
            }
            let c = &mut connection;
            let mut timer = Timer::new().unwrap();
            // We need to respond within 100 ms, so set a 90ms timer
            let respond_by = timer.oneshot(Duration::milliseconds(90));

            let output = Path::new(sock_try!(c.read_to_string()));
            index += 1;
            snd_path.send((index, output)).unwrap();

            loop {
                let resp = recv_prompt.try_recv();

                if resp.is_ok() {
                    // We got a good response
                    let (ix, text) = resp.unwrap();

                    cached_response = text;
                    if ix == index {
                        sock_try!(write!(c, "{}", &cached_response));
                        break;
                    }
                }

                // We ran out of time!
                if respond_by.try_recv().is_ok() {
                    sock_try!(write!(c, "{}~ ", &cached_response));
                    break;
                }
            }

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
        stream.set_read_timeout(Some(100));
        match stream.read_to_string() {
            Ok(s) => println!("{}", s),
            Err(_) => {
                println!("Response too slow");
                get_prompt().print_fast();
                return
            }
        }
    }
}
