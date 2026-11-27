//! Minimal MCP (JSON-RPC 2.0 over stdio) server for the SSH tool surface.
//!
//! Launch the desktop binary with `--mcp` to expose SSH/SFTP operations to an
//! MCP client. The GUI is not started in this mode.

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::event::event_bus;
use crate::tools::{SshSessionManager, mcp_tools};

pub fn run() -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("failed to start MCP runtime: {error}"))?;
    runtime.block_on(serve())
}

async fn serve() -> Result<(), String> {
    let manager = SshSessionManager::new(event_bus());
    let mut reader = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .await
            .map_err(|error| format!("failed to read stdin: {error}"))?;
        if read == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(trimmed) {
            Ok(request) => handle(&manager, request).await,
            Err(error) => Some(json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": { "code": -32700, "message": format!("parse error: {error}") }
            })),
        };
        if let Some(response) = response {
            let text = serde_json::to_string(&response)
                .map_err(|error| format!("failed to serialize response: {error}"))?;
            stdout
                .write_all(text.as_bytes())
                .await
                .map_err(|error| format!("failed to write stdout: {error}"))?;
            stdout
                .write_all(b"\n")
                .await
                .map_err(|error| format!("failed to write stdout: {error}"))?;
            stdout
                .flush()
                .await
                .map_err(|error| format!("failed to flush stdout: {error}"))?;
        }
    }

    manager.stop_all().await;
    Ok(())
}

async fn handle(manager: &SshSessionManager, request: Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    // JSON-RPC notifications have no id and must not receive a response.
    let id = id?;

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2024-11-05"),
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": {
                "name": "modbus-tool-ssh",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": mcp_tools() })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            match manager.call_mcp(name, arguments).await {
                Ok(value) => Ok(value),
                Err(error) => Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("{}: {}", error.code, error.detail)
                    }],
                    "isError": true
                })),
            }
        }
        other => Err((-32601, format!("method not found: {other}"))),
    };

    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    })
}
