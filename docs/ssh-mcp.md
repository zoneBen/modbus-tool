# SSH MCP / AI interface

The SSH console exposes an MCP-compatible tool surface so an AI agent can drive
interactive sessions without duplicating terminal or SFTP logic.

## MCP server over stdio

Launch the desktop binary with `--mcp` to run a JSON-RPC 2.0 MCP server over
stdin/stdout instead of the GUI:

```powershell
modbus-tool.exe --mcp
```

Example MCP client configuration:

```json
{
  "mcpServers": {
    "modbus-tool-ssh": {
      "command": "C:\\Program Files\\Modbus Tool\\modbus-tool.exe",
      "args": ["--mcp"]
    }
  }
}
```

Supported requests: `initialize`, `notifications/initialized`, `ping`,
`tools/list`, `tools/call`. The server manages its own SSH sessions and exits
when stdin closes.

## In-process bridge

The GUI also exposes the same contract through Tauri commands:

- `tool_ssh_mcp_tools` → `tools/list` payload (`name`, `description`, `inputSchema`)
- `tool_ssh_mcp_call` → `tools/call` payload (`content[]`, `isError`)

`tool_ssh_mcp_call` accepts:

```json
{
  "name": "ssh_send",
  "arguments": { "sessionId": "...", "data": "uname -a\r" }
}
```

The SSH manager keeps a rolling 512 KiB output buffer per session. `ssh_read`
takes an optional byte cursor and returns the new text plus the next cursor, which
makes polling safe for agents. ANSI escape sequences are stripped for MCP reads;
the raw stream is still delivered to xterm.

## Tools

| Tool | Purpose |
|------|---------|
| `ssh_list` | List live sessions and status |
| `ssh_connect` | Open an interactive shell (password or key auth) |
| `ssh_disconnect` | Close a session |
| `ssh_send` | Send terminal input/control characters |
| `ssh_read` | Read output since a cursor |
| `ssh_execute` | Run a one-shot command without a session |
| `ssh_sftp_list` | List a remote directory |
| `ssh_upload` | Upload a local file to a remote directory |
| `ssh_download` | Download a remote file to a local path |
| `ssh_mkdir` | Create a remote directory |
| `ssh_remove` | Remove a remote file or directory |
| `ssh_rename` | Rename a remote path |

## Security

The SSH console accepts host keys without `known_hosts` verification for
diagnostics. Before exposing the MCP server to an autonomous agent, add a trust
policy (known hosts, allowlists or explicit approval) and protect credentials in
the MCP client configuration. The stdio server is local to the process that
starts it and does not open a network port.
