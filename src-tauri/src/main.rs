// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|argument| argument == "--mcp") {
        if let Err(error) = modbus_tool_lib::run_mcp_stdio() {
            eprintln!("MCP server failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    modbus_tool_lib::run()
}
