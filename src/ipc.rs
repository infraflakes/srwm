use std::io::Read;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static RUNNING: AtomicBool = AtomicBool::new(true);

pub fn socket_path() -> PathBuf {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime).join("srwm.sock")
    } else {
        PathBuf::from("/tmp/srwm.sock")
    }
}

pub fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

pub fn set_running(value: bool) {
    RUNNING.store(value, Ordering::SeqCst);
}

pub fn start_ipc_server() {
    let path = socket_path();

    std::thread::spawn(move || {
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("srwm: failed to bind socket {}: {}", path.display(), e);
                return;
            }
        };

        if let Err(e) = listener.set_nonblocking(true) {
            eprintln!("srwm: failed to set socket nonblocking: {}", e);
            return;
        }

        while is_running() {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    std::thread::spawn(move || {
                        handle_connection(&mut stream);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("srwm: socket accept error: {}", e);
                }
            }
        }

        let _ = std::fs::remove_file(&path);
    });
}

fn handle_connection(stream: &mut impl Read) {
    let mut buf = [0u8; 256];
    let mut command = String::new();

    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                command.push_str(&String::from_utf8_lossy(&buf[..n]));
                if command.contains('\n') {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => {
                eprintln!("srwm: IPC read error: {}", e);
                return;
            }
        }
    }

    let cmd = command.trim();
    match cmd {
        "shutdown" => {
            eprintln!("srwm: IPC received shutdown");
            unsafe {
                crate::ffi::srwm_quit();
            }
        }
        "restart" => {
            eprintln!("srwm: IPC received restart");
            unsafe {
                crate::ffi::srwm_request_restart();
            }
        }
        "refresh" => {
            eprintln!("srwm: IPC received refresh");
            unsafe {
                crate::ffi::srwm_grabkeys();
            }
        }
        _ => {
            eprintln!("srwm: unknown IPC command: {}", cmd);
        }
    }
}

pub fn send_command(command: &str) -> Result<(), String> {
    let path = socket_path();

    use std::io::Write;
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(&path)
        .map_err(|e| format!("failed to connect to srwm: {} (is it running?)", e))?;

    stream
        .write_all(command.as_bytes())
        .map_err(|e| format!("failed to send: {}", e))?;
    stream
        .flush()
        .map_err(|e| format!("failed to flush: {}", e))?;

    Ok(())
}
