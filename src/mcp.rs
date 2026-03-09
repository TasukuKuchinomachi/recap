use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::storage::Storage;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i64, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!({ "code": code, "message": message })),
        }
    }
}

pub fn run_mcp_server() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let storage = Storage::new();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(
                    Value::Null,
                    -32700,
                    &format!("Parse error: {}", e),
                );
                let out = serde_json::to_string(&resp)?;
                writeln!(stdout.lock(), "{}", out)?;
                stdout.lock().flush()?;
                continue;
            }
        };

        let id = req.id.clone().unwrap_or(Value::Null);

        let resp = match req.method.as_str() {
            "initialize" => JsonRpcResponse::success(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "recap",
                        "version": "0.1.0"
                    }
                }),
            ),
            "notifications/initialized" => continue,
            "tools/list" => JsonRpcResponse::success(
                id,
                json!({
                    "tools": [
                        {
                            "name": "recap_last",
                            "description": "直前のコマンド実行結果を取得",
                            "inputSchema": {
                                "type": "object",
                                "properties": {},
                                "required": []
                            }
                        },
                        {
                            "name": "recap_list",
                            "description": "実行履歴の一覧を取得",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "limit": {
                                        "type": "integer",
                                        "description": "取得件数 (デフォルト: 10)",
                                        "default": 10
                                    }
                                },
                                "required": []
                            }
                        },
                        {
                            "name": "recap_get",
                            "description": "指定IDの実行結果を取得",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "id": {
                                        "type": "integer",
                                        "description": "実行ID"
                                    }
                                },
                                "required": ["id"]
                            }
                        }
                    ]
                }),
            ),
            "tools/call" => handle_tool_call(&storage, id.clone(), &req.params),
            _ => JsonRpcResponse::error(id, -32601, "Method not found"),
        };

        let out = serde_json::to_string(&resp)?;
        writeln!(stdout.lock(), "{}", out)?;
        stdout.lock().flush()?;
    }

    Ok(())
}

fn handle_tool_call(storage: &Storage, id: Value, params: &Value) -> JsonRpcResponse {
    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "recap_last" => match storage.get_last() {
            Ok(Some(entry)) => {
                let output = storage.read_log(entry.id).unwrap_or_default();
                let result = json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&json!({
                            "id": entry.id,
                            "cmd": entry.cmd,
                            "exit": entry.exit,
                            "at": entry.at,
                            "output": output
                        })).unwrap()
                    }]
                });
                JsonRpcResponse::success(id, result)
            }
            Ok(None) => {
                let result = json!({
                    "content": [{
                        "type": "text",
                        "text": "No runs recorded yet."
                    }]
                });
                JsonRpcResponse::success(id, result)
            }
            Err(e) => JsonRpcResponse::error(id, -32000, &format!("Storage error: {}", e)),
        },
        "recap_list" => {
            let limit = arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            match storage.get_last_n(limit) {
                Ok(entries) => {
                    let runs: Vec<Value> = entries
                        .iter()
                        .rev()
                        .map(|e| {
                            json!({
                                "id": e.id,
                                "cmd": e.cmd,
                                "exit": e.exit,
                                "at": e.at
                            })
                        })
                        .collect();
                    let result = json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string(&json!({ "runs": runs })).unwrap()
                        }]
                    });
                    JsonRpcResponse::success(id, result)
                }
                Err(e) => JsonRpcResponse::error(id, -32000, &format!("Storage error: {}", e)),
            }
        }
        "recap_get" => {
            let run_id = match arguments.get("id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    return JsonRpcResponse::error(id, -32602, "Missing required parameter: id")
                }
            };
            match storage.get_by_id(run_id) {
                Ok(Some(entry)) => {
                    let output = storage.read_log(entry.id).unwrap_or_default();
                    let result = json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string(&json!({
                                "id": entry.id,
                                "cmd": entry.cmd,
                                "exit": entry.exit,
                                "at": entry.at,
                                "output": output
                            })).unwrap()
                        }]
                    });
                    JsonRpcResponse::success(id, result)
                }
                Ok(None) => {
                    let result = json!({
                        "content": [{
                            "type": "text",
                            "text": format!("No run found with id {}", run_id)
                        }]
                    });
                    JsonRpcResponse::success(id, result)
                }
                Err(e) => JsonRpcResponse::error(id, -32000, &format!("Storage error: {}", e)),
            }
        }
        _ => JsonRpcResponse::error(id, -32602, &format!("Unknown tool: {}", tool_name)),
    }
}
