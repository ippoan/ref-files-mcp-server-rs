//! ref-files-mcp-server-rs — binary entry (Phase 0 stub).
//!
//! Phase 0 only ships the shared types + ts-rs generator. The MCP server
//! itself (rmcp `#[tool_router]` modules under `src/tools/`, streamable
//! HTTP transport on 127.0.0.1, auth-worker WS relay client) lands in
//! Phase 1, mirroring `github-mcp-server-rs/src/mcp_server.rs`.
//!
//! Keeping this stub buildable lets `rust-ci.yml` (`cargo build --release`
//! + `--help` smoke test) pass on the very first PR, so the CI plumbing is
//! exercised before any real tool code lands.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Reference file storage MCP server (Phase 0 — ts-rs source of truth only; tools land in Phase 1)"
)]
struct Cli {
    /// Override ref-files-worker base URL (default: prod / staging selected by `--env`).
    #[arg(long, env = "REF_FILES_WORKER_BASE")]
    worker_base: Option<String>,

    /// Bind address for the local Streamable HTTP MCP server. auth-worker's
    /// relay attaches to this socket (matches github-mcp-server-rs).
    #[arg(long, default_value = "127.0.0.1:0")]
    bind: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    eprintln!(
        "ref-files-mcp-server-rs: Phase 0 stub. worker_base={:?} bind={}",
        cli.worker_base, cli.bind
    );
    eprintln!("Tools (repo_init / folder_* / file_*) land in Phase 1.");
    Ok(())
}
