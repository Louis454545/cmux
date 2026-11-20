// cmux.protocol/2 client: newline-JSON over the daemon's unix socket.
// Each connection owns a reader thread; calls are matched by numeric id.
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn runtime_base() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("TMPDIR").map(PathBuf::from))
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn uid() -> u32 {
    unsafe { libc::getuid() }
}

pub fn socket_path_for_session(session: &str) -> PathBuf {
    if let Ok(p) = std::env::var("CMUX_TUI_SOCKET") {
        return PathBuf::from(p);
    }
    let base = runtime_base().join(format!("cmux-tui-{}", uid()));
    let candidate = base.join(format!("{}.sock", session));
    if candidate.as_os_str().len() < 100 {
        return candidate;
    }
    // Long names hash into a parallel dir (mirrors the server fallback).
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    session.hash(&mut h);
    runtime_base()
        .join(format!("cmux-tui-hashed-{}", uid()))
        .join(format!("{:x}.sock", h.finish()))
}

pub struct Connection {
    write: Mutex<UnixStream>,
    pending: Mutex<HashMap<u64, std_mpsc::Sender<Value>>>,
    next_id: AtomicU64,
}

impl Connection {
    /// Opens a socket and starts the reader thread; parsed messages go to
    /// `events` (id-less `{event:...}` frames plus unsolicited frames).
    pub fn open(
        path: &PathBuf,
        events: smol::channel::Sender<Value>,
    ) -> std::io::Result<Arc<Connection>> {
        let stream = UnixStream::connect(path)?;
        let reader = stream.try_clone()?;
        let conn = Arc::new(Connection {
            write: Mutex::new(stream),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        });
        let conn2 = conn.clone();
        std::thread::spawn(move || {
            let mut lines = BufReader::new(reader);
            let mut line = String::new();
            loop {
                line.clear();
                match lines.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
                    continue;
                };
                if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                    if msg.get("ok").is_some() {
                        let tx = conn2.pending.lock().unwrap().remove(&id);
                        if let Some(tx) = tx {
                            let _ = tx.send(msg);
                        }
                        continue;
                    }
                }
                let _ = events.send_blocking(msg);
            }
            // Socket closed: fail all pending calls so callers don't hang.
            for (_, tx) in conn2.pending.lock().unwrap().drain() {
                let _ = tx.send(json!({"id": 0, "ok": false, "error": "socket closed"}));
            }
        });
        Ok(conn)
    }

    /// Blocking call with a 10s timeout. Runs on caller's thread — keep it
    /// off the UI thread.
    pub fn call(&self, cmd: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut req = json!({"id": id, "cmd": cmd});
        if let Some(obj) = params.as_object() {
            for (k, v) in obj {
                req[k] = v.clone();
            }
        }
        let (tx, rx) = std_mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let write = self.write.lock().unwrap().write_all(
            (serde_json::to_string(&req).unwrap() + "\n").as_bytes(),
        );
        if let Err(e) = write {
            self.pending.lock().unwrap().remove(&id);
            return Err(e.to_string());
        }
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(resp) => {
                if resp.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                    Ok(resp.get("data").cloned().unwrap_or(json!({})))
                } else {
                    Err(resp
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("command failed")
                        .to_string())
                }
            }
            Err(e) => {
                self.pending.lock().unwrap().remove(&id);
                Err(e.to_string())
            }
        }
    }
}

/// Session driver: one control connection plus one connection per
/// attached surface; all events land on the same `events` channel.
pub struct Mux {
    pub control: Arc<Connection>,
    pub session: String,
    pub identity: Value,
    attaches: Mutex<HashMap<u64, Arc<Connection>>>,
    events: smol::channel::Sender<Value>,
}

impl Mux {
    /// Connects to a running daemon; on failure spawns
    /// `cmux-tui server start --session <session>` and retries.
    pub fn connect_or_start(
        session: &str,
        events: smol::channel::Sender<Value>,
    ) -> Result<Arc<Mux>, String> {
        let path = socket_path_for_session(session);
        let mut spawned = false;
        for _ in 0..30 {
            if let Ok(control) = Connection::open(&path, events.clone()) {
                let identity = control.call("identify", json!({}))?;
                control.call(
                    "set-client-info",
                    json!({"name": "cmux-gui", "capabilities": ["shared-sizing-v1"]}),
                )?;
                control.call("subscribe", json!({"tree_events": "deltas"}))?;
                return Ok(Arc::new(Mux {
                    control,
                    session: session.to_string(),
                    identity,
                    attaches: Mutex::new(HashMap::new()),
                    events,
                }));
            }
            if !spawned {
                spawned = true;
                let bin = std::env::var("CMUX_BIN").unwrap_or_else(|_| "cmux".to_string());
                let _ = std::process::Command::new(bin)
                    .args(["server", "start", "--session", session])
                    .spawn();
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        Err("cmux server did not come up".into())
    }

    /// Attach a render-mode connection for one surface. Re-attaching the
    /// same surface retires the previous stream.
    pub fn attach_render(&self, surface: u64) {
        self.detach(surface);
        let path = socket_path_for_session(&self.session);
        if let Ok(conn) = Connection::open(&path, self.events.clone()) {
            conn.call("attach-surface", json!({"surface": surface, "mode": "render"}))
                .ok();
            self.attaches.lock().unwrap().insert(surface, conn);
        }
    }

    pub fn detach(&self, surface: u64) {
        self.attaches.lock().unwrap().remove(&surface);
    }
}
