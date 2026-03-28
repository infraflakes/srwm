use std::io::Read;
use std::process::{Command, Stdio};

pub struct Session {
    child: Option<std::process::Child>,
    address: String,
}

impl Session {
    pub fn start() -> Option<Session> {
        if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok() {
            return None;
        }

        let dbus_path = which::which("dbus-daemon").ok()?;

        let mut child = Command::new(&dbus_path)
            .args(&["--session", "--print-address", "--nofork", "--nopidfile"])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .ok()?;

        let mut address = String::new();
        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if let Some(ref mut stdout) = child.stdout {
                let mut buf = [0u8; 256];
                if let Ok(n) = stdout.read(&mut buf) {
                    address.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if address.contains('\n') {
                        break;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        if address.is_empty() {
            let _ = child.kill();
            return None;
        }

        let address = address.trim().to_string();
        unsafe {
            std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &address);
        }
        eprintln!("srwm: D-Bus session bus started: {}", address);

        Some(Session {
            child: Some(child),
            address,
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
            eprintln!("srwm: D-Bus session bus stopped");
        }
    }
}
