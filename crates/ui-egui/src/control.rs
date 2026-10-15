//! JSON-lines control channel on 127.0.0.1 (native only).
//!
//! Each line is a request `{"id": .., "method": .., "params": {..}}`; each
//! reply is one line `{"id": .., "result": ..}` or `{"id": .., "error":
//! {"code": .., "message": ..}}`. Requests are untrusted input.
//!
//! Methods: `engine.execute` {command, params}, `engine.commands`,
//! `ui.run` {command, params} (UI and engine commands), `ui.screenshot`
//! {path}, `ui.state`, `app.ping`.

use crate::App;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Longest request line accepted, in bytes.
pub const MAX_LINE: usize = 64 << 20;

pub struct Request {
    pub id: Value,
    pub method: String,
    pub params: Value,
    pub reply: Sender<String>,
}

pub struct Server {
    rx: Mutex<Receiver<Request>>,
}

impl Server {
    pub fn start(port: u16, ctx: egui::Context) -> Result<Server, String> {
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| e.to_string())?;
        let (tx, rx) = channel::<Request>();
        let tx = Arc::new(Mutex::new(tx));
        std::thread::Builder::new()
            .name("rustfig-control".into())
            .spawn(move || {
                for stream in listener.incoming().flatten() {
                    let tx = tx.clone();
                    let ctx = ctx.clone();
                    let _ = std::thread::Builder::new()
                        .name("rustfig-control-conn".into())
                        .spawn(move || serve(stream, tx, ctx)); // a failed spawn drops just this connection
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Server { rx: Mutex::new(rx) })
    }

    /// Pending requests (non-blocking).
    pub fn drain(&self) -> Vec<Request> {
        match self.rx.lock() {
            Ok(rx) => rx.try_iter().collect(),
            Err(_) => Vec::new(),
        }
    }
}

fn serve(stream: TcpStream, tx: Arc<Mutex<Sender<Request>>>, ctx: egui::Context) {
    let Ok(read) = stream.try_clone() else { return };
    let mut writer = stream;
    let mut reader = BufReader::new(read);
    loop {
        let mut line = String::new();
        let mut limited = (&mut reader).take(MAX_LINE as u64);
        match limited.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Err(e) => json!({"id": null, "error": {"code": "parse", "message": e.to_string()}}).to_string(),
            Ok(v) => {
                let id = v.get("id").cloned().unwrap_or(Value::Null);
                let method = v.get("method").and_then(Value::as_str).unwrap_or("").to_string();
                let params = v.get("params").cloned().unwrap_or_else(|| json!({}));
                let (rtx, rrx) = channel();
                let sent = tx
                    .lock()
                    .map(|t| {
                        t.send(Request {
                            id: id.clone(),
                            method,
                            params,
                            reply: rtx,
                        })
                        .is_ok()
                    })
                    .unwrap_or(false);
                if !sent {
                    return;
                }
                ctx.request_repaint();
                match rrx.recv_timeout(Duration::from_secs(60)) {
                    Ok(r) => r,
                    Err(_) => {
                        json!({"id": id, "error": {"code": "timeout", "message": "the app did not answer"}}).to_string()
                    }
                }
            }
        };
        if writer.write_all(reply.as_bytes()).is_err() || writer.write_all(b"\n").is_err() {
            return;
        }
    }
}

use std::io::Read as _;

/// Handle one request on the UI thread.
pub fn handle(app: &mut App, method: &str, p: &Value) -> Result<Value, (&'static str, String)> {
    let command = || p.get("command").and_then(Value::as_str).unwrap_or("").to_string();
    let params = || p.get("params").cloned().unwrap_or_else(|| json!({}));
    match method {
        "app.ping" => Ok(json!("pong")),
        "engine.execute" => app
            .engine
            .execute(&command(), &params())
            .map_err(|e| (e.code, e.message)),
        "ui.run" => app.try_run(&command(), &params()).map_err(|m| ("failed", m)),
        "engine.commands" => {
            let mut v: Vec<Value> = rustfig_engine::cmd::all()
                .iter()
                .map(|c| json!({"id": c.id, "label": c.label, "menu": c.menu, "shortcut": c.shortcut, "params": c.params, "mutates": c.mutates}))
                .collect();
            v.extend(crate::menus::UI_COMMANDS.iter().map(|c| {
                json!({"id": c.id, "label": c.label, "menu": "", "shortcut": c.shortcut, "params": c.params, "ui": true})
            }));
            Ok(Value::Array(v))
        }
        "ui.state" => Ok(json!({
            "tool": app.engine.tool().id(),
            "selection": app.engine.selection().iter().map(|i| i.0).collect::<Vec<_>>(),
            "zoom": app.engine.view.zoom,
            "origin": [app.engine.view.origin.x, app.engine.view.origin.y],
            "canvas": [app.engine.view.size.0, app.engine.view.size.1],
            "modified": app.engine.is_modified(),
            "path": app.engine.path(),
            "message": app.message.as_ref().map(|m| m.text.clone()),
            "render_ms": app.canvas.render_ms,
            "frame_ms": app.frame_times.back().copied().unwrap_or(0.0),
            "frame_ms_max_recent": app.frame_times.iter().copied().fold(0.0, f64::max),
        })),
        _ => Err(("unknown_method", format!("unknown method {method:?}"))),
    }
}
