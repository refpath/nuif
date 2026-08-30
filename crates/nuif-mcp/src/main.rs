fn main() {
    if let Err(error) = nuif_mcp::run_stdio() {
        eprintln!("NUIF_MCP_SERVER_FAILED: {error}");
        std::process::exit(1);
    }
}
