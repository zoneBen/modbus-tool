//! MCP-compatible SSH tool surface.
//!
//! The Tauri commands `tool_ssh_mcp_tools` and `tool_ssh_mcp_call` expose this
//! contract to a future MCP transport (stdio/SSE/HTTP). Keeping the contract
//! here means an external MCP server can forward tools to the desktop app
//! without duplicating SSH/SFTP behavior.

use serde::Serialize;
use serde_json::{Value, json};

use super::model::{
    SshConnectInput, SshDownloadInput, SshInput, SshTransferInput, ToolErrorDto, ToolResult,
};
use super::ssh_session::SshSessionManager;

pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "ssh_list",
            "List live SSH sessions managed by Modbus Tool.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "ssh_connect",
            "Open an interactive SSH shell. Returns a sessionId used by ssh_send and ssh_read.",
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "host": { "type": "string" },
                    "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
                    "username": { "type": "string" },
                    "auth": { "type": "string", "enum": ["password", "key"] },
                    "password": { "type": "string" },
                    "keyPath": { "type": "string" },
                    "keyPassphrase": { "type": "string" },
                    "cols": { "type": "integer", "minimum": 20, "maximum": 500 },
                    "rows": { "type": "integer", "minimum": 4, "maximum": 300 },
                    "followCwd": { "type": "boolean" },
                    "timeoutMs": { "type": "integer", "minimum": 2000, "maximum": 60000 }
                },
                "required": ["host", "username"]
            }),
        ),
        tool(
            "ssh_disconnect",
            "Close an interactive SSH session.",
            json!({
                "type": "object",
                "properties": { "sessionId": { "type": "string" } },
                "required": ["sessionId"]
            }),
        ),
        tool(
            "ssh_send",
            "Send terminal input (text or control characters) to an SSH session.",
            json!({
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string" },
                    "data": { "type": "string" }
                },
                "required": ["sessionId", "data"]
            }),
        ),
        tool(
            "ssh_read",
            "Read new terminal output since a cursor. Omit cursor for the retained buffer start.",
            json!({
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string" },
                    "cursor": { "type": "integer", "minimum": 0 }
                },
                "required": ["sessionId"]
            }),
        ),
        tool(
            "ssh_execute",
            "Run a one-shot SSH command without an interactive session.",
            json!({
                "type": "object",
                "properties": {
                    "host": { "type": "string" },
                    "port": { "type": "integer" },
                    "username": { "type": "string" },
                    "auth": { "type": "string", "enum": ["password", "key"] },
                    "password": { "type": "string" },
                    "keyPath": { "type": "string" },
                    "keyPassphrase": { "type": "string" },
                    "command": { "type": "string" },
                    "timeoutMs": { "type": "integer" }
                },
                "required": ["host", "username", "command"]
            }),
        ),
        tool(
            "ssh_sftp_list",
            "List a remote SFTP directory.",
            json!({
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["sessionId"]
            }),
        ),
        tool(
            "ssh_upload",
            "Upload a local file into a remote directory over SFTP.",
            json!({
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string" },
                    "localPath": { "type": "string" },
                    "remoteDir": { "type": "string" }
                },
                "required": ["sessionId", "localPath", "remoteDir"]
            }),
        ),
        tool(
            "ssh_download",
            "Download a remote file to a local path over SFTP.",
            json!({
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string" },
                    "remotePath": { "type": "string" },
                    "localPath": { "type": "string" }
                },
                "required": ["sessionId", "remotePath", "localPath"]
            }),
        ),
        tool(
            "ssh_mkdir",
            "Create a remote directory over SFTP.",
            json!({
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["sessionId", "path"]
            }),
        ),
        tool(
            "ssh_remove",
            "Remove a remote file or directory over SFTP.",
            json!({
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string" },
                    "path": { "type": "string" },
                    "isDir": { "type": "boolean" }
                },
                "required": ["sessionId", "path"]
            }),
        ),
        tool(
            "ssh_rename",
            "Rename a remote path over SFTP.",
            json!({
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string" },
                    "from": { "type": "string" },
                    "to": { "type": "string" }
                },
                "required": ["sessionId", "from", "to"]
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

pub async fn call(manager: &SshSessionManager, name: &str, arguments: Value) -> ToolResult<Value> {
    let args = if arguments.is_null() {
        json!({})
    } else {
        arguments
    };
    match name {
        "ssh_list" => Ok(content(&manager.list())),
        "ssh_connect" => {
            let input: SshConnectInput = serde_json::from_value(args)
                .map_err(|error| ToolErrorDto::new("mcpInvalidArguments", error.to_string()))?;
            Ok(content(&manager.connect(input).await?))
        }
        "ssh_disconnect" => {
            let session_id = string_arg(&args, "sessionId")?;
            Ok(content(&json!({
                "disconnected": manager.disconnect(&session_id).await
            })))
        }
        "ssh_send" => {
            let session_id = string_arg(&args, "sessionId")?;
            let data = string_arg(&args, "data")?;
            manager.write_input(&session_id, &data).await?;
            Ok(content(&json!({ "sent": data.len() })))
        }
        "ssh_read" => {
            let session_id = string_arg(&args, "sessionId")?;
            let cursor = args.get("cursor").and_then(Value::as_u64);
            let mut result = manager.read_output(&session_id, cursor)?;
            result.data = crate::tools::ssh_session::strip_ansi(&result.data);
            Ok(content(&result))
        }
        "ssh_execute" => {
            let input: SshInput = serde_json::from_value(args)
                .map_err(|error| ToolErrorDto::new("mcpInvalidArguments", error.to_string()))?;
            Ok(content(&super::ssh::execute(input).await?))
        }
        "ssh_sftp_list" => {
            let session_id = string_arg(&args, "sessionId")?;
            let path = args.get("path").and_then(Value::as_str).map(str::to_owned);
            Ok(content(&manager.sftp_list(&session_id, path).await?))
        }
        "ssh_upload" => {
            let input: SshTransferInput = serde_json::from_value(args)
                .map_err(|error| ToolErrorDto::new("mcpInvalidArguments", error.to_string()))?;
            manager.upload(input).await?;
            Ok(content(&json!({ "uploaded": true })))
        }
        "ssh_download" => {
            let input: SshDownloadInput = serde_json::from_value(args)
                .map_err(|error| ToolErrorDto::new("mcpInvalidArguments", error.to_string()))?;
            manager.download(input).await?;
            Ok(content(&json!({ "downloaded": true })))
        }
        "ssh_mkdir" => {
            let session_id = string_arg(&args, "sessionId")?;
            let path = string_arg(&args, "path")?;
            manager.sftp_mkdir(&session_id, &path).await?;
            Ok(content(&json!({ "created": path })))
        }
        "ssh_remove" => {
            let session_id = string_arg(&args, "sessionId")?;
            let path = string_arg(&args, "path")?;
            let is_dir = args.get("isDir").and_then(Value::as_bool).unwrap_or(false);
            manager.sftp_remove(&session_id, &path, is_dir).await?;
            Ok(content(&json!({ "removed": path })))
        }
        "ssh_rename" => {
            let session_id = string_arg(&args, "sessionId")?;
            let from = string_arg(&args, "from")?;
            let to = string_arg(&args, "to")?;
            manager.sftp_rename(&session_id, &from, &to).await?;
            Ok(content(&json!({ "renamed": from, "to": to })))
        }
        _ => Err(ToolErrorDto::new(
            "mcpUnknownTool",
            format!("Unknown SSH MCP tool: {name}"),
        )),
    }
}

fn string_arg(args: &Value, key: &str) -> ToolResult<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ToolErrorDto::new("mcpInvalidArguments", format!("Missing {key}")))
}

fn content<T: Serialize>(value: &T) -> Value {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_owned());
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_ssh_tools_with_schemas() {
        let tools = tools();
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .collect();
        assert!(names.contains(&"ssh_connect"));
        assert!(names.contains(&"ssh_send"));
        assert!(names.contains(&"ssh_read"));
        assert!(names.contains(&"ssh_upload"));
        assert!(names.contains(&"ssh_download"));
        for tool in tools {
            assert!(tool.get("inputSchema").is_some());
        }
    }
}
