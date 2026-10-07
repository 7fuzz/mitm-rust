//! Single-instance guard. The first instance binds a fixed localhost port and answers PING / QUIT /
//! FOCUS on it; binding is atomic, so two instances launched together can't both win.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;

const INSTANCE_ADDR: &str = "127.0.0.1:48217";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OtherInstance {
    pub pid: u32,
    pub started_at_ms: i64,
}

/// The running instance this one is waiting on, until the user picks what to do
#[derive(Default)]
pub struct InstanceState(pub Mutex<Option<OtherInstance>>);

pub enum Claim {
    Primary,
    Conflict(OtherInstance),
    /// The port is held by something that isn't us; run without the guard
    Unguarded,
}

/// What the primary instance does when another launch asks it to. Without `on_quit` it exits the
/// process; without `on_focus` FOCUS is ignored.
#[derive(Default)]
pub struct Handlers {
    pub on_quit: Option<Box<dyn Fn() + Send + 'static>>,
    pub on_focus: Option<Box<dyn Fn() + Send + 'static>>,
}

pub fn try_claim(handlers: Handlers) -> Claim {
    match TcpListener::bind(INSTANCE_ADDR) {
        Ok(listener) => {
            serve(listener, handlers);
            Claim::Primary
        }
        Err(_) => match ping() {
            Some(other) => Claim::Conflict(other),
            None => Claim::Unguarded,
        },
    }
}

pub fn ping() -> Option<OtherInstance> {
    let reply = send("PING")?;
    let mut parts = reply.strip_prefix("PONG ")?.split_whitespace();
    Some(OtherInstance {
        pid: parts.next()?.parse().ok()?,
        started_at_ms: parts.next()?.parse().ok()?,
    })
}

pub fn request_quit() -> bool {
    send("QUIT").as_deref() == Some("BYE")
}

pub fn request_focus() -> bool {
    send("FOCUS").as_deref() == Some("OK")
}

fn send(command: &str) -> Option<String> {
    let addr = INSTANCE_ADDR.parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    writeln!(stream, "{}", command).ok()?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).ok()?;
    Some(reply.trim().to_string())
}

fn serve(listener: TcpListener, handlers: Handlers) {
    let pid = std::process::id();
    let started_at_ms = chrono::Local::now().timestamp_millis();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut line = String::new();
            let Ok(mut writer) = stream.try_clone() else { continue };
            if BufReader::new(stream).read_line(&mut line).is_err() {
                continue;
            }
            match line.trim() {
                "PING" => {
                    let _ = writeln!(writer, "PONG {} {}", pid, started_at_ms);
                }
                "QUIT" => {
                    let _ = writeln!(writer, "BYE");
                    let _ = writer.flush();
                    if let Some(ref cb) = handlers.on_quit {
                        cb();
                    } else {
                        std::process::exit(0);
                    }
                }
                "FOCUS" => {
                    let _ = writeln!(writer, "OK");
                    if let Some(ref cb) = handlers.on_focus {
                        cb();
                    }
                }
                _ => {}
            }
        }
    });
}
