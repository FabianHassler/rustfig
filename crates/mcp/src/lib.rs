//! RustFig MCP server (layer L5): the Model Context Protocol over stdio.
//!
//! Exposes the engine's commands as MCP tools so assistants can draw,
//! inspect, convert and render figures. Every message is untrusted input.

use rustfig_engine::{cmd, Engine};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// MCP protocol version implemented.
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// Longest message accepted, in bytes.
pub const MAX_MESSAGE: usize = 64 << 20;

/// An MCP session over one engine.
pub struct Server {
    pub engine: Engine,
}

impl Default for Server {
    fn default() -> Self {
        Server { engine: Engine::new() }
    }
}

fn tools() -> Value {
    json!([
        {
            "name": "rustfig_execute",
            "description": "Run a RustFig command on the current drawing. Use rustfig_commands to list commands and their parameters. Coordinates are Fig units (1200 per inch, y grows downwards).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Command id, e.g. shape.rectangle"},
                    "params": {"type": "object", "description": "Command parameters"}
                },
                "required": ["command"]
            }
        },
        {
            "name": "rustfig_commands",
            "description": "List RustFig commands with their parameters. Optional filter by substring.",
            "inputSchema": {"type": "object", "properties": {"filter": {"type": "string"}}}
        },
        {
            "name": "rustfig_render",
            "description": "Render the current drawing as a PNG image to look at it.",
            "inputSchema": {"type": "object", "properties": {"dpi": {"type": "number", "description": "Resolution, default 100"}}}
        },
        {
            "name": "rustfig_document",
            "description": "Return the current drawing as Fig 3.2 text (or RustFig JSON with format=rustfig).",
            "inputSchema": {"type": "object", "properties": {"format": {"type": "string", "enum": ["fig", "rustfig"]}}}
        }
    ])
}

fn text_result(v: &Value) -> Value {
    json!({"content": [{"type": "text", "text": v.to_string()}]})
}

fn error_result(msg: &str) -> Value {
    json!({"content": [{"type": "text", "text": msg}], "isError": true})
}

impl Server {
    pub fn new() -> Server {
        Server::default()
    }

    fn call_tool(&mut self, name: &str, args: &Value) -> Value {
        match name {
            "rustfig_execute" => {
                let Some(command) = args.get("command").and_then(Value::as_str) else {
                    return error_result("missing \"command\"");
                };
                let params = args.get("params").cloned().unwrap_or_else(|| json!({}));
                match self.engine.execute(command, &params) {
                    Ok(v) => text_result(&v),
                    Err(e) => error_result(&format!("{} ({})", e.message, e.code)),
                }
            }
            "rustfig_commands" => {
                let filter = args.get("filter").and_then(Value::as_str).unwrap_or("").to_lowercase();
                let list: Vec<Value> = cmd::all()
                    .iter()
                    .filter(|c| filter.is_empty() || c.id.contains(&filter) || c.label.to_lowercase().contains(&filter))
                    .map(|c| json!({"id": c.id, "label": c.label, "params": c.params}))
                    .collect();
                text_result(&json!({"commands": list, "style_params": cmd::STYLE_PARAMS}))
            }
            "rustfig_render" => {
                let dpi = args
                    .get("dpi")
                    .and_then(Value::as_f64)
                    .unwrap_or(100.0)
                    .clamp(10.0, 600.0);
                match self.engine.execute(
                    "file.export_data",
                    &json!({"format": "png", "dpi": dpi, "margin": 60, "background": "#ffffff", "options": {"smoothing": 4}}),
                ) {
                    Ok(v) => match v.get("base64").and_then(Value::as_str) {
                        Some(b) => json!({"content": [{"type": "image", "data": b, "mimeType": "image/png"}]}),
                        None => error_result("render produced no image"),
                    },
                    Err(e) => error_result(&e.message),
                }
            }
            "rustfig_document" => {
                let format = args.get("format").and_then(Value::as_str).unwrap_or("fig");
                match self.engine.execute("file.save_data", &json!({ "format": format })) {
                    Ok(v) => {
                        json!({"content": [{"type": "text", "text": v.get("data").and_then(Value::as_str).unwrap_or("")}]})
                    }
                    Err(e) => error_result(&e.message),
                }
            }
            other => error_result(&format!("unknown tool {other:?}")),
        }
    }

    /// Handle one JSON-RPC message; returns the reply (None for notifications).
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str);
        let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
        let Some(method) = method else {
            return id.map(|id| rpc_error(id, -32600, "invalid request"));
        };
        let result = match method {
            "initialize" => json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "rustfig", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "RustFig draws vector figures (xfig-compatible). Use rustfig_commands to discover commands, rustfig_execute to draw and edit, rustfig_render to look at the result."
            }),
            "ping" => json!({}),
            "tools/list" => json!({"tools": tools()}),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                let r = rustfig_engine::guard::catch(|| self.call_tool(name, &args));
                r.unwrap_or_else(|m| error_result(&format!("internal error: {m}")))
            }
            m if m.starts_with("notifications/") => return None,
            _ => return id.map(|id| rpc_error(id, -32601, "method not found")),
        };
        id.map(|id| json!({"jsonrpc": "2.0", "id": id, "result": result}))
    }

    /// Handle one line of input.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        if line.trim().is_empty() {
            return None;
        }
        if line.len() > MAX_MESSAGE {
            return Some(rpc_error(Value::Null, -32600, "message too large").to_string());
        }
        match serde_json::from_str::<Value>(line) {
            Ok(Value::Array(batch)) => {
                let replies: Vec<Value> = batch.iter().filter_map(|m| self.handle(m)).collect();
                if replies.is_empty() {
                    None
                } else {
                    Some(Value::Array(replies).to_string())
                }
            }
            Ok(v) => self.handle(&v).map(|r| r.to_string()),
            Err(e) => Some(rpc_error(Value::Null, -32700, &format!("parse error: {e}")).to_string()),
        }
    }

    /// Serve over stdin/stdout until end of input.
    pub fn serve_stdio(&mut self) -> std::io::Result<()> {
        let stdin = std::io::stdin();
        let mut out = std::io::stdout();
        for line in stdin.lock().lines() {
            let line = line?;
            if let Some(reply) = self.handle_line(&line) {
                out.write_all(reply.as_bytes())?;
                out.write_all(b"\n")?;
                out.flush()?;
            }
        }
        Ok(())
    }
}

fn rpc_error(id: Value, code: i64, msg: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": msg}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_list_call() {
        let mut s = Server::new();
        let r = s
            .handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}))
            .unwrap();
        assert_eq!(r["result"]["serverInfo"]["name"], "rustfig");
        let r = s
            .handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}))
            .unwrap();
        assert_eq!(r["result"]["tools"].as_array().unwrap().len(), 4);
        let r = s
            .handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "rustfig_execute", "arguments": {"command": "shape.circle", "params": {"cx": 600, "cy": 600, "r": 300}}}}))
            .unwrap();
        assert!(r["result"].get("isError").is_none(), "{r}");
        let r = s
            .handle(&json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "rustfig_render", "arguments": {}}}))
            .unwrap();
        assert_eq!(r["result"]["content"][0]["type"], "image");
        assert!(s
            .handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .is_none());
    }

    #[test]
    fn bad_input() {
        let mut s = Server::new();
        assert!(s.handle_line("{nope").unwrap().contains("-32700"));
        assert!(s.handle_line("").is_none());
        assert!(s.handle_line("[]").is_none());
        assert!(s
            .handle_line("{\"id\":1,\"method\":\"nope\"}")
            .unwrap()
            .contains("-32601"));
    }
}
