//! A small local MCP server (stdio, JSON-RPC 2.0, one message per line) so AI
//! assistants on this Mac can search cards, read Today and add captures.
//! Run by the `noma-mcp` binary; it opens NOMA's database directly. Captures
//! are written as pending rows that the running app picks up (within ~30 s).

use std::io::{BufRead, Write};
use std::path::PathBuf;

use chrono::Utc;
use rusqlite::Connection;
use serde_json::{Value, json};

use crate::commands::ai::{CaptureContext, placeholder};
use crate::db::{self, filing, models::Item, models::ItemFilter};
use crate::state::local_tz;

const PROTOCOL: &str = "2025-06-18";
const SEARCH_LIMIT: u32 = 20;

/// `$NOMA_DB`, else NOMA's database in Application Support.
pub fn db_path() -> PathBuf {
    if let Some(p) = std::env::var_os("NOMA_DB") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join("Library/Application Support/com.noma.app/noma.db")
}

fn tools() -> Value {
    json!([
        {
            "name": "search_cards",
            "description": "Full-text search over the user's NOMA cards (tasks, reminders, notes, ideas, events).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Words to look for." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
                },
                "required": ["query"]
            }
        },
        {
            "name": "list_today",
            "description": "The cards on the user's Today list in NOMA (due or reminding today, plus overdue).",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "add_capture",
            "description": "Send free text to NOMA as a capture. NOMA's on-device AI turns it into cards (tasks, reminders, notes…) and files them.",
            "inputSchema": {
                "type": "object",
                "properties": { "text": { "type": "string", "description": "What to capture, in natural language." } },
                "required": ["text"]
            }
        }
    ])
}

fn card(conn: &Connection, item: &Item) -> Value {
    let place = filing::place_of(conn, &item.id)
        .map(|(_, name)| name)
        .unwrap_or_else(|_| filing::INBOX_NAME.to_string());
    json!({
        "id": item.id,
        "kind": item.kind,
        "title": item.title,
        "body": item.body,
        "status": item.status,
        "priority": item.priority,
        "dueAt": item.due_at,
        "remindAt": item.remind_at,
        "place": place,
        "tags": item.tags,
    })
}

fn cards(conn: &Connection, items: &[Item]) -> String {
    let all: Vec<Value> = items.iter().map(|i| card(conn, i)).collect();
    serde_json::to_string_pretty(&all).unwrap_or_default()
}

/// Runs one tool; `Err` becomes an `isError` result the model can read.
pub fn call(conn: &Connection, name: &str, args: &Value) -> Result<String, String> {
    let text = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("missing \"{key}\""))
    };
    match name {
        "search_cards" => {
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .map_or(SEARCH_LIMIT, |n| n.clamp(1, 50) as u32);
            let items =
                db::items::search_items(conn, text("query")?, limit).map_err(|e| e.to_string())?;
            Ok(cards(conn, &items))
        }
        "list_today" => {
            let filter = ItemFilter::Today {
                now: Utc::now(),
                tz: local_tz(),
            };
            let items = db::items::list_items(conn, &filter, false).map_err(|e| e.to_string())?;
            Ok(cards(conn, &items))
        }
        "add_capture" => {
            let raw = text("text")?;
            let now = Utc::now();
            let item =
                db::items::create_item(conn, placeholder(raw, &CaptureContext::default()), now)
                    .map_err(|e| e.to_string())?;
            db::captures::create_capture(conn, "text", raw, Some(&item.id), now)
                .map_err(|e| e.to_string())?;
            Ok(format!(
                "Captured. NOMA will turn it into cards shortly (placeholder card {}).",
                item.id
            ))
        }
        other => Err(format!("unknown tool {other}")),
    }
}

fn reply(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Answers one JSON-RPC message; notifications get no answer.
pub fn handle(conn: Option<&Connection>, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned()?;
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    Some(match method {
        "initialize" => {
            let version = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL);
            reply(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "noma", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": "NOMA is the user's personal second mind: cards for tasks, reminders, notes and ideas. Use add_capture to hand NOMA anything the user wants to remember."
                }),
            )
        }
        "ping" => reply(id, json!({})),
        "tools/list" => reply(id, json!({ "tools": tools() })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let outcome = match conn {
                Some(conn) => call(conn, name, &args),
                None => Err(format!(
                    "NOMA's database was not found at {}. Open NOMA once, or set NOMA_DB.",
                    db_path().display()
                )),
            };
            let (text, is_error) = match outcome {
                Ok(t) => (t, false),
                Err(e) => (e, true),
            };
            reply(
                id,
                json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }),
            )
        }
        _ => error(id, -32601, "method not found"),
    })
}

/// Serves stdin/stdout until stdin closes. Returns the process exit code.
pub fn run() -> i32 {
    let path = db_path();
    let conn = path
        .is_file()
        .then(|| db::open(&path))
        .and_then(|r| r.map_err(|e| eprintln!("noma-mcp: {e}")).ok());
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let answer = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(conn.as_ref(), &msg),
            Err(_) => Some(error(Value::Null, -32700, "parse error")),
        };
        if let Some(answer) = answer
            && writeln!(stdout, "{answer}")
                .and_then(|_| stdout.flush())
                .is_err()
        {
            break;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpc(conn: &Connection, method: &str, params: Value) -> Value {
        handle(
            Some(conn),
            &json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }),
        )
        .unwrap()
    }

    #[test]
    fn speaks_mcp_and_runs_tools() {
        let conn = db::open_in_memory().unwrap();
        let init = rpc(
            &conn,
            "initialize",
            json!({ "protocolVersion": "2025-03-26" }),
        );
        assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
        assert!(
            handle(
                Some(&conn),
                &json!({ "method": "notifications/initialized" })
            )
            .is_none()
        );
        let list = rpc(&conn, "tools/list", json!({}));
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 3);

        let add = rpc(
            &conn,
            "tools/call",
            json!({ "name": "add_capture", "arguments": { "text": "buy oat milk" } }),
        );
        assert_eq!(add["result"]["isError"], false);
        assert!(db::captures::next_pending(&conn).unwrap().is_some());

        let found = rpc(
            &conn,
            "tools/call",
            json!({ "name": "search_cards", "arguments": { "query": "oat" } }),
        );
        assert!(
            found["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("buy oat milk")
        );

        let bad = rpc(
            &conn,
            "tools/call",
            json!({ "name": "search_cards", "arguments": {} }),
        );
        assert_eq!(bad["result"]["isError"], true);
        assert_eq!(rpc(&conn, "nope", json!({}))["error"]["code"], -32601);
    }
}
